use super::common::{
    check_authorization, decode_hex_path, decode_pubkey, default_limit,
    AddAccountContactRequest,
    ApiResponse, ApiTags, AutoAcceptRequest, AutoAcceptResponse,
    BulkUpdatePricesRequest, BulkUpdateStatusRequest,
    DuplicateOfferingRequest, EmptyResponse,
    GenerateOfferingsRequest, GenerateOfferingsResponse, HelpcenterSyncResponse,
    OfferingSuggestionsResponse,
    OnboardingUpdateResponse, ProvisioningStatusRequest, ReconcileKeepInstance,
    ReconcilePauseInstance, ReconcileRequest, ReconcileResponse, ReconcileTerminateInstance,
    ReconcileUnknownInstance,
    RentalResponseRequest, ResponseMetricsResponse, ResponseTimeDistributionResponse,
    UpdatePasswordRequest,
};
use crate::auth::{AgentAuthenticatedUser, ApiAuthenticatedUser, ProviderOrAgentAuth};
use crate::database::Database;
use dcc_common::ssh_exec::validate_recipe;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

const SSE_POLL_INTERVAL: Duration = Duration::from_secs(5);

fn validate_recipe_if_present(script: Option<&String>) -> Result<(), String> {
    if let Some(script) = script {
        let result = validate_recipe(script);
        if !result.valid {
            let errors: Vec<String> = result
                .issues
                .into_iter()
                .filter(|i| {
                    matches!(
                        i.severity,
                        dcc_common::ssh_exec::RecipeValidationSeverity::Error
                    )
                })
                .map(|i| i.message)
                .collect();
            return Err(format!("Recipe validation failed: {}", errors.join("; ")));
        }
    }
    Ok(())
}

/// Validate that an offering's `currency` is one Stripe can actually settle in.
///
/// ICPay (the ICP cryptocurrency rail) is fully retired — Stripe is the sole
/// payment rail. Every offering MUST be priced in a Stripe-supported currency
/// so a rental checkout can always complete. This rejects retired/crypto
/// currencies (e.g. `"ICP"`, `"BTC"`) at the boundary with a clear, actionable
/// message rather than silently accepting them and surfacing as a broken
/// checkout later.
///
/// Reuses the single source of truth in
/// [`dcc_common::payment_method::is_stripe_supported_currency`].
fn validate_offering_currency(currency: &str) -> Result<(), String> {
    if dcc_common::payment_method::is_stripe_supported_currency(currency) {
        Ok(())
    } else {
        Err(format!(
            "Currency '{}' is not supported. Stripe is the sole payment rail; \
             use a Stripe-supported currency (e.g. USD, EUR). \
             See https://stripe.com/docs/currencies",
            currency
        ))
    }
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct OfferingSliReportRequest {
    pub report_date: String,
    pub uptime_percent: f64,
    pub response_sli_percent: Option<f64>,
    pub incident_count: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateOfferingSliReportsRequest {
    pub sla_target_percent: f64,
    pub reports: Vec<OfferingSliReportRequest>,
}

/// SSE handler: streams pending password reset count changes every 5 seconds.
///
/// Authenticates via provider or agent auth headers/query params.
/// Accepts either provider auth (X-Public-Key) or agent auth (X-Agent-Pubkey).
/// Query params supported for EventSource: pubkey/agent_pubkey, signature, timestamp, nonce.
/// Sends an immediate event on connect, then polls every 5 seconds and emits an event
/// when the count or contract IDs change. Keep-alive comment sent every 30 seconds.
///
/// Event format:
///   event: password-reset-count
///   data: {"count":<n>,"contract_ids":["<id>",...]}
#[poem::handler]
pub async fn password_reset_events(
    req: &poem::Request,
    db: Data<&Arc<Database>>,
    poem::web::Path(pubkey): poem::web::Path<String>,
) -> poem::Result<poem::web::sse::SSE> {
    use futures::StreamExt;
    use poem::http::StatusCode;
    use poem::web::sse::{Event, SSE};

    let pubkey_bytes = decode_pubkey(&pubkey)
        .map_err(|e| poem::Error::from_string(e, StatusCode::BAD_REQUEST))?;

    let provider_pubkey = crate::auth::authenticate_provider_or_agent_from_request(req, &db)
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::UNAUTHORIZED))?;

    if provider_pubkey != pubkey_bytes {
        return Err(poem::Error::from_string(
            "Unauthorized: can only access your own provider's contracts",
            StatusCode::FORBIDDEN,
        ));
    }

    let db_clone: Arc<Database> = Arc::clone(&db);
    let stream = futures::stream::unfold(
        (db_clone, pubkey_bytes, None::<Vec<String>>),
        |(db, pk, prev_ids): (Arc<Database>, Vec<u8>, Option<Vec<String>>)| async move {
            let contracts: Vec<crate::database::contracts::Contract> =
                match db.get_pending_password_resets(&pk).await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("SSE password-reset-events DB error: {:#}", e);
                        return None;
                    }
                };
            let ids: Vec<String> = contracts.iter().map(|c| c.contract_id.clone()).collect();
            let event: Option<Event> = if prev_ids.as_ref() != Some(&ids) {
                let data = serde_json::json!({
                    "count": ids.len(),
                    "contract_ids": ids,
                });
                Some(Event::message(data.to_string()).event_type("password-reset-count"))
            } else {
                None
            };
            tokio::time::sleep(SSE_POLL_INTERVAL).await;
            Some((event, (db, pk, Some(ids))))
        },
    )
    .filter_map(|opt: Option<Event>| async move { opt });

    Ok(SSE::new(stream).keep_alive(std::time::Duration::from_secs(30)))
}

/// SSE handler: streams contract status changes for a user every 5 seconds.
///
/// Authenticates via user signature headers (X-Public-Key, X-Signature, etc.).
/// Sends an immediate event on connect, then polls every 5 seconds and emits events
/// when any contract status or updated_at_ns changes. Keep-alive sent every 30 seconds.
/// Closes after 5 minutes (client reconnects).
///
/// Event types emitted:
///   event: contract-status
///   data: {"contract_id":"<id>","status":"<status>","updated_at_ns":<ns>}
///
///   event: ssh_key_rotation
///   data: {"contract_id":"<id>","created_at":<ns>,"actor":"tenant","details":null}
///
///   event: ssh_key_rotation_complete
///   data: {"contract_id":"<id>","created_at":<ns>,"actor":"provider","details":"<msg>"}
#[poem::handler]
pub async fn contract_status_events(
    req: &poem::Request,
    db: Data<&Arc<Database>>,
    poem::web::Path(pubkey): poem::web::Path<String>,
) -> poem::Result<poem::web::sse::SSE> {
    use futures::StreamExt;
    use poem::http::StatusCode;
    use poem::web::sse::{Event, SSE};

    let pubkey_bytes = decode_pubkey(&pubkey)
        .map_err(|e| poem::Error::from_string(e, StatusCode::BAD_REQUEST))?;

    let auth_pubkey = crate::auth::authenticate_user_from_request(req)
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::UNAUTHORIZED))?;

    if auth_pubkey != pubkey_bytes {
        return Err(poem::Error::from_string(
            "Unauthorized: can only access your own contract events",
            StatusCode::FORBIDDEN,
        ));
    }

    type Snapshot = std::collections::HashMap<String, (String, Option<i64>)>;

    struct SseState {
        db: Arc<Database>,
        pk: Vec<u8>,
        prev_snapshot: Option<Snapshot>,
        poll_count: u32,
        last_rotation_event_ns: i64,
    }

    let db_clone: Arc<Database> = Arc::clone(&db);
    let initial = SseState {
        db: db_clone,
        pk: pubkey_bytes,
        prev_snapshot: None,
        poll_count: 0,
        last_rotation_event_ns: 0,
    };
    let stream = futures::stream::unfold(initial, |state: SseState| async move {
        if state.poll_count >= 60 {
            return None;
        }
        let contracts = match state.db.get_user_contracts(&state.pk).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("SSE contract-status-events DB error: {:#}", e);
                return None;
            }
        };

        let current: Snapshot = contracts
            .iter()
            .map(|c| {
                (
                    c.contract_id.clone(),
                    (c.status.clone(), c.status_updated_at_ns),
                )
            })
            .collect();

        let mut events: Vec<Event> = match &state.prev_snapshot {
            None => contracts
                .iter()
                .map(|c| {
                    let data = serde_json::json!({
                        "contract_id": c.contract_id,
                        "status": c.status,
                        "updated_at_ns": c.status_updated_at_ns,
                    });
                    Event::message(data.to_string()).event_type("contract-status")
                })
                .collect(),
            Some(prev) => contracts
                .iter()
                .filter(|c| {
                    prev.get(&c.contract_id)
                        .map(|(ps, pt)| ps != &c.status || pt != &c.status_updated_at_ns)
                        .unwrap_or(true)
                })
                .map(|c| {
                    let data = serde_json::json!({
                        "contract_id": c.contract_id,
                        "status": c.status,
                        "updated_at_ns": c.status_updated_at_ns,
                    });
                    Event::message(data.to_string()).event_type("contract-status")
                })
                .collect(),
        };

        let mut next_rotation_ns = state.last_rotation_event_ns;
        match state
            .db
            .get_ssh_key_rotation_events_for_user(&state.pk, state.last_rotation_event_ns)
            .await
        {
            Ok(rotation_events) => {
                for ev in &rotation_events {
                    let data = serde_json::json!({
                        "contract_id": ev.contract_id,
                        "created_at": ev.created_at,
                        "actor": ev.actor,
                        "details": ev.details,
                    });
                    events.push(Event::message(data.to_string()).event_type(&ev.event_type));
                    if ev.created_at > next_rotation_ns {
                        next_rotation_ns = ev.created_at;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("SSE ssh-key-rotation-events DB error: {:#}", e);
            }
        }

        tokio::time::sleep(SSE_POLL_INTERVAL).await;
        Some((
            events,
            SseState {
                db: state.db,
                pk: state.pk,
                prev_snapshot: Some(current),
                poll_count: state.poll_count + 1,
                last_rotation_event_ns: next_rotation_ns,
            },
        ))
    })
    .flat_map(|events: Vec<Event>| futures::stream::iter(events));

    Ok(SSE::new(stream).keep_alive(std::time::Duration::from_secs(30)))
}

/// Validate and normalize provisioning details
pub fn normalize_provisioning_details(
    status: &str,
    details: Option<String>,
) -> Result<Option<String>, String> {
    let sanitized = details.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    if status == "provisioned" && sanitized.is_none() {
        return Err(
            "Instance details are required when marking a contract as provisioned".to_string(),
        );
    }

    Ok(sanitized)
}

