use super::common::{
    check_authorization, decode_pubkey, default_limit, ApiResponse, ApiTags,
    BulkUpdateStatusRequest, CsvImportError, CsvImportResult, DuplicateOfferingRequest,
    ProvisioningStatusRequest, RentalResponseRequest,
};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
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

    /// Get provider contracts
    ///
    /// Returns contracts for a specific provider
    #[oai(
        path = "/providers/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contracts(
        &self,
        db: Data<&Arc<Database>>,
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
            Ok(id) => Json(ApiResponse {
                success: true,
                data: Some(id),
                error: None,
            }),
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
    ) -> Json<ApiResponse<usize>> {
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
                let _ = csv_writer.write_record([
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
                    "min_contract_hours",
                    "max_contract_hours",
                    "payment_methods",
                    "features",
                    "operating_systems",
                ]);

                // Write data rows
                for offering in offerings {
                    let _ = csv_writer.write_record([
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
                    ]);
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
    /// Accept or reject a rental request (requires authentication)
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

        let new_status = if req.accept { "accepted" } else { "rejected" };

        match db
            .update_contract_status(&contract_id, new_status, &auth.pubkey, req.memo.as_deref())
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some(format!("Contract {}", new_status)),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update provisioning status
    ///
    /// Updates the provisioning status of a contract (requires authentication)
    #[oai(
        path = "/provider/rental-requests/:id/provisioning",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_provisioning_status(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
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
            .update_contract_status(&contract_id, &req.status, &auth.pubkey, None)
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
}
