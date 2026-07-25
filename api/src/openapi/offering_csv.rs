//! Provider offering CSV import / export endpoints.
//!
//! Extracted from `providers.rs` (#444 large-file split). These handlers carry
//! the `ApiTags::Offerings` tag. The cluster depends on the shared
//! `validate_cloud_offering` helper, which stays in `providers.rs` (it is also
//! used by the offering create/update handlers there) and is referenced here as
//! `pub(crate)`. Registration is unchanged from the consumer's perspective:
//! `OfferingCsvApi` is combined with the other `*Api` types in
//! `openapi::create_combined_api`, and every path, method, tag, and schema
//! below is identical to the pre-split API.

use super::common::{
    check_authorization, decode_pubkey, ApiResponse, ApiTags, CsvImportError, CsvImportResult,
};
use super::providers::validate_cloud_offering;
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct OfferingCsvApi;

#[OpenApi]
impl OfferingCsvApi {
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
            Err(e) => return poem_openapi::payload::PlainText(e),
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
                // Post-import: validate cloud offerings against live catalog
                match db.get_provider_offerings(&pubkey_bytes).await {
                    Ok(offerings) => {
                        for offering in offerings.iter().filter(|o| {
                            matches!(
                                o.provisioner_type.as_deref(),
                                Some("hetzner") | Some("vultr")
                            )
                        }) {
                            if let Err(e) =
                                validate_cloud_offering(&db, offering, &pubkey_bytes).await
                            {
                                errors.push((
                                    0,
                                    format!(
                                        "{} validation failed for offering '{}': {}",
                                        offering
                                            .provisioner_type
                                            .as_deref()
                                            .unwrap_or("unknown")
                                            .to_uppercase(),
                                        offering.offering_id,
                                        e
                                    ),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "CSV import post-validation: failed to fetch offerings for provider {}: {:#}",
                            hex::encode(&pubkey_bytes),
                            e
                        );
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
}

#[cfg(test)]
mod tests {
    use crate::openapi::common::{CsvImportError, CsvImportResult};

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
}