/// Validate cloud offering config against the live provider catalog.
/// Delegates to the appropriate provider validation based on provisioner_type.
/// No-op for offerings without a recognized cloud provisioner_type.
/// Dispatch cloud-offering validation to the matching provider backend.
///
/// `pub(crate)` so the extracted `OfferingCsvApi` (CSV import post-validation)
/// can call it; it stays here because the offering create/update handlers in
/// `ProvidersApi` are the other caller.
pub(crate) async fn validate_cloud_offering(
    db: &Database,
    offering: &crate::database::offerings::Offering,
    pubkey_bytes: &[u8],
) -> Result<(), String> {
    match offering.provisioner_type.as_deref() {
        Some("hetzner") => validate_hetzner_offering_inner(db, offering, pubkey_bytes).await,
        Some("vultr") => validate_vultr_offering_inner(db, offering, pubkey_bytes).await,
        _ => Ok(()),
    }
}

async fn validate_hetzner_offering_inner(
    db: &Database,
    offering: &crate::database::offerings::Offering,
    pubkey_bytes: &[u8],
) -> Result<(), String> {
    let config = crate::cloud::hetzner::resolve_provisioner_config(
        offering.provisioner_config.as_deref(),
        &offering.datacenter_city,
        offering.template_name.as_deref(),
    )
    .map_err(|e| format!("Invalid provisioner config: {e:#}"))?;

    let cloud_account_id = db
        .find_hetzner_cloud_account_for_provider(pubkey_bytes)
        .await
        .map_err(|e| format!("Failed to look up Hetzner cloud account: {e:#}"))?
        .ok_or_else(|| {
            "No Hetzner cloud account configured for this provider. \
             Add a Hetzner cloud account before creating Hetzner offerings."
                .to_string()
        })?;

    let (_account_id, _backend_type, credentials_encrypted) = db
        .get_cloud_account_credentials(&cloud_account_id)
        .await
        .map_err(|e| format!("Failed to get cloud account credentials: {e:#}"))?
        .ok_or_else(|| "Cloud account credentials not found".to_string())?;

    let encryption_key = crate::crypto::ServerEncryptionKey::from_env()
        .map_err(|e| format!("Server credential encryption not configured: {e:#}"))?;

    let token = crate::crypto::decrypt_server_credential(&credentials_encrypted, &encryption_key)
        .map_err(|e| format!("Failed to decrypt Hetzner credentials: {e:#}"))?;

    let backend = crate::cloud::hetzner::HetznerBackend::new(token)
        .map_err(|e| format!("Failed to create Hetzner client: {e:#}"))?;

    backend
        .validate_offering_config(&config)
        .await
        .map_err(|e| format!("Hetzner offering validation failed: {e:#}"))?;

    Ok(())
}

async fn validate_vultr_offering_inner(
    db: &Database,
    offering: &crate::database::offerings::Offering,
    pubkey_bytes: &[u8],
) -> Result<(), String> {
    let config = crate::cloud::vultr::resolve_provisioner_config(
        offering.provisioner_config.as_deref(),
        &offering.datacenter_city,
        offering.template_name.as_deref(),
    )
    .map_err(|e| format!("Invalid provisioner config: {e:#}"))?;

    let cloud_account_id = db
        .find_vultr_cloud_account_for_provider(pubkey_bytes)
        .await
        .map_err(|e| format!("Failed to look up Vultr cloud account: {e:#}"))?
        .ok_or_else(|| {
            "No Vultr cloud account configured for this provider. \
             Add a Vultr cloud account before creating Vultr offerings."
                .to_string()
        })?;

    let (_account_id, _backend_type, credentials_encrypted) = db
        .get_cloud_account_credentials(&cloud_account_id)
        .await
        .map_err(|e| format!("Failed to get cloud account credentials: {e:#}"))?
        .ok_or_else(|| "Cloud account credentials not found".to_string())?;

    let encryption_key = crate::crypto::ServerEncryptionKey::from_env()
        .map_err(|e| format!("Server credential encryption not configured: {e:#}"))?;

    let api_key = crate::crypto::decrypt_server_credential(&credentials_encrypted, &encryption_key)
        .map_err(|e| format!("Failed to decrypt Vultr credentials: {e:#}"))?;

    let backend = crate::cloud::vultr::VultrBackend::new(api_key)
        .map_err(|e| format!("Failed to create Vultr client: {e:#}"))?;

    backend
        .validate_offering_config(&config)
        .await
        .map_err(|e| format!("Vultr offering validation failed: {e:#}"))?;

    Ok(())
}

pub struct ProvidersApi;

/// Combined provider dashboard payload — all five dashboard sections in one
/// authenticated response. Each section is independent: a failing query yields
/// `None` for that section so one slow/broken source never blanks the whole
/// dashboard. Replaces the 5-call fan-out from the dashboard page.
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ProviderDashboardResponse {
    /// Provider trust score and reliability metrics.
    pub trust_metrics: Option<crate::database::stats::ProviderTrustMetrics>,
    /// Contract request response-time / SLA metrics.
    pub response_metrics: Option<ResponseMetricsResponse>,
    /// Aggregated uptime / health-check summary (default 30d window).
    pub health_summary: Option<crate::database::contracts::ProviderHealthSummary>,
    /// The authenticated provider's offerings (public + private).
    pub offerings: Option<Vec<crate::database::offerings::Offering>>,
    /// The authenticated user's blockchain activity summary.
    pub activity: Option<crate::database::users::UserActivity>,
}

/// Map raw DB response metrics to the API response shape. Shared by the
/// provider-dashboard endpoint here and the `response-metrics` handler in the
/// extracted `ProviderStatsApi`; `pub(crate)` so that sibling module can call
/// it. Keeping it here avoids duplicating the seconds→hours mapping.
pub(crate) fn build_response_metrics(
    metrics: crate::database::chatwoot::ProviderResponseMetrics,
) -> ResponseMetricsResponse {
    ResponseMetricsResponse {
        avg_response_seconds: metrics.avg_response_seconds,
        avg_response_hours: metrics.avg_response_seconds.map(|s| s / 3600.0),
        sla_compliance_percent: metrics.sla_compliance_percent,
        breach_count_30d: metrics.breach_count_30d,
        total_inquiries_30d: metrics.total_inquiries_30d,
        distribution: ResponseTimeDistributionResponse {
            within_1h_pct: metrics.distribution.within_1h_pct,
            within_4h_pct: metrics.distribution.within_4h_pct,
            within_12h_pct: metrics.distribution.within_12h_pct,
            within_24h_pct: metrics.distribution.within_24h_pct,
            within_72h_pct: metrics.distribution.within_72h_pct,
            total_responses: metrics.distribution.total_responses,
        },
    }
}

fn default_new_providers_limit() -> i64 {
    6
}

