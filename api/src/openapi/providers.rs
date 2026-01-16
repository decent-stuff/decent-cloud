use super::common::{
    check_authorization, decode_pubkey, default_limit, ApiResponse, ApiTags, AutoAcceptRequest,
    AutoAcceptResponse, BulkUpdateStatusRequest, CreatePoolRequest, CreateSetupTokenRequest,
    CsvImportError, CsvImportResult, DuplicateOfferingRequest, HelpcenterSyncResponse,
    LockResponse, NotificationConfigResponse, NotificationUsageResponse, OnboardingUpdateResponse,
    ProvisioningStatusRequest, ReconcileKeepInstance, ReconcileRequest, ReconcileResponse,
    ReconcileTerminateInstance, ReconcileUnknownInstance, RentalResponseRequest,
    ResponseMetricsResponse, ResponseTimeDistributionResponse, TestNotificationRequest,
    TestNotificationResponse, UpdateNotificationConfigRequest, UpdatePoolRequest,
};
use crate::auth::{AgentAuthenticatedUser, ApiAuthenticatedUser, ProviderOrAgentAuth};
use crate::database::{AgentPoolWithStats, Database, SetupToken};
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

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

pub struct ProvidersApi;

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

    /// Get provider contract response metrics
    ///
    /// Returns response time and SLA compliance metrics for contract status changes.
    /// Measures how quickly a provider responds to rental requests (accepts/rejects).
    /// For message response metrics, use `/providers/:pubkey/response-metrics` instead.
    #[oai(
        path = "/providers/:pubkey/contract-response-metrics",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contract_response_metrics(
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

    /// Get provider offerings
    ///
    /// Returns all offerings for a specific provider
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

        match db.get_provider_offerings(&pubkey_bytes).await {
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
            Ok((success_count, errors)) => {
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
                let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

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
                    let contract_provider =
                        hex::decode(&contract.provider_pubkey).unwrap_or_default();
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
                if hex::decode(pool.provider_pubkey).unwrap_or_default() != provider_pubkey {
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
        let expires_at_ns =
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) + lock_duration_ns;

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
