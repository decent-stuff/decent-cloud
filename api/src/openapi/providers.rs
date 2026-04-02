use super::common::{
    check_authorization, decode_pubkey, default_limit, default_weeks, AddAccountContactRequest,
    AllowlistAddRequest, ApiResponse, ApiTags, AutoAcceptRequest, AutoAcceptResponse,
    BulkUpdatePricesRequest, BulkUpdateStatusRequest, CreatePoolRequest, CreateSetupTokenRequest,
    CsvImportError, CsvImportResult, DuplicateOfferingRequest, EmptyResponse,
    GenerateOfferingsRequest, GenerateOfferingsResponse, HelpcenterSyncResponse, LockResponse,
    NotificationConfigResponse, NotificationUsageResponse, OfferingSuggestionsResponse,
    OnboardingUpdateResponse, PoolUpgradeRequest, ProvisioningStatusRequest, ReconcileKeepInstance,
    ReconcileRequest, ReconcileResponse, ReconcileTerminateInstance, ReconcileUnknownInstance,
    RentalResponseRequest, ResponseMetricsResponse, ResponseTimeDistributionResponse,
    TestNotificationRequest, TestNotificationResponse, UpdateNotificationConfigRequest,
    UpdatePasswordRequest, UpdatePoolRequest, UpdateSlaUptimeConfigRequest,
};
use crate::auth::{AgentAuthenticatedUser, ApiAuthenticatedUser, ProviderOrAgentAuth};
use crate::database::{AgentPoolWithStats, Database, SetupToken};
use dcc_common::ssh_exec::validate_recipe;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