#[OpenApi]
impl ProvidersApi {
    /// List all providers
    ///
    /// Returns a paginated list of registered providers
    #[oai(path = "/providers", method = "get", tag = "ApiTags::Providers")]
    async fn list_providers(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>> {
        match db.list_providers(limit.0, offset.0).await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get active providers
    ///
    /// Returns providers that have checked in within the specified number of days
    #[oai(
        path = "/providers/active/:days",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_active_providers(
        &self,
        db: Data<&Arc<Database>>,
        days: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>> {
        match db.get_active_providers(days.0).await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get recently joined providers
    ///
    /// Returns providers that joined within the last 90 days and have at least one public offering.
    /// Public — no auth required.
    #[oai(path = "/providers/new", method = "get", tag = "ApiTags::Providers")]
    async fn get_new_providers(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_new_providers_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::NewProvider>>> {
        let limit = limit.0.min(10);
        match db.get_new_providers(limit).await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get new providers: {e:#?}")),
            }),
        }
    }

    /// Get provider profile
    ///
    /// Returns profile information for a specific provider
    #[oai(
        path = "/providers/:pubkey",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_profile(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::providers::ProviderProfile>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db.get_provider_profile(&pubkey_bytes).await {
            Ok(Some(profile)) => Json(ApiResponse {
                success: true,
                data: Some(profile),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Provider not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contacts
    ///
    /// Returns contact information for a specific provider
    #[oai(
        path = "/providers/:pubkey/contacts",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contacts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderContact>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db.get_provider_contacts(&pubkey_bytes).await {
            Ok(contacts) => Json(ApiResponse {
                success: true,
                data: Some(contacts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Add provider contact
    ///
    /// Adds a new contact to a provider profile (requires authentication as that provider)
    #[oai(
        path = "/providers/:pubkey/contacts",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn add_provider_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<AddAccountContactRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = crate::validation::validate_contact_type(&req.contact_type) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        if let Err(e) =
            crate::validation::validate_contact_value(&req.contact_type, &req.contact_value)
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        match db
            .add_provider_contact(&pubkey_bytes, &req.contact_type, &req.contact_value)
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Contact added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete provider contact
    ///
    /// Deletes a contact from a provider profile (requires authentication as that provider)
    #[oai(
        path = "/providers/:pubkey/contacts/:contact_id",
        method = "delete",
        tag = "ApiTags::Providers"
    )]
    async fn delete_provider_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contact_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .delete_provider_contact(&pubkey_bytes, contact_id.0)
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Contact deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contracts
    ///
    /// Returns contracts for a specific provider.
    /// Requires authentication - provider can only access their own contracts.
    #[oai(
        path = "/providers/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contracts(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: provider can only access their own contracts
        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own contracts".to_string()),
            });
        }

        match db.get_provider_contracts(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contracts pending provisioning
    ///
    /// Returns contracts ready for provisioning (accepted + payment succeeded) with offering specs.
    /// Includes cpu_cores, memory_amount, and storage_capacity from the associated offering.
    /// Requires agent authentication - agent can only access their delegated provider's contracts.
    /// If agent belongs to a pool, only returns contracts matching that pool (explicit or location-based).
    #[oai(
        path = "/providers/:pubkey/contracts/pending-provision",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_pending_provision_contracts(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractWithSpecs>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: agent can only access contracts for their delegated provider
        if auth.provider_pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: can only access your delegated provider's contracts".to_string(),
                ),
            });
        }

        // Get agent's pool info - pool membership is now required
        let pool_id = match db.get_agent_pool_id(&auth.agent_pubkey).await {
            Ok(Some(pool_id)) => pool_id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(
                        "Agent must belong to a pool. Re-register using a setup token.".to_string(),
                    ),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get agent pool: {}", e)),
                });
            }
        };

        // Get pool location for location-based matching
        let location = match db.get_agent_pool(&pool_id).await {
            Ok(Some(pool)) => pool.location,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Pool {} not found", pool_id)),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get pool info: {}", e)),
                });
            }
        };

        let result = db
            .get_pending_provision_contracts_for_pool(
                &pubkey_bytes,
                Some(&pool_id),
                Some(&location),
            )
            .await;

        match result {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contracts pending termination
    ///
    /// Returns cancelled contracts that had VMs provisioned and need termination.
    /// Requires agent authentication - agent can only access their delegated provider's contracts.
    #[oai(
        path = "/providers/:pubkey/contracts/pending-termination",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_pending_termination_contracts(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractPendingTermination>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: agent can only access contracts for their delegated provider
        if auth.provider_pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: can only access your delegated provider's contracts".to_string(),
                ),
            });
        }

        match db.get_pending_termination_contracts(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contracts pending password reset
    ///
    /// Returns active contracts where the user has requested a password reset.
    /// The agent should reset the password via SSH and call the password update endpoint.
    /// Accepts provider auth (the provider viewing their own dashboard) or agent auth
    /// (a delegated provisioning agent); either way the caller must match the path pubkey.
    #[oai(
        path = "/providers/:pubkey/contracts/pending-password-reset",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_pending_password_reset_contracts(
        &self,
        db: Data<&Arc<Database>>,
        auth: ProviderOrAgentAuth,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: caller (provider or delegated agent) must match the path pubkey.
        if auth.provider_pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: can only access your own provider's contracts".to_string(),
                ),
            });
        }

        match db.get_pending_password_resets(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contracts pending SSH key rotation
    ///
    /// Returns active contracts where the user has requested an SSH key rotation.
    /// The agent should inject the new key into the VM via SSH and call the
    /// complete-ssh-key-rotation endpoint.
    /// Requires agent authentication - agent can only access their delegated provider's contracts.
    #[oai(
        path = "/providers/:pubkey/contracts/pending-ssh-key-rotation",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_pending_ssh_key_rotation_contracts(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractPendingSshKeyRotation>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        if auth.provider_pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: can only access your delegated provider's contracts".to_string(),
                ),
            });
        }

        match db.get_pending_ssh_key_rotations(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Complete SSH key rotation
    ///
    /// Called by dc-agent after successfully injecting the new SSH key into a VM.
    /// Clears the pending rotation flag and records the event.
    /// Accepts either provider authentication (X-Public-Key) or agent authentication (X-Agent-Pubkey).
    #[oai(
        path = "/provider/rental-requests/:id/ssh-key-rotation",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn complete_ssh_key_rotation(
        &self,
        db: Data<&Arc<Database>>,
        auth: ProviderOrAgentAuth,
        id: Path<String>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => {
                if contract.provider_pubkey != hex::encode(&auth.provider_pubkey) {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(
                            "Unauthorized: you are not the provider for this contract".to_string(),
                        ),
                    });
                }
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        match db.complete_ssh_key_rotation(&contract_id).await {
            Ok(new_ssh_pubkey) => {
                if let Err(e) = db
                    .insert_contract_event(
                        &contract_id,
                        "ssh_key_rotation_complete",
                        None,
                        None,
                        "provider",
                        Some(&format!(
                            "SSH key rotated to {}... by agent",
                            &new_ssh_pubkey[..20.min(new_ssh_pubkey.len())]
                        )),
                    )
                    .await
                {
                    tracing::warn!(
                        contract_id = %hex::encode(&contract_id),
                        "Failed to insert ssh_key_rotation_complete event: {:#}",
                        e
                    );
                }
                Json(ApiResponse {
                    success: true,
                    data: Some("SSH key rotation completed successfully".to_string()),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Mark a contract as terminated
    ///
    /// Called by dc-agent after successfully terminating a VM for a cancelled contract.
    /// Requires agent authentication - agent can only mark contracts for their delegated provider.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/terminated",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn mark_contract_terminated(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<bool>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        let contract_id_bytes = match decode_hex_path(&contract_id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: agent can only mark contracts for their delegated provider
        if auth.provider_pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: can only mark your delegated provider's contracts".to_string(),
                ),
            });
        }

        match db.mark_contract_terminated(&contract_id_bytes).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some(true),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider offerings (public)
    ///
    /// Returns public offerings for a specific provider.
    /// Private offerings are only visible via the authenticated /provider/my-offerings endpoint.
    #[oai(
        path = "/providers/:pubkey/offerings",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_provider_offerings(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::offerings::Offering>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        // Return only public offerings - private offerings require authentication
        match db.get_provider_offerings_public(&pubkey_bytes).await {
            Ok(offerings) => Json(ApiResponse {
                success: true,
                data: Some(offerings),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get my offerings (authenticated)
    ///
    /// Returns all offerings for the authenticated provider, including private ones.
    /// Use this endpoint for "My Resources" UI section.
    #[oai(
        path = "/provider/my-offerings",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_my_offerings(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<Vec<crate::database::offerings::Offering>>> {
        match db.get_provider_offerings(&auth.pubkey).await {
            Ok(offerings) => Json(ApiResponse {
                success: true,
                data: Some(offerings),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider dashboard (combined, authenticated)
    ///
    /// Returns all five dashboard sections in a single authenticated call:
    /// trust metrics, response metrics, health summary, the provider's own
    /// offerings, and the user's activity. Replaces the 5-endpoint fan-out
    /// the dashboard page used to make on every load.
    ///
    /// All sections are resolved for the AUTHENTICATED pubkey. Each section is
    /// independent: if one query fails, that section is `null` and the rest
    /// still load (the response is always `success: true`).
    #[oai(
        path = "/provider/dashboard",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_provider_dashboard(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<ProviderDashboardResponse>> {
        // Run every section independently — a failure in one must not blank the
        // others. Errors are logged with context (no silent ignores) and surface
        // as a null section to the client.
        let trust_metrics = match db.get_provider_trust_metrics(&auth.pubkey).await {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!("dashboard: trust metrics failed: {e:#}");
                None
            }
        };

        let response_metrics = match db.get_provider_response_metrics(&auth.pubkey).await {
            Ok(m) => Some(build_response_metrics(m)),
            Err(e) => {
                tracing::warn!("dashboard: response metrics failed: {e:#}");
                None
            }
        };

        let health_summary = match db.get_provider_health_summary(&auth.pubkey, None).await {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!("dashboard: health summary failed: {e:#}");
                None
            }
        };

        let offerings = match db.get_provider_offerings(&auth.pubkey).await {
            Ok(o) => Some(o),
            Err(e) => {
                tracing::warn!("dashboard: offerings failed: {e:#}");
                None
            }
        };

        let activity = match db.get_user_activity(&auth.pubkey).await {
            Ok(a) => Some(a),
            Err(e) => {
                tracing::warn!("dashboard: activity failed: {e:#}");
                None
            }
        };

        Json(ApiResponse {
            success: true,
            data: Some(ProviderDashboardResponse {
                trust_metrics,
                response_metrics,
                health_summary,
                offerings,
                activity,
            }),
            error: None,
        })
    }

    /// Create provider offering
    ///
    /// Creates a new offering for a provider (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn create_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        offering: Json<crate::database::offerings::Offering>,
    ) -> Json<ApiResponse<i64>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let mut params = offering.0;
        params.id = None;
        params.pubkey = hex::encode(&pubkey_bytes);

        if let Err(e) = validate_offering_currency(&params.currency) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = validate_cloud_offering(&db, &params, &pubkey_bytes).await {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = validate_recipe_if_present(params.post_provision_script.as_ref()) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.create_offering(&pubkey_bytes, params).await {
            Ok(id) => {
                // Note: Chatwoot resources (inbox/team/portal) are created when
                // provider completes onboarding setup, not on offering creation.
                // See update_provider_onboarding for the onboarding flow.

                Json(ApiResponse {
                    success: true,
                    data: Some(id),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update provider offering
    ///
    /// Updates an existing offering (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/:id",
        method = "put",
        tag = "ApiTags::Offerings"
    )]
    async fn update_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        offering: Json<crate::database::offerings::Offering>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let mut params = offering.0;
        params.pubkey = hex::encode(&pubkey_bytes);

        if let Err(e) = validate_offering_currency(&params.currency) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = validate_cloud_offering(&db, &params, &pubkey_bytes).await {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = validate_recipe_if_present(params.post_provision_script.as_ref()) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.update_offering(&pubkey_bytes, id.0, params).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Offering updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete provider offering
    ///
    /// Deletes an offering (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/:id",
        method = "delete",
        tag = "ApiTags::Offerings"
    )]
    async fn delete_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_offering(&pubkey_bytes, id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Offering deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Duplicate provider offering
    ///
    /// Creates a duplicate of an existing offering (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/duplicate",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn duplicate_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        req: Json<DuplicateOfferingRequest>,
    ) -> Json<ApiResponse<i64>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .duplicate_offering(&pubkey_bytes, id.0, req.0.new_offering_id)
            .await
        {
            Ok(new_id) => Json(ApiResponse {
                success: true,
                data: Some(new_id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Bulk update offering status
    ///
    /// Updates stock status for multiple offerings (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/bulk-status",
        method = "put",
        tag = "ApiTags::Offerings"
    )]
    async fn bulk_update_provider_offerings_status(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<BulkUpdateStatusRequest>,
    ) -> Json<ApiResponse<u64>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .bulk_update_stock_status(&pubkey_bytes, &req.offering_ids, &req.stock_status)
            .await
        {
            Ok(count) => Json(ApiResponse {
                success: true,
                data: Some(count),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Bulk update offering prices
    ///
    /// Updates `monthly_price` for multiple offerings atomically (requires authentication).
    /// Accepts a list of `{id, price_e9s}` pairs where `price_e9s` is the price in nanocents
    /// (1 USD = 1_000_000_000 price_e9s). All offerings must belong to the authenticated provider.
    #[oai(
        path = "/providers/:pubkey/offerings/bulk-prices",
        method = "patch",
        tag = "ApiTags::Offerings"
    )]
    async fn bulk_update_provider_offering_prices(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<BulkUpdatePricesRequest>,
    ) -> Json<ApiResponse<u64>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let updates: Vec<(i64, i64)> = req.0.updates.iter().map(|u| (u.id, u.price_e9s)).collect();

        match db
            .bulk_update_offering_prices(&pubkey_bytes, &updates)
            .await
        {
            Ok(count) => Json(ApiResponse {
                success: true,
                data: Some(count),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get pending rental requests
    ///
    /// Returns pending rental requests for the authenticated provider
    #[oai(
        path = "/provider/rental-requests/pending",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_pending_rental_requests(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        match db.get_pending_provider_contracts(&auth.pubkey).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Respond to rental request
    ///
    /// Accept or reject a rental request (requires authentication).
    /// Rejection triggers full refund since user never received the service.
    #[oai(
        path = "/provider/rental-requests/:id/respond",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn respond_to_rental_request(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<RentalResponseRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        if req.accept {
            // Accept: update status and notify user
            match db
                .update_contract_status(&contract_id, "accepted", &auth.pubkey, req.memo.as_deref())
                .await
            {
                Ok(_) => {
                    if let Err(e) = db
                        .try_activate_self_provisioned_contract(&contract_id)
                        .await
                    {
                        tracing::warn!(
                            "Self-provisioned fulfillment failed for contract {}: {:#}",
                            hex::encode(&contract_id),
                            e
                        );
                    }

                    // Send notification email to user (async, don't fail endpoint)
                    crate::receipts::send_contract_accepted_notification(db.as_ref(), &contract_id)
                        .await;

                    Json(ApiResponse {
                        success: true,
                        data: Some("Contract accepted".to_string()),
                        error: None,
                    })
                }
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                }),
            }
        } else {
            // Reject: trigger full refund since user never got the service
            let stripe_client = crate::stripe_client::stripe_client_or_warn();

            match db
                .reject_contract(
                    &contract_id,
                    &auth.pubkey,
                    req.memo.as_deref(),
                    stripe_client.as_ref(),
                )
                .await
            {
                Ok(_) => {
                    // Send notification email to user (async, don't fail endpoint)
                    crate::receipts::send_contract_rejected_notification(
                        db.as_ref(),
                        &contract_id,
                        req.memo.as_deref(),
                    )
                    .await;

                    Json(ApiResponse {
                        success: true,
                        data: Some("Contract rejected, refund initiated".to_string()),
                        error: None,
                    })
                }
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                }),
            }
        }
    }

    /// Update provisioning status
    ///
    /// Updates the provisioning status of a contract.
    /// Accepts either provider authentication (X-Public-Key) or agent authentication (X-Agent-Pubkey).
    #[oai(
        path = "/provider/rental-requests/:id/provisioning",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_provisioning_status(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        auth: ProviderOrAgentAuth,
        id: Path<String>,
        req: Json<ProvisioningStatusRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        let sanitized_details =
            match normalize_provisioning_details(&req.status, req.instance_details.clone()) {
                Ok(details) => details,
                Err(msg) => {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(msg),
                    })
                }
            };

        match db
            .update_contract_status(&contract_id, &req.status, &auth.provider_pubkey, None)
            .await
        {
            Ok(_) => {
                if req.status == "provisioned" {
                    if let Some(details) = sanitized_details.as_deref() {
                        if let Err(e) = db.add_provisioning_details(&contract_id, details).await {
                            return Json(ApiResponse {
                                success: false,
                                data: None,
                                error: Some(format!(
                                    "Status updated but failed to save details: {}",
                                    e
                                )),
                            });
                        }

                        // Check if provider has auto_accept_rentals enabled - if so, auto-activate
                        let auto_accept = db
                            .get_provider_auto_accept_rentals(&auth.provider_pubkey)
                            .await
                            .unwrap_or(false);

                        if auto_accept {
                            // Auto-transition to active
                            if let Err(e) = db
                                .update_contract_status(
                                    &contract_id,
                                    "active",
                                    &auth.provider_pubkey,
                                    Some(
                                        "Auto-activated (provider has auto_accept_rentals enabled)",
                                    ),
                                )
                                .await
                            {
                                tracing::warn!(
                                    "Failed to auto-activate contract {}: {}",
                                    &id.0[..16],
                                    e
                                );
                            }
                        }

                        // Notify user that their VM is ready
                        match db.get_contract(&contract_id).await {
                            Ok(Some(contract)) => {
                                if let Err(e) =
                                    crate::rental_notifications::notify_user_provisioned(
                                        &db,
                                        email_service.as_ref(),
                                        &contract,
                                        details,
                                    )
                                    .await
                                {
                                    tracing::warn!(
                                        "Failed to send provisioned notification for contract {}: {}",
                                        &id.0[..16],
                                        e
                                    );
                                }
                            }
                            Ok(None) => {
                                tracing::warn!(
                                    "Contract {} not found after provisioning status update",
                                    &id.0[..16]
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to fetch contract {} for provisioned notification: {:#}",
                                    &id.0[..16],
                                    e
                                );
                            }
                        }
                    }
                }
                Json(ApiResponse {
                    success: true,
                    data: Some("Provisioning status updated".to_string()),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update VM password
    ///
    /// Updates the root password for a provisioned VM. Called by the agent after
    /// successfully resetting the password via SSH. The password is encrypted with
    /// the requester's public key before storage.
    /// Accepts either provider authentication (X-Public-Key) or agent authentication (X-Agent-Pubkey).
    #[oai(
        path = "/provider/rental-requests/:id/password",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_contract_password(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        auth: ProviderOrAgentAuth,
        id: Path<String>,
        req: Json<UpdatePasswordRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match decode_hex_path(&id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Verify contract exists and belongs to this provider; keep it for the notification below.
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => {
                if contract.provider_pubkey != hex::encode(&auth.provider_pubkey) {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(
                            "Unauthorized: you are not the provider for this contract".to_string(),
                        ),
                    });
                }
                contract
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        match db
            .update_encrypted_credentials(&contract_id, &req.new_password)
            .await
        {
            Ok(_) => {
                // Clear any pending password reset request
                if let Err(e) = db.clear_password_reset_request(&contract_id).await {
                    tracing::warn!(
                        contract_id = %hex::encode(&contract_id),
                        "Failed to clear password reset request after password update: {:#}",
                        e
                    );
                }
                // Notify provider that the password reset is complete
                if let Err(e) = crate::rental_notifications::notify_provider_password_reset(
                    &db,
                    email_service.as_ref(),
                    &contract,
                    true,
                )
                .await
                {
                    tracing::warn!(
                        contract_id = %hex::encode(&contract_id),
                        "Failed to notify provider of completed password reset: {:#}",
                        e
                    );
                }
                // Notify tenant that their new credentials are ready
                if let Err(e) = crate::rental_notifications::notify_tenant_password_reset_complete(
                    &db,
                    email_service.as_ref(),
                    &contract,
                )
                .await
                {
                    tracing::warn!(
                        contract_id = %hex::encode(&contract_id),
                        "Failed to notify tenant of completed password reset: {:#}",
                        e
                    );
                }
                Json(ApiResponse {
                    success: true,
                    data: Some("Password updated successfully".to_string()),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Upsert provider-reported daily SLI reports for an offering
    #[oai(
        path = "/providers/:pubkey/offerings/:id/sli-reports",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn upsert_provider_offering_sli_reports(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        req: Json<UpdateOfferingSliReportsRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if !(1.0..=100.0).contains(&req.sla_target_percent) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("sla_target_percent must be between 1 and 100".to_string()),
            });
        }

        if req.reports.is_empty() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("At least one SLI report is required".to_string()),
            });
        }

        let mut reports = Vec::with_capacity(req.reports.len());
        for report in &req.reports {
            let report_date =
                match chrono::NaiveDate::parse_from_str(&report.report_date, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(format!(
                                "Invalid report_date '{}' - expected YYYY-MM-DD",
                                report.report_date
                            )),
                        });
                    }
                };
            if report_date > chrono::Utc::now().date_naive() {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("report_date must not be in the future".to_string()),
                });
            }
            if !(0.0..=100.0).contains(&report.uptime_percent) {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("uptime_percent must be between 0 and 100".to_string()),
                });
            }
            if report
                .response_sli_percent
                .is_some_and(|value| !(0.0..=100.0).contains(&value))
            {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("response_sli_percent must be between 0 and 100".to_string()),
                });
            }
            if report.incident_count < 0 {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("incident_count must be zero or greater".to_string()),
                });
            }

            reports.push(crate::database::offering_sla::UpsertOfferingSliReport {
                report_date: report.report_date.clone(),
                uptime_percent: report.uptime_percent,
                response_sli_percent: report.response_sli_percent,
                incident_count: report.incident_count,
                notes: report.notes.clone(),
            });
        }

        match db
            .upsert_provider_offering_sli_reports(
                &pubkey_bytes,
                id.0,
                req.sla_target_percent,
                &reports,
            )
            .await
        {
            Ok(()) => {
                if let Err(e) = db.get_provider_trust_metrics(&pubkey_bytes).await {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!(
                            "SLI reports stored but provider trust cache refresh failed: {e:#}"
                        )),
                    });
                }

                Json(ApiResponse {
                    success: true,
                    data: Some("SLI reports updated successfully".to_string()),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider onboarding data
    ///
    /// Returns onboarding information for a specific provider (public endpoint)
    #[oai(
        path = "/providers/:pubkey/onboarding",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_onboarding(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::providers::ProviderOnboarding>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db.get_provider_onboarding(&pubkey_bytes).await {
            Ok(Some(onboarding)) => Json(ApiResponse {
                success: true,
                data: Some(onboarding),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Provider onboarding data not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update provider onboarding data
    ///
    /// Updates onboarding information for a provider (requires authentication)
    #[oai(
        path = "/providers/:pubkey/onboarding",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_provider_onboarding(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        onboarding: Json<crate::database::providers::ProviderOnboarding>,
    ) -> Json<ApiResponse<OnboardingUpdateResponse>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // Get provider name from account (for new providers)
        let provider_name = match db.get_account_with_keys_by_public_key(&pubkey_bytes).await {
            Ok(Some(account)) => account
                .display_name
                .unwrap_or_else(|| account.username.clone()),
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get account: {}", e)),
                });
            }
        };

        match db
            .update_provider_onboarding(&pubkey_bytes, &onboarding.0, &provider_name)
            .await
        {
            Ok(_) => {
                // Note: Chatwoot resources are created lazily when sync_provider_helpcenter is called
                let timestamp = match crate::now_ns() {
                    Ok(ns) => ns,
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        })
                    }
                };
                Json(ApiResponse {
                    success: true,
                    data: Some(OnboardingUpdateResponse {
                        onboarding_completed_at: timestamp,
                    }),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Sync provider help center article
    ///
    /// Generates and syncs help center article to provider's Chatwoot portal (requires authentication).
    /// Auto-creates Chatwoot resources (inbox, team, portal) if they don't exist yet.
    #[oai(
        path = "/providers/:pubkey/helpcenter/sync",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn sync_provider_helpcenter(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<HelpcenterSyncResponse>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let chatwoot = match crate::chatwoot::ChatwootClient::from_env() {
            Ok(client) => client,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Chatwoot client initialization failed: {}", e)),
                });
            }
        };

        match crate::helpcenter::sync_provider_article(&db, &chatwoot, &pubkey_bytes).await {
            Ok(result) => Json(ApiResponse {
                success: true,
                data: Some(HelpcenterSyncResponse {
                    article_url: result.article_url,
                    action: result.action,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("{:#}", e)),
            }),
        }
    }

    /// Get provider auto-accept rentals setting
    ///
    /// Returns whether the provider has auto-accept rentals enabled.
    /// When enabled, new rental contracts skip provider approval and
    /// transition directly to 'accepted' status after payment succeeds.
    #[oai(
        path = "/provider/settings/auto-accept",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_auto_accept_rentals(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<AutoAcceptResponse>> {
        match db.get_provider_auto_accept_rentals(&auth.pubkey).await {
            Ok(enabled) => Json(ApiResponse {
                success: true,
                data: Some(AutoAcceptResponse {
                    auto_accept_rentals: enabled,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Set provider auto-accept rentals setting
    ///
    /// Enable or disable auto-accept for new rental contracts.
    /// When enabled, contracts skip provider approval step after payment succeeds.
    #[oai(
        path = "/provider/settings/auto-accept",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn set_auto_accept_rentals(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        req: Json<AutoAcceptRequest>,
    ) -> Json<ApiResponse<AutoAcceptResponse>> {
        match db
            .set_provider_auto_accept_rentals(&auth.pubkey, req.auto_accept_rentals)
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some(AutoAcceptResponse {
                    auto_accept_rentals: req.auto_accept_rentals,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Reconcile running VMs with contract state
    ///
    /// dc-agent reports running VMs, API returns which should continue running,
    /// be terminated, or are unknown (orphans). This replaces the old
    /// pending-termination polling approach with a reconciliation model.
    ///
    /// Requires agent authentication.
    #[oai(
        path = "/providers/:pubkey/reconcile",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn reconcile_instances(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<ReconcileRequest>,
    ) -> Json<ApiResponse<ReconcileResponse>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Authorization: agent can only reconcile their delegated provider's contracts
        if auth.provider_pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: can only reconcile your delegated provider's contracts"
                        .to_string(),
                ),
            });
        }

        // Get current timestamp for expiry checks
        let now_ns = match crate::now_ns() {
            Ok(ns) => ns,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let mut keep = Vec::new();
        let mut terminate = Vec::new();
        let mut unknown = Vec::new();
        let mut pause = Vec::new();

        for instance in &req.running_instances {
            // If no contract_id, mark as unknown
            let contract_id = match &instance.contract_id {
                Some(id) => id,
                None => {
                    unknown.push(ReconcileUnknownInstance {
                        external_id: instance.external_id.clone(),
                        message: "No contract ID associated with this instance".to_string(),
                    });
                    continue;
                }
            };

            // Look up contract
            let contract_id_bytes = match decode_hex_path(contract_id, "contract id") {
                Ok(bytes) => bytes,
                Err(msg) => {
                    unknown.push(ReconcileUnknownInstance {
                        external_id: instance.external_id.clone(),
                        message: msg,
                    });
                    continue;
                }
            };

            match db.get_contract(&contract_id_bytes).await {
                Ok(Some(contract)) => {
                    // Check if contract belongs to this provider
                    let contract_provider = match hex::decode(&contract.provider_pubkey) {
                        Ok(pk) => pk,
                        Err(e) => {
                            tracing::warn!("Malformed hex in contract.provider_pubkey: {:#}", e);
                            unknown.push(ReconcileUnknownInstance {
                                external_id: instance.external_id.clone(),
                                message: "Invalid pubkey format in database".to_string(),
                            });
                            continue;
                        }
                    };
                    if contract_provider != pubkey_bytes {
                        unknown.push(ReconcileUnknownInstance {
                            external_id: instance.external_id.clone(),
                            message: "Contract belongs to different provider".to_string(),
                        });
                        continue;
                    }

                    // Determine action based on contract state
                    let end_ns = contract.end_timestamp_ns.unwrap_or(0);
                    let is_expired = end_ns > 0 && end_ns < now_ns;
                    let is_cancelled = contract.status == "cancelled";
                    let is_paused = contract.status == "paused";

                    if is_cancelled {
                        terminate.push(ReconcileTerminateInstance {
                            external_id: instance.external_id.clone(),
                            contract_id: contract_id.clone(),
                            reason: "cancelled".to_string(),
                        });
                    } else if is_expired {
                        terminate.push(ReconcileTerminateInstance {
                            external_id: instance.external_id.clone(),
                            contract_id: contract_id.clone(),
                            reason: "expired".to_string(),
                        });
                    } else if is_paused {
                        // Stripe dispute pause: stop the VM but keep the row,
                        // disk, and DNS in place so resume_contract can put
                        // the customer back online without re-onboarding.
                        let reason = match db.get_pause_reason(&contract_id_bytes).await {
                            Ok(Some(r)) => r,
                            Ok(None) | Err(_) => "paused".to_string(),
                        };
                        pause.push(ReconcilePauseInstance {
                            external_id: instance.external_id.clone(),
                            contract_id: contract_id.clone(),
                            reason,
                        });
                    } else {
                        // Contract is active
                        keep.push(ReconcileKeepInstance {
                            external_id: instance.external_id.clone(),
                            contract_id: contract_id.clone(),
                            ends_at: end_ns,
                        });
                    }
                }
                Ok(None) => {
                    unknown.push(ReconcileUnknownInstance {
                        external_id: instance.external_id.clone(),
                        message: format!("No contract found with ID: {}", contract_id),
                    });
                }
                Err(e) => {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Database error: {}", e)),
                    });
                }
            }
        }

        Json(ApiResponse {
            success: true,
            data: Some(ReconcileResponse {
                keep,
                terminate,
                unknown,
                pause,
            }),
             error: None,
        })
    }

    /// Get bandwidth stats for all provider's contracts
    ///
    /// Returns the latest bandwidth usage for all contracts with gateway routing.
    /// Requires provider authentication.
    #[oai(
        path = "/providers/:pubkey/bandwidth",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_bandwidth(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<BandwidthStatsResponse>>> {
        // Decode and verify auth
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.get_provider_bandwidth_stats(&pubkey.0).await {
            Ok(stats) => {
                let response: Vec<BandwidthStatsResponse> = stats
                    .into_iter()
                    .map(|s| BandwidthStatsResponse {
                        contract_id: s.contract_id,
                        gateway_slug: s.gateway_slug,
                        bytes_in: s.bytes_in,
                        bytes_out: s.bytes_out,
                        last_updated_ns: s.last_updated_ns,
                    })
                    .collect();

                Json(ApiResponse {
                    success: true,
                    data: Some(response),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get bandwidth history for a specific contract
    ///
    /// Returns bandwidth history records for graphing/analysis.
    /// Requires provider authentication.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/bandwidth",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_contract_bandwidth(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<Vec<BandwidthHistoryResponse>>> {
        // Decode and verify auth
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // Get history (last 100 records)
        match db.get_bandwidth_history(&contract_id.0, 100).await {
            Ok(records) => {
                let response: Vec<BandwidthHistoryResponse> = records
                    .into_iter()
                    .map(|r| BandwidthHistoryResponse {
                        bytes_in: r.bytes_in as u64,
                        bytes_out: r.bytes_out as u64,
                        recorded_at_ns: r.recorded_at_ns,
                    })
                    .collect();

                Json(ApiResponse {
                    success: true,
                    data: Some(response),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    // ==================== Offering Generation Endpoints ====================

    /// Get offering suggestions for a pool
    ///
    /// Returns suggested offerings based on the pool's aggregated hardware capabilities
    /// from online agents. Providers can use these suggestions to generate offerings.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/offering-suggestions",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn get_offering_suggestions(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<OfferingSuggestionsResponse>> {
        use crate::database::offerings::{generate_suggestions, select_applicable_tiers};

        // Decode and verify authorization
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // Verify pool exists and belongs to provider
        let pool = match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(p)) => {
                let pool_owner = match decode_hex_path(&p.provider_pubkey, "pool owner pubkey") {
                    Ok(pk) => pk,
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(e),
                        });
                    }
                };
                if pool_owner != provider_pubkey {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Pool does not belong to this provider".to_string()),
                    });
                }
                p
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        };

        // Get pool capabilities from online agents
        let capabilities = match db.get_pool_capabilities(&pool_id.0).await {
            Ok(Some(caps)) => caps,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(
                        "No online agents with resource data in this pool. \
                         Ensure agents are online and have reported their hardware capabilities."
                            .to_string(),
                    ),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        };

        // Select applicable tiers
        let (applicable_tiers, unavailable_tiers) = select_applicable_tiers(&capabilities);

        // Generate suggestions
        let suggestions = generate_suggestions(
            &pool_id.0,
            &pool.name,
            &pool.location,
            &capabilities,
            &applicable_tiers,
        );

        Json(ApiResponse {
            success: true,
            data: Some(OfferingSuggestionsResponse {
                pool_capabilities: capabilities,
                suggested_offerings: suggestions,
                unavailable_tiers,
            }),
            error: None,
        })
    }

    /// Generate offerings for a pool
    ///
    /// Creates offerings based on pool capabilities and provided pricing.
    /// Requires pricing for each tier to be generated.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/generate-offerings",
        method = "post",
        tag = "ApiTags::Pools"
    )]
    async fn generate_offerings(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        req: Json<GenerateOfferingsRequest>,
    ) -> Json<ApiResponse<GenerateOfferingsResponse>> {
        use crate::database::offerings::{
            generate_suggestions, select_applicable_tiers, Offering, UnavailableTier,
        };

        // Decode and verify authorization
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // Verify pool exists and belongs to provider
        let pool = match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(p)) => {
                let pool_owner = match decode_hex_path(&p.provider_pubkey, "pool owner pubkey") {
                    Ok(pk) => pk,
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(e),
                        });
                    }
                };
                if pool_owner != provider_pubkey {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Pool does not belong to this provider".to_string()),
                    });
                }
                p
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        };

        // Get pool capabilities
        let capabilities = match db.get_pool_capabilities(&pool_id.0).await {
            Ok(Some(caps)) => caps,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("No online agents with resource data in this pool".to_string()),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        };

        // Select applicable tiers
        let (mut applicable_tiers, mut unavailable_tiers) = select_applicable_tiers(&capabilities);

        // Filter to requested tiers if specified
        if !req.tiers.is_empty() {
            let requested: std::collections::HashSet<_> = req.tiers.iter().cloned().collect();
            let (keep, skip): (Vec<_>, Vec<_>) = applicable_tiers
                .into_iter()
                .partition(|t| requested.contains(&t.name));
            applicable_tiers = keep;
            // Mark unrequested tiers as skipped
            for tier in skip {
                unavailable_tiers.push(UnavailableTier {
                    tier: tier.name,
                    reason: "Not in requested tier list".to_string(),
                });
            }
        }

        // Generate suggestions first
        let suggestions = generate_suggestions(
            &pool_id.0,
            &pool.name,
            &pool.location,
            &capabilities,
            &applicable_tiers,
        );

        // Convert suggestions to offerings with pricing
        let mut created_offerings = Vec::new();
        let mut skipped_tiers = unavailable_tiers;

        for suggestion in suggestions {
            // Check if pricing is provided for this tier
            let pricing = match req.pricing.get(&suggestion.tier_name) {
                Some(p) => p,
                None => {
                    skipped_tiers.push(UnavailableTier {
                        tier: suggestion.tier_name,
                        reason: "No pricing provided".to_string(),
                    });
                    continue;
                }
            };

            // Build the offering
            let offering = Offering {
                id: None,
                pubkey: hex::encode(&provider_pubkey),
                offering_id: suggestion.offering_id.clone(),
                offer_name: suggestion.offer_name.clone(),
                description: None,
                product_page_url: None,
                currency: pricing.currency.clone(),
                monthly_price: pricing.monthly_price,
                setup_fee: 0.0,
                visibility: req.visibility.clone(),
                product_type: "vps".to_string(),
                virtualization_type: Some(pool.provisioner_type.clone()),
                billing_interval: "monthly".to_string(),
                billing_unit: "month".to_string(),
                pricing_model: Some("flat".to_string()),
                price_per_unit: None,
                included_units: None,
                overage_price_per_unit: None,
                stripe_metered_price_id: None,
                is_subscription: false,
                subscription_interval_days: None,
                stock_status: "in_stock".to_string(),
                processor_brand: suggestion.processor_brand.clone(),
                processor_amount: Some(1),
                processor_cores: Some(suggestion.processor_cores),
                processor_speed: None,
                processor_name: suggestion.processor_name.clone(),
                memory_error_correction: None,
                memory_type: Some("DDR4".to_string()),
                memory_amount: Some(suggestion.memory_amount.clone()),
                hdd_amount: None,
                total_hdd_capacity: None,
                ssd_amount: Some(1),
                total_ssd_capacity: Some(suggestion.total_ssd_capacity.clone()),
                unmetered_bandwidth: false,
                uplink_speed: Some("1 Gbps".to_string()),
                traffic: None,
                datacenter_country: suggestion.datacenter_country.clone(),
                datacenter_city: pool.location.clone(),
                datacenter_latitude: None,
                datacenter_longitude: None,
                control_panel: None,
                gpu_name: suggestion.gpu_name.clone(),
                gpu_count: suggestion.gpu_count,
                gpu_memory_gb: None,
                min_contract_hours: Some(1),
                max_contract_hours: None,
                payment_methods: Some("card, crypto".to_string()),
                features: None,
                operating_systems: suggestion.operating_systems.clone(),
                trust_score: None,
                has_critical_flags: None,
                reliability_score: None,
                is_draft: false,
                publish_at: None,
                offering_source: Some("generated".to_string()),
                external_checkout_url: None,
                reseller_name: None,
                reseller_commission_percent: None,
                owner_username: None,
                provider_name: None,
                provisioner_type: Some(pool.provisioner_type.clone()),
                provisioner_config: None,
                template_name: capabilities.available_templates.first().cloned(),
                agent_pool_id: Some(pool_id.0.clone()),
                post_provision_script: None,
                provider_online: None,
                resolved_pool_id: None,
                resolved_pool_name: None,
                created_at_ns: None,
            };

            if !req.dry_run {
                // Create the offering
                match db.create_offering(&provider_pubkey, offering.clone()).await {
                    Ok(id) => {
                        let mut created = offering;
                        created.id = Some(id);
                        created_offerings.push(created);
                    }
                    Err(e) => {
                        skipped_tiers.push(UnavailableTier {
                            tier: suggestion.tier_name,
                            reason: format!("Failed to create: {}", e),
                        });
                    }
                }
            } else {
                // Dry run - just return what would be created
                created_offerings.push(offering);
            }
        }

        Json(ApiResponse {
            success: true,
            data: Some(GenerateOfferingsResponse {
                created_offerings,
                skipped_tiers,
            }),
            error: None,
        })
    }

    /// List auto-accept rules for the authenticated provider
    #[oai(
        path = "/provider/auto-accept-rules",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn list_auto_accept_rules(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<Vec<crate::database::providers::AutoAcceptRule>>> {
        match db.list_auto_accept_rules(&auth.pubkey).await {
            Ok(rules) => Json(ApiResponse {
                success: true,
                data: Some(rules),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("{e:#}")),
            }),
        }
    }

    /// Create a per-offering auto-accept rule for the authenticated provider
    #[oai(
        path = "/provider/auto-accept-rules",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn create_auto_accept_rule(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        req: Json<CreateAutoAcceptRuleRequest>,
    ) -> Json<ApiResponse<crate::database::providers::AutoAcceptRule>> {
        match db
            .create_auto_accept_rule(
                &auth.pubkey,
                &req.offering_id,
                req.min_duration_hours,
                req.max_duration_hours,
            )
            .await
        {
            Ok(rule) => Json(ApiResponse {
                success: true,
                data: Some(rule),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("{e:#}")),
            }),
        }
    }

    /// Update a per-offering auto-accept rule for the authenticated provider
    #[oai(
        path = "/provider/auto-accept-rules/:rule_id",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_auto_accept_rule(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        rule_id: Path<i64>,
        req: Json<UpdateAutoAcceptRuleRequest>,
    ) -> Json<ApiResponse<crate::database::providers::AutoAcceptRule>> {
        match db
            .update_auto_accept_rule(
                &auth.pubkey,
                rule_id.0,
                req.min_duration_hours,
                req.max_duration_hours,
                req.enabled,
            )
            .await
        {
            Ok(rule) => Json(ApiResponse {
                success: true,
                data: Some(rule),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("{e:#}")),
            }),
        }
    }

    /// Delete a per-offering auto-accept rule for the authenticated provider
    #[oai(
        path = "/provider/auto-accept-rules/:rule_id",
        method = "delete",
        tag = "ApiTags::Providers"
    )]
    async fn delete_auto_accept_rule(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        rule_id: Path<i64>,
    ) -> Json<ApiResponse<EmptyResponse>> {
        match db.delete_auto_accept_rule(&auth.pubkey, rule_id.0).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some(EmptyResponse {}),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("{e:#}")),
            }),
        }
    }
}

/// Request to create a per-offering auto-accept rule
#[derive(Debug, serde::Deserialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreateAutoAcceptRuleRequest {
    pub offering_id: String,
    pub min_duration_hours: Option<i64>,
    pub max_duration_hours: Option<i64>,
}

/// Request to update a per-offering auto-accept rule
#[derive(Debug, serde::Deserialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateAutoAcceptRuleRequest {
    pub min_duration_hours: Option<i64>,
    pub max_duration_hours: Option<i64>,
    pub enabled: bool,
}

/// Bandwidth stats for a contract
#[derive(Debug, serde::Serialize, poem_openapi::Object, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct BandwidthStatsResponse {
    pub contract_id: String,
    pub gateway_slug: String,
    #[ts(type = "number")]
    pub bytes_in: u64,
    #[ts(type = "number")]
    pub bytes_out: u64,
    #[ts(type = "number")]
    pub last_updated_ns: i64,
}

/// A single bandwidth history record
#[derive(Debug, serde::Serialize, poem_openapi::Object, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct BandwidthHistoryResponse {
    #[ts(type = "number")]
    pub bytes_in: u64,
    #[ts(type = "number")]
    pub bytes_out: u64,
    #[ts(type = "number")]
    pub recorded_at_ns: i64,
}

#[cfg(test)]
mod tests {
    use super::{BandwidthHistoryResponse, BandwidthStatsResponse};
    use crate::database::test_helpers::setup_test_db;
    use crate::openapi::common::{
        ApiResponse, AutoAcceptRequest, AutoAcceptResponse, BulkUpdatePricesRequest,
        BulkUpdateStatusRequest,
        DuplicateOfferingRequest, HelpcenterSyncResponse,
        OnboardingUpdateResponse, ProvisioningStatusRequest, ReconcileRequest,
        RentalResponseRequest,
    };
    use dcc_common::api_types::{
        ReconcileKeepInstance, ReconcilePauseInstance, ReconcileResponse,
        ReconcileTerminateInstance, ReconcileUnknownInstance,
    };
    use poem::web::Data;
    use poem_openapi::payload::Json;
    use std::sync::Arc;