fn validate_recipe_if_present(script: Option<&String>) -> Result<(), String> {
    if let Some(script) = script {
        let result = validate_recipe(script);
        if !result.valid {
            let errors: Vec<String> = result
                .issues
                .into_iter()
                .filter(|i| matches!(i.severity, dcc_common::ssh_exec::RecipeValidationSeverity::Error))
                .map(|i| i.message)
                .collect();
            return Err(format!("Recipe validation failed: {}", errors.join("; ")));
        }
    }
    Ok(())
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

    let pubkey_bytes = hex::decode(&pubkey)
        .map_err(|_| poem::Error::from_string("Invalid pubkey format", StatusCode::BAD_REQUEST))?;

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
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
/// Event format:
///   event: contract-status
///   data: {"contract_id":"<id>","status":"<status>","updated_at_ns":<ns>}
#[poem::handler]
pub async fn contract_status_events(
    req: &poem::Request,
    db: Data<&Arc<Database>>,
    poem::web::Path(pubkey): poem::web::Path<String>,
) -> poem::Result<poem::web::sse::SSE> {
    use futures::StreamExt;
    use poem::http::StatusCode;
    use poem::web::sse::{Event, SSE};

    let pubkey_bytes = hex::decode(&pubkey)
        .map_err(|_| poem::Error::from_string("Invalid pubkey format", StatusCode::BAD_REQUEST))?;

    let auth_pubkey = crate::auth::authenticate_user_from_request(req)
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::UNAUTHORIZED))?;

    if auth_pubkey != pubkey_bytes {
        return Err(poem::Error::from_string(
            "Unauthorized: can only access your own contract events",
            StatusCode::FORBIDDEN,
        ));
    }

    // Snapshot: contract_id -> (status, updated_at_ns)
    type Snapshot = std::collections::HashMap<String, (String, Option<i64>)>;

    let db_clone: Arc<Database> = Arc::clone(&db);
    // Close after 5 minutes: 60 polls × 5 seconds
    let stream =
        futures::stream::unfold(
            (db_clone, pubkey_bytes, None::<Snapshot>, 0u32),
            |(db, pk, prev_snapshot, poll_count): (
                Arc<Database>,
                Vec<u8>,
                Option<Snapshot>,
                u32,
            )| async move {
                if poll_count >= 60 {
                    return None;
                }
                let contracts = match db.get_user_contracts(&pk).await {
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

                let events: Vec<Event> = match &prev_snapshot {
                    None => {
                        // First poll: emit all contracts
                        contracts
                            .iter()
                            .map(|c| {
                                let data = serde_json::json!({
                                    "contract_id": c.contract_id,
                                    "status": c.status,
                                    "updated_at_ns": c.status_updated_at_ns,
                                });
                                Event::message(data.to_string()).event_type("contract-status")
                            })
                            .collect()
                    }
                    Some(prev) => {
                        // Subsequent polls: emit only changed contracts
                        contracts
                            .iter()
                            .filter(|c| {
                                prev.get(&c.contract_id)
                                    .map(|(ps, pt)| {
                                        ps != &c.status || pt != &c.status_updated_at_ns
                                    })
                                    .unwrap_or(true) // new contract
                            })
                            .map(|c| {
                                let data = serde_json::json!({
                                    "contract_id": c.contract_id,
                                    "status": c.status,
                                    "updated_at_ns": c.status_updated_at_ns,
                                });
                                Event::message(data.to_string()).event_type("contract-status")
                            })
                            .collect()
                    }
                };

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                Some((events, (db, pk, Some(current), poll_count + 1)))
            },
        )
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

/// Validate Hetzner offering config against the live Hetzner catalog.
/// No-op for non-Hetzner offerings.
async fn validate_hetzner_offering(
    db: &Database,
    offering: &crate::database::offerings::Offering,
    pubkey_bytes: &[u8],
) -> Result<(), String> {
    if offering.provisioner_type.as_deref() != Some("hetzner") {
        return Ok(());
    }

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

pub struct ProvidersApi;

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
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid pubkey hex: {} (value: {})", e, &pubkey.0)),
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
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid pubkey hex: {} (value: {})", e, &pubkey.0)),
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

    /// Get provider stats
    ///
    /// Returns statistics for a specific provider
    #[oai(
        path = "/providers/:pubkey/stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_stats(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ProviderStats>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_stats(&pubkey_bytes).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get monthly revenue breakdown for a provider (last 12 months)
    #[oai(
        path = "/providers/:pubkey/revenue-by-month",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_revenue_by_month(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::RevenueByMonth>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_revenue_by_month(&pubkey_bytes).await {
            Ok(data) => Json(ApiResponse {
                success: true,
                data: Some(data),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get revenue by month: {e:#}")),
            }),
        }
    }

    /// Get provider trust metrics
    ///
    /// Returns trust score and reliability metrics for a specific provider.
    /// Includes red flag detection for concerning patterns.
    #[oai(
        path = "/providers/:pubkey/trust-metrics",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_trust_metrics(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ProviderTrustMetrics>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_trust_metrics(&pubkey_bytes).await {
            Ok(metrics) => Json(ApiResponse {
                success: true,
                data: Some(metrics),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider feedback stats
    ///
    /// Returns aggregated user feedback statistics for a provider.
    /// Shows the percentage of renters who said the service matched its description
    /// and would rent from this provider again.
    #[oai(
        path = "/providers/:pubkey/feedback-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_feedback_stats(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ProviderFeedbackStats>> {
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

        match db.get_provider_feedback_stats(&pubkey_bytes).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get all feedback for a provider's contracts
    ///
    /// Returns individual feedback entries for all of the authenticated provider's contracts.
    /// Only the provider identified by the pubkey path parameter may call this endpoint.
    #[oai(
        path = "/providers/:pubkey/feedback",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_feedback_list(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::ProviderContractFeedback>>> {
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

        match db.get_provider_all_feedback(&pubkey_bytes).await {
            Ok(feedback) => Json(ApiResponse {
                success: true,
                data: Some(feedback),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider health summary
    ///
    /// Returns uptime metrics and health check statistics for a provider.
    /// Aggregates health check data across all contracts for the specified time window.
    /// Default period is last 30 days.
    #[oai(
        path = "/providers/:pubkey/health-summary",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_health_summary(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
        /// Number of days to look back (default: 30)
        #[oai(default)]
        days: poem_openapi::param::Query<Option<i64>>,
    ) -> Json<ApiResponse<crate::database::contracts::ProviderHealthSummary>> {
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

        match db.get_provider_health_summary(&pubkey_bytes, days.0).await {
            Ok(summary) => Json(ApiResponse {
                success: true,
                data: Some(summary),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get per-contract health summary (provider view)
    ///
    /// Returns aggregated uptime metrics for a single contract.
    /// Only the provider who owns the contract can access this endpoint.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/health",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contract_health_summary(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::ContractHealthSummary>> {
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

        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own contracts".to_string()),
            });
        }

        let contract_id_bytes = match hex::decode(&contract_id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Validate contract belongs to this provider
        let contract = match db.get_contract(&contract_id_bytes).await {
            Ok(Some(c)) => c,
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

        if contract.provider_pubkey != hex::encode(&pubkey_bytes) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: contract does not belong to this provider".to_string()),
            });
        }

        match db.get_contract_health_summary(&contract_id_bytes).await {
            Ok(summary) => Json(ApiResponse {
                success: true,
                data: Some(summary),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get per-contract health checks (provider view)
    ///
    /// Returns the last 50 health check records for a single contract.
    /// Only the provider who owns the contract can access this endpoint.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/health-checks",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contract_health_checks(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractHealthCheck>>> {
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

        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own contracts".to_string()),
            });
        }

        let contract_id_bytes = match hex::decode(&contract_id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Validate contract belongs to this provider
        let contract = match db.get_contract(&contract_id_bytes).await {
            Ok(Some(c)) => c,
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

        if contract.provider_pubkey != hex::encode(&pubkey_bytes) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: contract does not belong to this provider".to_string()),
            });
        }

        match db.get_recent_health_checks(&contract_id_bytes, 50).await {
            Ok(checks) => Json(ApiResponse {
                success: true,
                data: Some(checks),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contract request response metrics
    ///
    /// Returns response-time and SLA compliance metrics for contract rental requests.
    /// Measures how quickly a provider accepts or rejects incoming requests.
    /// This endpoint is for contract request handling, not chat message thread replies.
    #[oai(
        path = "/providers/:pubkey/response-metrics",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_response_metrics(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<ResponseMetricsResponse>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_response_metrics(&pubkey_bytes).await {
            Ok(metrics) => Json(ApiResponse {
                success: true,
                data: Some(ResponseMetricsResponse {
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
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
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
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
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
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
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
    /// Requires agent authentication - agent can only access their delegated provider's contracts.
    #[oai(
        path = "/providers/:pubkey/contracts/pending-password-reset",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_pending_password_reset_contracts(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
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
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        let contract_id_bytes = match hex::decode(&contract_id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
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

        if let Err(e) = validate_hetzner_offering(&db, &params, &pubkey_bytes).await {
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

        if let Err(e) = validate_hetzner_offering(&db, &params, &pubkey_bytes).await {
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

    /// Export provider offerings as CSV
    ///
    /// Returns all offerings for a provider in CSV format (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/export",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn export_provider_offerings_csv(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> poem_openapi::payload::PlainText<String> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => return poem_openapi::payload::PlainText("Invalid pubkey format".to_string()),
        };

        if check_authorization(&pubkey_bytes, &auth).is_err() {
            return poem_openapi::payload::PlainText("Unauthorized".to_string());
        }

        match db.get_provider_offerings(&pubkey_bytes).await {
            Ok(offerings) => {
                let mut csv_writer = csv::Writer::from_writer(vec![]);

                // Write header
                if let Err(e) = csv_writer.write_record([
                    "offering_id",
                    "offer_name",
                    "description",
                    "product_page_url",
                    "currency",
                    "monthly_price",
                    "setup_fee",
                    "visibility",
                    "product_type",
                    "virtualization_type",
                    "billing_interval",
                    "stock_status",
                    "processor_brand",
                    "processor_amount",
                    "processor_cores",
                    "processor_speed",
                    "processor_name",
                    "memory_error_correction",
                    "memory_type",
                    "memory_amount",
                    "hdd_amount",
                    "total_hdd_capacity",
                    "ssd_amount",
                    "total_ssd_capacity",
                    "unmetered_bandwidth",
                    "uplink_speed",
                    "traffic",
                    "datacenter_country",
                    "datacenter_city",
                    "datacenter_latitude",
                    "datacenter_longitude",
                    "control_panel",
                    "gpu_name",
                    "gpu_count",
                    "gpu_memory_gb",
                    "min_contract_hours",
                    "max_contract_hours",
                    "payment_methods",
                    "features",
                    "operating_systems",
                    "agent_pool_id",
                    "template_name",
                    "provisioner_type",
                    "provisioner_config",
                ]) {
                    return poem_openapi::payload::PlainText(format!(
                        "CSV header write error: {}",
                        e
                    ));
                }

                // Write data rows
                for offering in offerings {
                    if let Err(e) = csv_writer.write_record([
                        &offering.offering_id,
                        &offering.offer_name,
                        &offering.description.unwrap_or_default(),
                        &offering.product_page_url.unwrap_or_default(),
                        &offering.currency,
                        &offering.monthly_price.to_string(),
                        &offering.setup_fee.to_string(),
                        &offering.visibility,
                        &offering.product_type,
                        &offering.virtualization_type.unwrap_or_default(),
                        &offering.billing_interval,
                        &offering.stock_status,
                        &offering.processor_brand.unwrap_or_default(),
                        &offering
                            .processor_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .processor_cores
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.processor_speed.unwrap_or_default(),
                        &offering.processor_name.unwrap_or_default(),
                        &offering.memory_error_correction.unwrap_or_default(),
                        &offering.memory_type.unwrap_or_default(),
                        &offering.memory_amount.unwrap_or_default(),
                        &offering
                            .hdd_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.total_hdd_capacity.unwrap_or_default(),
                        &offering
                            .ssd_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.total_ssd_capacity.unwrap_or_default(),
                        &offering.unmetered_bandwidth.to_string(),
                        &offering.uplink_speed.unwrap_or_default(),
                        &offering.traffic.map(|v| v.to_string()).unwrap_or_default(),
                        &offering.datacenter_country,
                        &offering.datacenter_city,
                        &offering
                            .datacenter_latitude
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .datacenter_longitude
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.control_panel.unwrap_or_default(),
                        &offering.gpu_name.unwrap_or_default(),
                        &offering
                            .gpu_count
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .gpu_memory_gb
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .min_contract_hours
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .max_contract_hours
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.payment_methods.unwrap_or_default(),
                        &offering.features.unwrap_or_default(),
                        &offering.operating_systems.unwrap_or_default(),
                        &offering.agent_pool_id.unwrap_or_default(),
                        &offering.template_name.unwrap_or_default(),
                        &offering.provisioner_type.unwrap_or_default(),
                        &offering.provisioner_config.unwrap_or_default(),
                    ]) {
                        return poem_openapi::payload::PlainText(format!(
                            "CSV row write error for offering {}: {}",
                            offering.offering_id, e
                        ));
                    }
                }

                match csv_writer.into_inner() {
                    Ok(csv_data) => poem_openapi::payload::PlainText(
                        String::from_utf8_lossy(&csv_data).to_string(),
                    ),
                    Err(e) => {
                        poem_openapi::payload::PlainText(format!("CSV generation error: {}", e))
                    }
                }
            }
            Err(e) => poem_openapi::payload::PlainText(format!("Error: {}", e)),
        }
    }

    /// Import provider offerings from CSV
    ///
    /// Imports offerings from CSV format (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/import",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn import_provider_offerings_csv(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        #[oai(default)] upsert: poem_openapi::param::Query<bool>,
        csv_data: poem_openapi::payload::PlainText<String>,
    ) -> Json<ApiResponse<CsvImportResult>> {
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
            .import_offerings_csv(&pubkey_bytes, &csv_data.0, upsert.0)
            .await
        {
            Ok((success_count, mut errors)) => {
                // Post-import: validate Hetzner offerings against live catalog
                if let Ok(offerings) = db.get_provider_offerings(&pubkey_bytes).await {
                    for offering in offerings
                        .iter()
                        .filter(|o| o.provisioner_type.as_deref() == Some("hetzner"))
                    {
                        if let Err(e) =
                            validate_hetzner_offering(&db, offering, &pubkey_bytes).await
                        {
                            errors.push((
                                0,
                                format!(
                                    "Hetzner validation failed for offering '{}': {}",
                                    offering.offering_id, e
                                ),
                            ));
                        }
                    }
                }

                let result = CsvImportResult {
                    success_count,
                    errors: errors
                        .into_iter()
                        .map(|(row, message)| CsvImportError { row, message })
                        .collect(),
                };
                Json(ApiResponse {
                    success: true,
                    data: Some(result),
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

    /// Get offering allowlist
    ///
    /// Returns the visibility allowlist for a shared offering (requires authentication as owner)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/allowlist",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_offering_allowlist(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::visibility_allowlist::AllowlistEntry>>> {
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

        match db.get_allowlist(id.0, &pubkey_bytes).await {
            Ok(entries) => Json(ApiResponse {
                success: true,
                data: Some(entries),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Add to offering allowlist
    ///
    /// Adds a pubkey to the visibility allowlist for a shared offering (requires authentication as owner)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/allowlist",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn add_to_offering_allowlist(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        req: Json<AllowlistAddRequest>,
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

        let allowed_pubkey_bytes = match decode_pubkey(&req.0.allowed_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid allowed_pubkey: {}", e)),
                })
            }
        };

        match db
            .add_to_allowlist(id.0, &allowed_pubkey_bytes, &pubkey_bytes)
            .await
        {
            Ok(entry_id) => Json(ApiResponse {
                success: true,
                data: Some(entry_id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove from offering allowlist
    ///
    /// Removes a pubkey from the visibility allowlist for a shared offering (requires authentication as owner)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/allowlist/:allowed_pubkey",
        method = "delete",
        tag = "ApiTags::Offerings"
    )]
    async fn remove_from_offering_allowlist(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        allowed_pubkey: Path<String>,
    ) -> Json<ApiResponse<bool>> {
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

        let allowed_pubkey_bytes = match decode_pubkey(&allowed_pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid allowed_pubkey: {}", e)),
                })
            }
        };

        match db
            .remove_from_allowlist(id.0, &allowed_pubkey_bytes, &pubkey_bytes)
            .await
        {
            Ok(removed) => Json(ApiResponse {
                success: true,
                data: Some(removed),
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
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
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
            let stripe_client = crate::stripe_client::StripeClient::new().ok();
            let icpay_client = crate::icpay_client::IcpayClient::new().ok();

            match db
                .reject_contract(
                    &contract_id,
                    &auth.pubkey,
                    req.memo.as_deref(),
                    stripe_client.as_ref(),
                    icpay_client.as_ref(),
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
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
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
                        if let Ok(Some(contract)) = db.get_contract(&contract_id).await {
                            if let Err(e) = crate::rental_notifications::notify_user_provisioned(
                                &db,
                                email_service.as_ref(),
                                &contract,
                                details,
                            )
                            .await
                            {
                                // Log but don't fail - provisioning succeeded
                                tracing::warn!(
                                    "Failed to send provisioned notification for contract {}: {}",
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
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
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

    /// Get user notification configuration
    ///
    /// Returns notification preferences for the authenticated user
    #[oai(
        path = "/providers/me/notification-config",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_user_notification_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<NotificationConfigResponse>> {
        match db.get_user_notification_config(&auth.pubkey).await {
            Ok(Some(config)) => Json(ApiResponse {
                success: true,
                data: Some(NotificationConfigResponse {
                    notify_telegram: config.notify_telegram,
                    notify_email: config.notify_email,
                    notify_sms: config.notify_sms,
                    telegram_chat_id: config.telegram_chat_id,
                    notify_phone: config.notify_phone,
                    notify_email_address: config.notify_email_address,
                }),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: true,
                data: Some(NotificationConfigResponse {
                    notify_telegram: false,
                    notify_email: false,
                    notify_sms: false,
                    telegram_chat_id: None,
                    notify_phone: None,
                    notify_email_address: None,
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

    /// Update user notification configuration
    ///
    /// Updates notification preferences for the authenticated user
    #[oai(
        path = "/providers/me/notification-config",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_user_notification_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        req: Json<UpdateNotificationConfigRequest>,
    ) -> Json<ApiResponse<String>> {
        let config = crate::database::notification_config::UserNotificationConfig {
            user_pubkey: auth.pubkey.clone(),
            notify_telegram: req.notify_telegram,
            notify_email: req.notify_email,
            notify_sms: req.notify_sms,
            telegram_chat_id: req.telegram_chat_id.clone(),
            notify_phone: req.notify_phone.clone(),
            notify_email_address: req.notify_email_address.clone(),
        };

        match db.set_user_notification_config(&auth.pubkey, &config).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Notification configuration updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider notification usage
    ///
    /// Returns today's notification usage counts for the authenticated provider
    #[oai(
        path = "/providers/me/notification-usage",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_notification_usage(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<NotificationUsageResponse>> {
        let provider_id = hex::encode(&auth.pubkey);

        let telegram = db.get_notification_usage(&provider_id, "telegram").await;
        let sms = db.get_notification_usage(&provider_id, "sms").await;
        let email = db.get_notification_usage(&provider_id, "email").await;

        match (telegram, sms, email) {
            (Ok(tg), Ok(sm), Ok(em)) => Json(ApiResponse {
                success: true,
                data: Some(NotificationUsageResponse {
                    telegram_count: tg,
                    sms_count: sm,
                    email_count: em,
                    telegram_limit: crate::support_bot::notifications::TELEGRAM_DAILY_LIMIT,
                    sms_limit: crate::support_bot::notifications::SMS_DAILY_LIMIT,
                }),
                error: None,
            }),
            _ => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Failed to fetch usage data".to_string()),
            }),
        }
    }

    /// Test a notification channel
    ///
    /// Sends a test notification to the specified channel to verify configuration.
    /// Channels: "telegram", "email", "sms"
    #[oai(
        path = "/providers/me/notification-test",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn test_notification_channel(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        auth: ApiAuthenticatedUser,
        req: Json<TestNotificationRequest>,
    ) -> Json<ApiResponse<TestNotificationResponse>> {
        use crate::support_bot::test_notifications::send_test_notification;

        match send_test_notification(&db, email_service.as_ref(), &auth.pubkey, &req.channel).await
        {
            Ok(message) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: true,
                    message,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: false,
                    message: format!("{:#}", e), // Full error chain
                }),
                error: None,
            }),
        }
    }

    /// Test the full escalation notification flow
    ///
    /// Creates a mock escalation event and dispatches notifications to all enabled channels.
    /// This tests the complete pipeline from Chatwoot escalation to notification delivery.
    #[oai(
        path = "/providers/me/notification-test/escalation",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn test_escalation_notification(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<TestNotificationResponse>> {
        use crate::support_bot::test_notifications::send_test_escalation;

        match send_test_escalation(&db, email_service.as_ref(), &auth.pubkey).await {
            Ok(message) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: true,
                    message,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: true,
                data: Some(TestNotificationResponse {
                    sent: false,
                    message: format!("{:#}", e), // Full error chain
                }),
                error: None,
            }),
        }
    }

    /// Get provider SLA uptime alert configuration
    ///
    /// Returns the authenticated provider's uptime threshold and alert window settings.
    /// Defaults to 95% threshold and 24-hour window if no config row exists yet.
    #[oai(
        path = "/providers/:pubkey/sla-uptime-config",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_sla_uptime_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::providers::SlaUptimeConfig>> {
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

        match db.get_provider_sla_uptime_config(&pubkey_bytes).await {
            Ok(Some(config)) => Json(ApiResponse {
                success: true,
                data: Some(config),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: true,
                data: Some(crate::database::providers::SlaUptimeConfig {
                    uptime_threshold_percent: 95,
                    sla_alert_window_hours: 24,
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

    /// Update provider SLA uptime alert configuration
    ///
    /// Sets the uptime threshold (1–100%) and alert window (hours) for the authenticated provider.
    /// You'll receive a notification when any contract's uptime drops below the threshold.
    #[oai(
        path = "/providers/:pubkey/sla-uptime-config",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_provider_sla_uptime_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpdateSlaUptimeConfigRequest>,
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

        if req.uptime_threshold_percent < 1 || req.uptime_threshold_percent > 100 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("uptime_threshold_percent must be between 1 and 100".to_string()),
            });
        }
        if req.sla_alert_window_hours < 1 || req.sla_alert_window_hours > 168 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("sla_alert_window_hours must be between 1 and 168".to_string()),
            });
        }

        match db
            .upsert_provider_sla_uptime_config(
                &pubkey_bytes,
                req.uptime_threshold_percent,
                req.sla_alert_window_hours,
            )
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some("SLA uptime configuration updated successfully".to_string()),
                error: None,
            }),
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
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
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
            let contract_id_bytes = match hex::decode(contract_id) {
                Ok(bytes) => bytes,
                Err(_) => {
                    unknown.push(ReconcileUnknownInstance {
                        external_id: instance.external_id.clone(),
                        message: format!("Invalid contract ID format: {}", contract_id),
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
            }),
            error: None,
        })
    }

    // ==================== Agent Pool Endpoints ====================

    /// Create agent pool
    ///
    /// Creates a new agent pool for grouping provisioning agents by location and type.
    #[oai(
        path = "/providers/:pubkey/pools",
        method = "post",
        tag = "ApiTags::Pools"
    )]
    async fn create_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<CreatePoolRequest>,
    ) -> Json<ApiResponse<crate::database::AgentPool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // Generate a unique pool_id from name (sanitized)
        let pool_id = format!(
            "{}-{}",
            req.name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>()
                .to_lowercase(),
            &uuid::Uuid::new_v4().to_string()[..8]
        );

        match db
            .create_agent_pool(
                &pool_id,
                &provider_pubkey,
                &req.name,
                &req.location,
                &req.provisioner_type,
            )
            .await
        {
            Ok(pool) => Json(ApiResponse {
                success: true,
                data: Some(pool),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List agent pools
    ///
    /// Returns all agent pools for a provider with statistics.
    #[oai(
        path = "/providers/:pubkey/pools",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn list_pools(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<AgentPoolWithStats>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.list_agent_pools_with_stats(&provider_pubkey).await {
            Ok(pools) => Json(ApiResponse {
                success: true,
                data: Some(pools),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get agent pool details
    ///
    /// Returns details and statistics for a specific agent pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn get_pool_details(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<AgentPoolWithStats>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // This is not the most efficient way, but list_agent_pools_with_stats is what we have.
        // A dedicated get_pool_with_stats(pool_id) would be better.
        match db.list_agent_pools_with_stats(&provider_pubkey).await {
            Ok(pools) => {
                if let Some(pool) = pools.into_iter().find(|p| p.pool.pool_id == pool_id.0) {
                    Json(ApiResponse {
                        success: true,
                        data: Some(pool),
                        error: None,
                    })
                } else {
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(
                            "Pool not found or does not belong to this provider".to_string(),
                        ),
                    })
                }
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List agents in a pool
    ///
    /// Returns all active agent delegations for a specific pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/agents",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn list_agents_in_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::agent_delegations::AgentDelegation>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // Optional: Check if pool belongs to this provider
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) => {
                let pool_pubkey = match hex::decode(&pool.provider_pubkey) {
                    Ok(pk) => pk,
                    Err(e) => {
                        tracing::warn!("Malformed hex in pool.provider_pubkey: {:#}", e);
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("Invalid pubkey format in database".to_string()),
                        });
                    }
                };
                if pool_pubkey != provider_pubkey {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Pool does not belong to this provider".to_string()),
                    });
                }
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
        }

        match db.list_agents_in_pool(&pool_id.0).await {
            Ok(agents) => Json(ApiResponse {
                success: true,
                data: Some(agents),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get agent pool
    ///
    /// Returns details for a specific agent pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn get_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<crate::database::AgentPool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) => {
                // Verify pool belongs to this provider
                if pool.provider_pubkey != hex::encode(&provider_pubkey) {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Pool not found".to_string()),
                    });
                }
                Json(ApiResponse {
                    success: true,
                    data: Some(pool),
                    error: None,
                })
            }
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Pool not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update agent pool
    ///
    /// Updates an existing agent pool's name, location, or provisioner type.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "put",
        tag = "ApiTags::Pools"
    )]
    async fn update_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        req: Json<UpdatePoolRequest>,
    ) -> Json<ApiResponse<bool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .update_agent_pool(
                &pool_id.0,
                &provider_pubkey,
                req.name.as_deref(),
                req.location.as_deref(),
                req.provisioner_type.as_deref(),
            )
            .await
        {
            Ok(updated) => Json(ApiResponse {
                success: true,
                data: Some(updated),
                error: if updated {
                    None
                } else {
                    Some("No fields to update or pool not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete agent pool
    ///
    /// Deletes an agent pool. Fails if pool has any agents assigned.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "delete",
        tag = "ApiTags::Pools"
    )]
    async fn delete_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<bool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_agent_pool(&pool_id.0, &provider_pubkey).await {
            Ok(deleted) => Json(ApiResponse {
                success: true,
                data: Some(deleted),
                error: if deleted {
                    None
                } else {
                    Some("Pool not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Request agent upgrade for a pool
    ///
    /// Sets the target version for all agents in a pool. Agents pick up
    /// the upgrade directive on their next heartbeat and self-upgrade.
    /// Pass `version: null` to cancel a pending upgrade.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/upgrade",
        method = "post",
        tag = "ApiTags::Pools"
    )]
    async fn request_pool_upgrade(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        req: Json<PoolUpgradeRequest>,
    ) -> Json<ApiResponse<bool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        // Validate version format if provided (semver: X.Y.Z)
        if let Some(ref version) = req.version {
            let v = version.trim().trim_start_matches('v');
            let parts: Vec<&str> = v.split('.').collect();
            let valid = parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok());
            if !valid {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Invalid version format '{}': expected semver like 0.4.21",
                        version
                    )),
                });
            }
        }

        match db
            .set_pool_upgrade_version(&pool_id.0, &provider_pubkey, req.version.as_deref())
            .await
        {
            Ok(updated) => Json(ApiResponse {
                success: true,
                data: Some(updated),
                error: if updated {
                    None
                } else {
                    Some("Pool not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create setup token
    ///
    /// Creates a one-time setup token for agent registration in a pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/setup-tokens",
        method = "post",
        tag = "ApiTags::Pools"
    )]
    async fn create_setup_token(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        req: Json<CreateSetupTokenRequest>,
    ) -> Json<ApiResponse<SetupToken>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
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
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) if pool.provider_pubkey == hex::encode(&provider_pubkey) => {}
            Ok(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }

        let expires_in_hours = req.expires_in_hours.unwrap_or(24);

        match db
            .create_setup_token(&pool_id.0, req.label.as_deref(), expires_in_hours)
            .await
        {
            Ok(token) => Json(ApiResponse {
                success: true,
                data: Some(token),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List setup tokens
    ///
    /// Returns pending (unused, unexpired) setup tokens for a pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/setup-tokens",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn list_setup_tokens(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<Vec<SetupToken>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
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
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) if pool.provider_pubkey == hex::encode(&provider_pubkey) => {}
            Ok(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }

        match db.list_pending_setup_tokens(&pool_id.0).await {
            Ok(tokens) => Json(ApiResponse {
                success: true,
                data: Some(tokens),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete setup token
    ///
    /// Deletes a setup token (e.g., to revoke it before it's used).
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/setup-tokens/:token",
        method = "delete",
        tag = "ApiTags::Pools"
    )]
    async fn delete_setup_token(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        token: Path<String>,
    ) -> Json<ApiResponse<bool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
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
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) if pool.provider_pubkey == hex::encode(&provider_pubkey) => {}
            Ok(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }

        match db.delete_setup_token(&token.0).await {
            Ok(deleted) => Json(ApiResponse {
                success: true,
                data: Some(deleted),
                error: if deleted {
                    None
                } else {
                    Some("Token not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    // ==================== Provisioning Lock Endpoints ====================

    /// Acquire provisioning lock
    ///
    /// Atomically acquires a provisioning lock on a contract.
    /// Returns 200 with acquired=true if lock acquired, acquired=false if already locked.
    /// Requires agent authentication with provision permission.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/lock",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn acquire_lock(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<LockResponse>> {
        use crate::database::AgentPermission;

        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        // Verify agent belongs to this provider
        if provider_pubkey != auth.provider_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Agent is not delegated by this provider".to_string()),
            });
        }

        // Check provision permission
        if let Err(e) = auth.require_permission(AgentPermission::Provision) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Decode contract ID
        let contract_bytes = match hex::decode(&contract_id.0) {
            Ok(b) => b,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid contract_id hex: {}", e)),
                })
            }
        };

        // Lock duration: 5 minutes
        let lock_duration_ns = 5 * 60 * 1_000_000_000i64;
        let expires_at_ns = match crate::now_ns() {
            Ok(ns) => ns + lock_duration_ns,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        match db
            .acquire_provisioning_lock(&contract_bytes, &auth.agent_pubkey, lock_duration_ns)
            .await
        {
            Ok(acquired) => Json(ApiResponse {
                success: true,
                data: Some(LockResponse {
                    acquired,
                    expires_at_ns,
                }),
                error: if acquired {
                    None
                } else {
                    Some("Contract already locked by another agent".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Release provisioning lock
    ///
    /// Releases a provisioning lock held by this agent.
    /// Requires agent authentication.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/lock",
        method = "delete",
        tag = "ApiTags::Contracts"
    )]
    async fn release_lock(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<bool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        // Verify agent belongs to this provider
        if provider_pubkey != auth.provider_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Agent is not delegated by this provider".to_string()),
            });
        }

        // Decode contract ID
        let contract_bytes = match hex::decode(&contract_id.0) {
            Ok(b) => b,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid contract_id hex: {}", e)),
                })
            }
        };

        match db
            .release_provisioning_lock(&contract_bytes, &auth.agent_pubkey)
            .await
        {
            Ok(released) => Json(ApiResponse {
                success: true,
                data: Some(released),
                error: if released {
                    None
                } else {
                    Some("Lock not held by this agent".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
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
                let pool_owner = match hex::decode(&p.provider_pubkey) {
                    Ok(pk) => pk,
                    Err(_) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("Invalid pool owner pubkey".to_string()),
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
                let pool_owner = match hex::decode(&p.provider_pubkey) {
                    Ok(pk) => pk,
                    Err(_) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("Invalid pool owner pubkey".to_string()),
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
                is_example: false,
                is_draft: false,
                publish_at: None,
                offering_source: Some("generated".to_string()),
                external_checkout_url: None,
                reseller_name: None,
                reseller_commission_percent: None,
                owner_username: None,
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

    /// Get per-offering contract statistics for a provider
    ///
    /// Returns aggregated contract counts and revenue broken down by offering.
    /// Requires provider authentication — only the provider can access their own stats.
    #[oai(
        path = "/providers/:pubkey/offering-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_stats(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::OfferingStats>>> {
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

        match db.get_offering_stats(&provider_pubkey).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering stats: {e:#}")),
            }),
        }
    }

    /// Get weekly offering stats history for a provider
    ///
    /// Returns per-offering weekly contract counts and revenue for the last N weeks.
    /// Requires provider authentication — only the provider can access their own stats.
    #[oai(
        path = "/providers/:pubkey/offering-stats-history",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_stats_history(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        #[oai(default = "default_weeks")] weeks: poem_openapi::param::Query<i32>,
    ) -> Json<ApiResponse<Vec<crate::database::users::OfferingStatsWeek>>> {
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

        let weeks = weeks.0.clamp(1, 52);
        match db.get_offering_stats_history(&provider_pubkey, weeks).await {
            Ok(rows) => Json(ApiResponse {
                success: true,
                data: Some(rows),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering stats history: {e:#}")),
            }),
        }
    }

    /// Get per-offering conversion stats for a provider
    ///
    /// Returns views vs rentals breakdown per offering for the last 7 and 30 days.
    /// Requires provider authentication — only the provider can access their own stats.
    #[oai(
        path = "/providers/:pubkey/offering-conversion-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_conversion_stats(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::OfferingConversionStats>>> {
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

        match db.get_offering_conversion_stats(&provider_pubkey).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering conversion stats: {e:#}")),
            }),
        }
    }

    /// Get per-offering tenant satisfaction stats for the authenticated provider
    #[oai(
        path = "/providers/:pubkey/offering-satisfaction-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_satisfaction_stats(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::OfferingSatisfactionStats>>> {
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

        match db.get_offering_satisfaction_stats(&provider_pubkey).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering satisfaction stats: {e:#}")),
            }),
        }
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
        BulkUpdateStatusRequest, CreatePoolRequest, CreateSetupTokenRequest, CsvImportError,
        CsvImportResult, DuplicateOfferingRequest, HelpcenterSyncResponse,
        NotificationConfigResponse, NotificationUsageResponse, OnboardingUpdateResponse,
        ProvisioningStatusRequest, ReconcileRequest, RentalResponseRequest,
        ResponseMetricsResponse, ResponseTimeDistributionResponse, TestNotificationResponse,
        UpdatePoolRequest,
    };
    use dcc_common::api_types::{
        LockResponse, ReconcileKeepInstance, ReconcileResponse, ReconcileTerminateInstance,
        ReconcileUnknownInstance,
    };
    use poem::web::Data;
    use poem_openapi::param::Path;
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

    // ── ResponseMetricsResponse ──────────────────────────────────────────────

    #[test]
    fn test_response_metrics_response_optional_fields_null() {
        let dist = ResponseTimeDistributionResponse {
            within_1h_pct: 50.0,
            within_4h_pct: 70.0,
            within_12h_pct: 85.0,
            within_24h_pct: 90.0,
            within_72h_pct: 95.0,
            total_responses: 42,
        };
        let metrics = ResponseMetricsResponse {
            avg_response_seconds: None,
            avg_response_hours: None,
            sla_compliance_percent: 88.5,
            breach_count_30d: 3,
            total_inquiries_30d: 100,
            distribution: dist,
        };
        let json = serde_json::to_value(&metrics).unwrap();
        assert!(json["avgResponseSeconds"].is_null());
        assert!(json["avgResponseHours"].is_null());
        assert_eq!(json["slaCompliancePercent"], 88.5_f64);
        assert_eq!(json["breachCount30d"], 3_i64);
        assert_eq!(json["distribution"]["within1hPct"], 50.0_f64);
        assert_eq!(json["distribution"]["totalResponses"], 42_i64);
    }

    #[test]
    fn test_response_metrics_response_with_values() {
        let dist = ResponseTimeDistributionResponse {
            within_1h_pct: 0.0,
            within_4h_pct: 0.0,
            within_12h_pct: 0.0,
            within_24h_pct: 0.0,
            within_72h_pct: 0.0,
            total_responses: 0,
        };
        let metrics = ResponseMetricsResponse {
            avg_response_seconds: Some(3600.0),
            avg_response_hours: Some(1.0),
            sla_compliance_percent: 100.0,
            breach_count_30d: 0,
            total_inquiries_30d: 0,
            distribution: dist,
        };
        let json = serde_json::to_value(&metrics).unwrap();
        assert_eq!(json["avgResponseSeconds"], 3600.0_f64);
        assert_eq!(json["avgResponseHours"], 1.0_f64);
    }

    // ── NotificationConfigResponse ───────────────────────────────────────────

    #[test]
    fn test_notification_config_response_optional_fields_absent_when_none() {
        let config = NotificationConfigResponse {
            notify_telegram: false,
            notify_email: true,
            notify_sms: false,
            telegram_chat_id: None,
            notify_phone: None,
            notify_email_address: None,
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["notifyTelegram"], false);
        assert_eq!(json["notifyEmail"], true);
        // None fields serialise as null through serde (skip_serializing_if is poem-specific)
        assert!(
            json.get("telegramChatId").is_none_or(|v| v.is_null()),
            "telegramChatId should be absent or null"
        );
    }

    #[test]
    fn test_notification_config_response_with_all_fields() {
        let config = NotificationConfigResponse {
            notify_telegram: true,
            notify_email: true,
            notify_sms: true,
            telegram_chat_id: Some("123456789".to_string()),
            notify_phone: Some("+1555000".to_string()),
            notify_email_address: Some("a@b.com".to_string()),
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["telegramChatId"], "123456789");
        assert_eq!(json["notifyPhone"], "+1555000");
        assert_eq!(json["notifyEmailAddress"], "a@b.com");
    }

    // ── NotificationUsageResponse ────────────────────────────────────────────

    #[test]
    fn test_notification_usage_response_field_names() {
        let usage = NotificationUsageResponse {
            telegram_count: 5,
            sms_count: 2,
            email_count: 10,
            telegram_limit: 50,
            sms_limit: 10,
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(json["telegramCount"], 5_i64);
        assert_eq!(json["smsCount"], 2_i64);
        assert_eq!(json["emailCount"], 10_i64);
        assert_eq!(json["telegramLimit"], 50_i64);
        assert_eq!(json["smsLimit"], 10_i64);
    }

    // ── TestNotificationResponse ─────────────────────────────────────────────

    #[test]
    fn test_notification_test_response_sent_true() {
        let resp = TestNotificationResponse {
            sent: true,
            message: "Telegram message delivered".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["sent"], true);
        assert_eq!(json["message"], "Telegram message delivered");
    }

    #[test]
    fn test_notification_test_response_sent_false() {
        let resp = TestNotificationResponse {
            sent: false,
            message: "Bot token not configured".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["sent"], false);
        assert!(!json["message"].as_str().unwrap().is_empty());
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

    // ── CsvImportResult / CsvImportError ────────────────────────────────────

    #[test]
    fn test_csv_import_result_with_errors() {
        let result = CsvImportResult {
            success_count: 3,
            errors: vec![
                CsvImportError {
                    row: 2,
                    message: "Missing required field".to_string(),
                },
                CsvImportError {
                    row: 5,
                    message: "Invalid price".to_string(),
                },
            ],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["successCount"], 3_i64);
        let errors = json["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0]["row"], 2_i64);
        assert_eq!(errors[0]["message"], "Missing required field");
    }

    #[test]
    fn test_csv_import_result_no_errors() {
        let result = CsvImportResult {
            success_count: 10,
            errors: vec![],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["successCount"], 10_i64);
        assert_eq!(json["errors"].as_array().unwrap().len(), 0);
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
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["keep"][0]["externalId"], "vm-1");
        assert_eq!(json["keep"][0]["endsAt"], 9_999_999_i64);
        assert_eq!(json["terminate"][0]["reason"], "cancelled");
        assert_eq!(json["unknown"][0]["message"], "No contract found");
    }

    #[test]
    fn test_reconcile_request_deserialization() {
        let raw = r#"{"runningInstances":[{"externalId":"vm-5","contractId":"abc"}]}"#;
        let req: ReconcileRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.running_instances.len(), 1);
        assert_eq!(req.running_instances[0].external_id, "vm-5");
        assert_eq!(req.running_instances[0].contract_id.as_deref(), Some("abc"));
    }

    // ── CreatePoolRequest / UpdatePoolRequest / CreateSetupTokenRequest ───────

    #[test]
    fn test_create_pool_request_deserialization() {
        let raw = r#"{"name":"eu-proxmox","location":"europe","provisionerType":"proxmox"}"#;
        let req: CreatePoolRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.name, "eu-proxmox");
        assert_eq!(req.location, "europe");
        assert_eq!(req.provisioner_type, "proxmox");
    }

    #[test]
    fn test_update_pool_request_all_optional_none() {
        let raw = r#"{}"#;
        let req: UpdatePoolRequest = serde_json::from_str(raw).unwrap();
        assert!(req.name.is_none());
        assert!(req.location.is_none());
        assert!(req.provisioner_type.is_none());
    }

    #[test]
    fn test_create_setup_token_request_defaults() {
        let raw = r#"{}"#;
        let req: CreateSetupTokenRequest = serde_json::from_str(raw).unwrap();
        assert!(req.label.is_none());
        assert!(req.expires_in_hours.is_none());
    }

    #[test]
    fn test_create_setup_token_request_with_values() {
        let raw = r#"{"label":"worker-01","expiresInHours":48}"#;
        let req: CreateSetupTokenRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.label.as_deref(), Some("worker-01"));
        assert_eq!(req.expires_in_hours, Some(48));
    }

    // ── LockResponse ─────────────────────────────────────────────────────────

    #[test]
    fn test_lock_response_acquired_camelcase() {
        let resp = LockResponse {
            acquired: true,
            expires_at_ns: 1_700_000_300_000_000_000,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["acquired"], true);
        assert_eq!(json["expiresAtNs"], 1_700_000_300_000_000_000_i64);
    }

    #[test]
    fn test_lock_response_not_acquired() {
        let resp = LockResponse {
            acquired: false,
            expires_at_ns: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["acquired"], false);
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

    #[tokio::test]
    async fn test_get_provider_response_metrics_success_with_empty_dataset() {
        let db = Arc::new(setup_test_db().await);
        let api = super::ProvidersApi;
        let pubkey = "0".repeat(64);

        let Json(response) = api
            .get_provider_response_metrics(Data(&db), Path(pubkey))
            .await;

        assert!(response.success);
        assert!(response.error.is_none());

        let metrics = response.data.expect("response data should be present");
        assert!(metrics.avg_response_seconds.is_none());
        assert!(metrics.avg_response_hours.is_none());
        assert_eq!(metrics.sla_compliance_percent, 100.0);
        assert_eq!(metrics.breach_count_30d, 0);
        assert_eq!(metrics.total_inquiries_30d, 0);
        assert_eq!(metrics.distribution.total_responses, 0);
    }

    #[tokio::test]
    async fn test_get_provider_response_metrics_invalid_pubkey() {
        let db = Arc::new(setup_test_db().await);
        let api = super::ProvidersApi;

        let Json(response) = api
            .get_provider_response_metrics(Data(&db), Path("invalid-pubkey".to_string()))
            .await;

        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error.as_deref(), Some("Invalid pubkey format"));
    }

    #[test]
    fn test_provider_response_metrics_route_is_declared() {
        const PROVIDERS_RS: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/openapi/providers.rs"
        ));
        assert!(
            PROVIDERS_RS.contains("path = \"/providers/:pubkey/response-metrics\""),
            "Providers API must declare /providers/:pubkey/response-metrics route"
        );
        assert!(
            PROVIDERS_RS.contains("async fn get_provider_response_metrics"),
            "Providers API must keep get_provider_response_metrics handler"
        );
    }
}