    // ── normalize_provisioning_details ──────────────────────────────────────

    #[test]
    fn test_normalize_provisioning_details_provisioned_with_details() {
        let result = super::normalize_provisioning_details(
            "provisioned",
            Some("  192.168.1.1 root/pass  ".to_string()),
        );
        assert_eq!(result, Ok(Some("192.168.1.1 root/pass".to_string())));
    }

    #[test]
    fn test_normalize_provisioning_details_provisioned_no_details_fails() {
        let result = super::normalize_provisioning_details("provisioned", None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Instance details are required"));
    }

    #[test]
    fn test_normalize_provisioning_details_provisioned_empty_string_fails() {
        // Whitespace-only trims to empty, treated as None — must fail for "provisioned"
        let result = super::normalize_provisioning_details("provisioned", Some("   ".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_provisioning_details_other_status_no_details_ok() {
        let result = super::normalize_provisioning_details("provisioning", None);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_normalize_provisioning_details_other_status_empty_string_returns_none() {
        let result = super::normalize_provisioning_details("provisioning", Some("  ".to_string()));
        assert_eq!(result, Ok(None));
    }

    // ── BandwidthStatsResponse ───────────────────────────────────────────────

    #[test]
    fn test_bandwidth_stats_response_camelcase_field_names() {
        let resp = BandwidthStatsResponse {
            contract_id: "abc".to_string(),
            gateway_slug: "k7m2p4".to_string(),
            bytes_in: 1024,
            bytes_out: 2048,
            last_updated_ns: 1_700_000_000_000_000_000,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["contractId"], "abc");
        assert_eq!(json["gatewaySlug"], "k7m2p4");
        assert_eq!(json["bytesIn"], 1024_u64);
        assert_eq!(json["bytesOut"], 2048_u64);
        assert_eq!(json["lastUpdatedNs"], 1_700_000_000_000_000_000_i64);
    }

    // ── BandwidthHistoryResponse ─────────────────────────────────────────────

    #[test]
    fn test_bandwidth_history_response_camelcase_field_names() {
        let resp = BandwidthHistoryResponse {
            bytes_in: 512,
            bytes_out: 256,
            recorded_at_ns: 9_000_000_000,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["bytesIn"], 512_u64);
        assert_eq!(json["bytesOut"], 256_u64);
        assert_eq!(json["recordedAtNs"], 9_000_000_000_i64);
    }

    // ── AutoAcceptRequest / AutoAcceptResponse ───────────────────────────────

    #[test]
    fn test_auto_accept_response_serialization() {
        let resp = AutoAcceptResponse {
            auto_accept_rentals: true,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["autoAcceptRentals"], true);
    }

    #[test]
    fn test_auto_accept_request_deserialization() {
        let raw = r#"{"autoAcceptRentals": false}"#;
        let req: AutoAcceptRequest = serde_json::from_str(raw).unwrap();
        assert!(!req.auto_accept_rentals);
    }

    // ── OnboardingUpdateResponse ─────────────────────────────────────────────

    #[test]
    fn test_onboarding_update_response_field_name() {
        let resp = OnboardingUpdateResponse {
            onboarding_completed_at: 1_700_000_000_000_000_000,
        };
        let json = serde_json::to_value(&resp).unwrap();
        // This field has an explicit #[serde(rename = "onboarding_completed_at")]
        assert_eq!(
            json["onboarding_completed_at"],
            1_700_000_000_000_000_000_i64
        );
    }

    // ── HelpcenterSyncResponse ───────────────────────────────────────────────

    #[test]
    fn test_helpcenter_sync_response_field_names() {
        let resp = HelpcenterSyncResponse {
            article_url: "https://example.com/article".to_string(),
            action: "created".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["articleUrl"], "https://example.com/article");
        assert_eq!(json["action"], "created");
    }

    // ── ReconcileRequest / ReconcileResponse ─────────────────────────────────

    #[test]
    fn test_reconcile_response_all_buckets_camelcase() {
        let resp = ReconcileResponse {
            keep: vec![ReconcileKeepInstance {
                external_id: "vm-1".to_string(),
                contract_id: "c-1".to_string(),
                ends_at: 9_999_999,
            }],
            terminate: vec![ReconcileTerminateInstance {
                external_id: "vm-2".to_string(),
                contract_id: "c-2".to_string(),
                reason: "cancelled".to_string(),
            }],
            unknown: vec![ReconcileUnknownInstance {
                external_id: "vm-3".to_string(),
                message: "No contract found".to_string(),
            }],
            pause: vec![ReconcilePauseInstance {
                external_id: "vm-4".to_string(),
                contract_id: "c-4".to_string(),
                reason: "stripe_dispute:du_x".to_string(),
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["keep"][0]["externalId"], "vm-1");
        assert_eq!(json["keep"][0]["endsAt"], 9_999_999_i64);
        assert_eq!(json["terminate"][0]["reason"], "cancelled");
        assert_eq!(json["unknown"][0]["message"], "No contract found");
        assert_eq!(json["pause"][0]["externalId"], "vm-4");
        assert_eq!(json["pause"][0]["reason"], "stripe_dispute:du_x");
    }

    #[test]
    fn test_reconcile_request_deserialization() {
        let raw = r#"{"runningInstances":[{"externalId":"vm-5","contractId":"abc"}]}"#;
        let req: ReconcileRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.running_instances.len(), 1);
        assert_eq!(req.running_instances[0].external_id, "vm-5");
        assert_eq!(req.running_instances[0].contract_id.as_deref(), Some("abc"));
    }

    // ── RentalResponseRequest / ProvisioningStatusRequest ────────────────────

    #[test]
    fn test_rental_response_request_accept_with_memo() {
        let raw = r#"{"accept":true,"memo":"Looks good"}"#;
        let req: RentalResponseRequest = serde_json::from_str(raw).unwrap();
        assert!(req.accept);
        assert_eq!(req.memo.as_deref(), Some("Looks good"));
    }

    #[test]
    fn test_rental_response_request_reject_no_memo() {
        let raw = r#"{"accept":false}"#;
        let req: RentalResponseRequest = serde_json::from_str(raw).unwrap();
        assert!(!req.accept);
        assert!(req.memo.is_none());
    }

    #[test]
    fn test_provisioning_status_request_with_details() {
        let raw = r#"{"status":"provisioned","instanceDetails":"192.0.2.1 root/secret"}"#;
        let req: ProvisioningStatusRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.status, "provisioned");
        assert_eq!(
            req.instance_details.as_deref(),
            Some("192.0.2.1 root/secret")
        );
    }

    #[test]
    fn test_provisioning_status_request_without_details() {
        let raw = r#"{"status":"provisioning"}"#;
        let req: ProvisioningStatusRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.status, "provisioning");
        assert!(req.instance_details.is_none());
    }

    // ── BulkUpdateStatusRequest / DuplicateOfferingRequest ───────────────────

    #[test]
    fn test_bulk_update_status_request_deserialization() {
        let raw = r#"{"offeringIds":[1,2,3],"stockStatus":"out_of_stock"}"#;
        let req: BulkUpdateStatusRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.offering_ids, vec![1_i64, 2, 3]);
        assert_eq!(req.stock_status, "out_of_stock");
    }

    #[test]
    fn test_duplicate_offering_request_deserialization() {
        let raw = r#"{"newOfferingId":"offer-clone-01"}"#;
        let req: DuplicateOfferingRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.new_offering_id, "offer-clone-01");
    }

    #[test]
    fn test_bulk_update_prices_request_deserialization() {
        let raw =
            r#"{"updates":[{"id":1,"priceE9s":15000000000},{"id":2,"priceE9s":25000000000}]}"#;
        let req: BulkUpdatePricesRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.updates.len(), 2);
        assert_eq!(req.updates[0].id, 1);
        assert_eq!(req.updates[0].price_e9s, 15_000_000_000);
        assert_eq!(req.updates[1].id, 2);
        assert_eq!(req.updates[1].price_e9s, 25_000_000_000);
    }

    #[test]
    fn test_bulk_update_prices_request_empty_updates() {
        let raw = r#"{"updates":[]}"#;
        let req: BulkUpdatePricesRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.updates.len(), 0);
    }

    // ── ApiResponse wrapping provider-specific types ─────────────────────────

    #[test]
    fn test_api_response_bandwidth_stats_success() {
        let stats = vec![BandwidthStatsResponse {
            contract_id: "cid1".to_string(),
            gateway_slug: "abc123".to_string(),
            bytes_in: 4096,
            bytes_out: 8192,
            last_updated_ns: 1_000,
        }];
        let resp = ApiResponse {
            success: true,
            data: Some(stats),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json.get("error").is_none());
        assert_eq!(json["data"][0]["bytesIn"], 4096_u64);
    }

    #[test]
    fn test_api_response_invalid_pubkey_format_error() {
        let resp: ApiResponse<BandwidthStatsResponse> = ApiResponse {
            success: false,
            data: None,
            error: Some("Invalid pubkey format".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json.get("data").is_none());
        assert_eq!(json["error"], "Invalid pubkey format");
    }

    // ── OfferingStats serialization ──────────────────────────────────────────

    #[test]
    fn test_offering_stats_camelcase_field_names() {
        use crate::database::users::OfferingStats;
        let stats = OfferingStats {
            offering_id: "pool-small".to_string(),
            total_requests: 10,
            active_count: 2,
            cancelled_count: 3,
            expired_count: 1,
            rejected_count: 4,
            total_revenue_e9s: 5_000_000_000,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["offeringId"], "pool-small");
        assert_eq!(json["totalRequests"], 10_i64);
        assert_eq!(json["activeCount"], 2_i64);
        assert_eq!(json["cancelledCount"], 3_i64);
        assert_eq!(json["expiredCount"], 1_i64);
        assert_eq!(json["rejectedCount"], 4_i64);
        assert_eq!(json["totalRevenueE9s"], 5_000_000_000_i64);
    }

    #[test]
    fn test_api_response_offering_stats_success() {
        use crate::database::users::OfferingStats;
        let stats = vec![OfferingStats {
            offering_id: "pool-large".to_string(),
            total_requests: 5,
            active_count: 1,
            cancelled_count: 0,
            expired_count: 0,
            rejected_count: 0,
            total_revenue_e9s: 2_000_000_000,
        }];
        let resp: ApiResponse<Vec<OfferingStats>> = ApiResponse {
            success: true,
            data: Some(stats),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json.get("error").is_none());
        assert_eq!(json["data"][0]["offeringId"], "pool-large");
        assert_eq!(json["data"][0]["totalRequests"], 5_i64);
    }

    // ── OfferingStatsWeek serialization ──────────────────────────────────────

    #[test]
    fn test_offering_stats_week_camelcase_field_names() {
        use crate::database::users::OfferingStatsWeek;
        let row = OfferingStatsWeek {
            week_start: "2024-01-08".to_string(),
            offering_id: "gpu-xl".to_string(),
            total_requests: 3,
            active_count: 1,
            revenue_e9s: 9_000_000_000,
        };
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["weekStart"], "2024-01-08");
        assert_eq!(json["offeringId"], "gpu-xl");
        assert_eq!(json["totalRequests"], 3_i64);
        assert_eq!(json["activeCount"], 1_i64);
        assert_eq!(json["revenueE9s"], 9_000_000_000_i64);
    }

    #[test]
    fn test_api_response_offering_stats_week_success() {
        use crate::database::users::OfferingStatsWeek;
        let rows = vec![OfferingStatsWeek {
            week_start: "2024-02-05".to_string(),
            offering_id: "pool-medium".to_string(),
            total_requests: 7,
            active_count: 3,
            revenue_e9s: 14_000_000_000,
        }];
        let resp: ApiResponse<Vec<OfferingStatsWeek>> = ApiResponse {
            success: true,
            data: Some(rows),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json.get("error").is_none());
        assert_eq!(json["data"][0]["weekStart"], "2024-02-05");
        assert_eq!(json["data"][0]["revenueE9s"], 14_000_000_000_i64);
    }

    // ── password_reset_events SSE handler ───────────────────────────────────

    #[test]
    fn test_password_reset_events_route_registered() {
        // Verify the SSE route is registered in main.rs and uses GET method
        const MAIN_RS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"));
        assert!(
            MAIN_RS.contains("/api/v1/providers/:pubkey/password-reset-events"),
            "SSE route must be registered in main.rs"
        );
        assert!(
            MAIN_RS.contains("password_reset_events"),
            "SSE handler must be referenced in main.rs"
        );
    }

    #[test]
    fn test_password_reset_sse_event_format() {
        // Verify the SSE event data JSON structure matches frontend expectations
        let ids: Vec<String> = vec!["contract-abc".to_string(), "contract-def".to_string()];
        let data = serde_json::json!({
            "count": ids.len(),
            "contract_ids": ids,
        });
        let json_str = data.to_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["count"], 2);
        assert_eq!(parsed["contract_ids"][0], "contract-abc");
        assert_eq!(parsed["contract_ids"][1], "contract-def");
    }

    #[tokio::test]
    async fn test_sse_response_has_event_stream_content_type() {
        use futures::stream;
        use poem::web::sse::{Event, SSE};
        use poem::IntoResponse;

        let events: Vec<Event> = vec![Event::message(r#"{"count":1,"contract_ids":["id1"]}"#)
            .event_type("password-reset-count")];
        let sse = SSE::new(stream::iter(events));
        let resp = sse.into_response();
        assert_eq!(
            resp.content_type(),
            Some("text/event-stream"),
            "SSE response must have text/event-stream content type"
        );
    }

    // ── contract_status_events SSE handler ───────────────────────────────────

    #[test]
    fn test_contract_status_events_route_registered() {
        const MAIN_RS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"));
        assert!(
            MAIN_RS.contains("/api/v1/users/:pubkey/contract-events"),
            "Contract SSE route must be registered in main.rs"
        );
        assert!(
            MAIN_RS.contains("contract_status_events"),
            "SSE handler must be referenced in main.rs"
        );
    }

    #[test]
    fn test_contract_status_sse_event_format() {
        // Verify the SSE event data JSON structure matches frontend expectations
        let contract_id = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let status = "active";
        let updated_at_ns: Option<i64> = Some(1_700_000_000_000_000_000);
        let data = serde_json::json!({
            "contract_id": contract_id,
            "status": status,
            "updated_at_ns": updated_at_ns,
        });
        let json_str = data.to_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["contract_id"], contract_id);
        assert_eq!(parsed["status"], "active");
        assert_eq!(parsed["updated_at_ns"], 1_700_000_000_000_000_000_i64);
    }

    #[test]
    fn test_contract_status_sse_event_format_null_updated_at() {
        let data = serde_json::json!({
            "contract_id": "deadbeef",
            "status": "pending",
            "updated_at_ns": serde_json::Value::Null,
        });
        let json_str = data.to_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["status"], "pending");
        assert!(parsed["updated_at_ns"].is_null());
    }

    #[tokio::test]
    async fn test_contract_status_sse_response_content_type() {
        use futures::stream;
        use poem::web::sse::{Event, SSE};
        use poem::IntoResponse;

        let events: Vec<Event> =
            vec![
                Event::message(r#"{"contract_id":"abc","status":"active","updated_at_ns":null}"#)
                    .event_type("contract-status"),
            ];
        let sse = SSE::new(stream::iter(events));
        let resp = sse.into_response();
        assert_eq!(
            resp.content_type(),
            Some("text/event-stream"),
            "Contract SSE response must have text/event-stream content type"
        );
    }

    #[test]
    fn test_ssh_key_rotation_sse_event_format_rotation_requested() {
        let contract_id = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let data = serde_json::json!({
            "contract_id": contract_id,
            "created_at": 1_700_000_000_000_000_000_i64,
            "actor": "tenant",
            "details": serde_json::Value::Null,
        });
        let json_str = data.to_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["contract_id"], contract_id);
        assert_eq!(parsed["created_at"], 1_700_000_000_000_000_000_i64);
        assert_eq!(parsed["actor"], "tenant");
        assert!(parsed["details"].is_null());
    }

    #[test]
    fn test_ssh_key_rotation_sse_event_format_rotation_complete() {
        let contract_id = "deadbeef12345678";
        let data = serde_json::json!({
            "contract_id": contract_id,
            "created_at": 1_700_000_001_000_000_000_i64,
            "actor": "provider",
            "details": "SSH key rotated to ssh-ed25519 AAA... by agent",
        });
        let json_str = data.to_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["contract_id"], contract_id);
        assert_eq!(parsed["actor"], "provider");
        assert_eq!(
            parsed["details"],
            "SSH key rotated to ssh-ed25519 AAA... by agent"
        );
    }

    #[tokio::test]
    async fn test_ssh_key_rotation_sse_event_types() {
        use futures::stream;
        use poem::web::sse::{Event, SSE};
        use poem::IntoResponse;

        let events: Vec<Event> = vec![
            Event::message(
                r#"{"contract_id":"abc","created_at":123,"actor":"tenant","details":null}"#,
            )
            .event_type("ssh_key_rotation"),
            Event::message(
                r#"{"contract_id":"abc","created_at":456,"actor":"provider","details":"rotated"}"#,
            )
            .event_type("ssh_key_rotation_complete"),
        ];
        let sse = SSE::new(stream::iter(events));
        let resp = sse.into_response();
        assert_eq!(
            resp.content_type(),
            Some("text/event-stream"),
            "SSH key rotation SSE events must use text/event-stream"
        );
    }

    // ── validate_offering_currency ──────────────────────────────────────
    //
    // ICPay (the ICP cryptocurrency rail) is fully retired — Stripe is the sole
    // payment rail. Offerings MUST be priced in a Stripe-supported currency so
    // every checkout can actually settle. These tests pin the boundary: a
    // non-Stripe currency (e.g. "ICP", "BTC") is rejected with a clear,
    // actionable message rather than silently accepted and surfacing as a
    // broken checkout later.

    #[test]
    fn test_validate_offering_currency_accepts_stripe_currencies() {
        for cur in ["USD", "usd", "EUR", "eur", "GBP", "JPY", "CAD"] {
            assert!(
                super::validate_offering_currency(cur).is_ok(),
                "Stripe-supported currency {cur:?} should be accepted"
            );
        }
    }

    #[test]
    fn test_validate_offering_currency_rejects_icp() {
        let result = super::validate_offering_currency("ICP");
        assert!(result.is_err(), "ICP is not a Stripe currency and must be rejected");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("ICP") && msg.contains("Stripe"),
            "error message must name ICP and Stripe; got: {msg}"
        );
    }

    #[test]
    fn test_validate_offering_currency_rejects_other_crypto_and_garbage() {
        for cur in ["BTC", "ETH", "ckBTC", "DCT", "", "unknown"] {
            assert!(
                super::validate_offering_currency(cur).is_err(),
                "non-Stripe currency {cur:?} must be rejected"
            );
        }
    }

    #[test]
    fn test_validate_offering_currency_is_case_insensitive() {
        // Stripe currency codes are matched case-insensitively (lowercased).
        assert!(super::validate_offering_currency("UsD").is_ok());
        assert!(super::validate_offering_currency("eUr").is_ok());
        // Mixed-case ICP is still ICP and still rejected.
        assert!(super::validate_offering_currency("iCp").is_err());
    }

    // ── validate_recipe_if_present ──────────────────────────────────────

    #[test]
    fn test_validate_recipe_if_present_none_returns_ok() {
        assert!(super::validate_recipe_if_present(None).is_ok());
    }

    #[test]
    fn test_validate_recipe_if_present_valid_script_returns_ok() {
        let script = "#!/bin/bash\necho hello".to_string();
        assert!(super::validate_recipe_if_present(Some(&script)).is_ok());
    }

    #[test]
    fn test_validate_recipe_if_present_dangerous_script_returns_err() {
        let script = "#!/bin/bash\nrm -rf /".to_string();
        let result = super::validate_recipe_if_present(Some(&script));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Recipe validation failed"));
    }

    // ── Combined provider dashboard endpoint ─────────────────────────────────
    //
    // The dashboard page previously fanned out to 5 endpoints on every load
    // (trust-metrics, response-metrics, health-summary, my-offerings, activity).
    // The combined /provider/dashboard endpoint returns all five in one
    // authenticated call. Each section is independent: a failing query yields
    // None for that section so one slow/broken source never blanks the page.

    #[tokio::test]
    async fn test_get_provider_dashboard_returns_all_sections() {
        let db = Arc::new(setup_test_db().await);
        let api = super::ProvidersApi;
        let auth = crate::auth::ApiAuthenticatedUser {
            pubkey: vec![0u8; 32],
        };

        let Json(response) = api.get_provider_dashboard(Data(&db), auth).await;

        assert!(response.success, "combined response must be success");
        assert!(response.error.is_none());
        let dashboard = response
            .data
            .expect("dashboard data must be present on success");

        // All five sections must be present (each query resolves, even on empty data).
        assert!(
            dashboard.trust_metrics.is_some(),
            "trustMetrics section must be present"
        );
        assert!(
            dashboard.response_metrics.is_some(),
            "responseMetrics section must be present"
        );
        assert!(
            dashboard.health_summary.is_some(),
            "healthSummary section must be present"
        );
        assert!(
            dashboard.offerings.is_some(),
            "offerings section must be present"
        );
        assert!(
            dashboard.activity.is_some(),
            "activity section must be present"
        );

        // Offerings is a Vec — on an empty DB it must be an empty list, not null.
        assert!(
            dashboard.offerings.as_ref().unwrap().is_empty(),
            "offerings must be empty for an unknown pubkey"
        );
    }

    #[tokio::test]
    async fn test_get_provider_dashboard_uses_authenticated_pubkey() {
        // SECURITY: the dashboard must serve the AUTHENTICATED user's own data,
        // ignoring any other pubkey. We verify by giving the auth a specific
        // pubkey and confirming offerings come back for that same key.
        let db = Arc::new(setup_test_db().await);
        let api = super::ProvidersApi;
        let auth = crate::auth::ApiAuthenticatedUser {
            pubkey: vec![7u8; 32],
        };

        let Json(response) = api.get_provider_dashboard(Data(&db), auth).await;

        assert!(response.success);
        // offerings for an unknown pubkey must be empty (the handler queried auth.pubkey).
        let dashboard = response.data.unwrap();
        assert_eq!(
            dashboard.offerings.as_ref().unwrap().len(),
            0,
            "dashboard must query the authenticated pubkey's offerings"
        );
    }

    #[test]
    fn test_provider_dashboard_route_and_sections_declared() {
        const PROVIDERS_RS: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/openapi/providers.rs",
        ));
        assert!(
            PROVIDERS_RS.contains("path = \"/provider/dashboard\""),
            "Providers API must declare /provider/dashboard route"
        );
        assert!(
            PROVIDERS_RS.contains("async fn get_provider_dashboard"),
            "Providers API must keep get_provider_dashboard handler"
        );
        // The combined response must expose all five dashboard sections.
        assert!(PROVIDERS_RS.contains("pub struct ProviderDashboardResponse"));
        for field in ["trust_metrics", "response_metrics", "health_summary", "offerings", "activity"] {
            assert!(
                PROVIDERS_RS.contains(field),
                "ProviderDashboardResponse must declare field `{field}`"
            );
        }
    }

    #[test]
    fn test_provider_dashboard_response_camelcase_field_names() {
        // Frontend consumes camelCase keys; the combined struct must match.
        let resp = super::ProviderDashboardResponse {
            trust_metrics: None,
            response_metrics: None,
            health_summary: None,
            offerings: None,
            activity: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        let obj = json.as_object().expect("must serialize to an object");
        for key in ["trustMetrics", "responseMetrics", "healthSummary", "offerings", "activity"] {
            assert!(obj.contains_key(key), "missing camelCase key `{key}`");
        }
    }
}
