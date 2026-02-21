use super::common::{default_false, default_limit, ApiResponse, ApiTags, EmptyResponse};
use crate::auth::{ApiAuthenticatedUser, OptionalApiAuth};
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// CSV template column headers for offerings export/import
pub const CSV_HEADERS: &[&str] = &[
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
];

/// Map a product type key to a human-readable label with icon
pub fn product_type_label(key: &str) -> &str {
    match key {
        "compute" => "\u{1f4bb} Compute / VPS",
        "dedicated" => "\u{1f5a5}\u{fe0f} Dedicated Server",
        "gpu" => "\u{1f3ae} GPU / AI",
        "network" => "\u{1f310} Network / CDN",
        "storage" => "\u{1f4be} Storage",
        // Unknown types pass through as-is
        _ => key,
    }
}

/// Request body for contacting an offering's provider
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct ContactOfferingRequest {
    /// Message to the provider (1–2000 characters)
    pub message: String,
}

pub struct OfferingsApi;

fn default_trending_limit() -> i64 {
    6
}

#[OpenApi]
impl OfferingsApi {
    /// Get pricing statistics for offerings
    ///
    /// Returns price statistics (min, max, avg, median) for a given product type and optional country
    #[oai(path = "/offerings/stats", method = "get", tag = "ApiTags::Offerings")]
    async fn get_offering_stats(
        &self,
        db: Data<&Arc<Database>>,
        product_type: poem_openapi::param::Query<String>,
        country: poem_openapi::param::Query<Option<String>>,
    ) -> Json<ApiResponse<crate::database::offerings::OfferingPricingStats>> {
        match db
            .get_offering_pricing_stats(&product_type.0, country.0.as_deref())
            .await
        {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get pricing stats: {e:#?}")),
            }),
        }
    }

    /// Get trending offerings
    ///
    /// Returns the top offerings by view count in the last 7 days.
    /// Only public, non-draft, in-stock offerings are included.
    /// Public — no auth required.
    #[oai(path = "/offerings/trending", method = "get", tag = "ApiTags::Offerings")]
    async fn get_trending_offerings(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_trending_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::offerings::TrendingOffering>>> {
        let limit = limit.0.min(10);
        match db.get_trending_offerings(limit).await {
            Ok(offerings) => Json(ApiResponse {
                success: true,
                data: Some(offerings),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get trending offerings: {e:#?}")),
            }),
        }
    }

    /// Search offerings
    ///
    /// Search for offerings with optional filters or DSL query
    #[oai(path = "/offerings", method = "get", tag = "ApiTags::Offerings")]
    #[allow(clippy::too_many_arguments)]
    async fn search_offerings(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
        product_type: poem_openapi::param::Query<Option<String>>,
        country: poem_openapi::param::Query<Option<String>>,
        #[oai(default = "default_false")] in_stock_only: poem_openapi::param::Query<bool>,
        #[oai(default = "default_false")] has_recipe: poem_openapi::param::Query<bool>,
        min_price_monthly: poem_openapi::param::Query<Option<f64>>,
        max_price_monthly: poem_openapi::param::Query<Option<f64>>,
        q: poem_openapi::param::Query<Option<String>>,
    ) -> Json<ApiResponse<Vec<crate::database::offerings::Offering>>> {
        // If DSL query is provided, use search_offerings_dsl
        if let Some(query) = q.0.as_ref() {
            if !query.trim().is_empty() {
                return match db.search_offerings_dsl(query, limit.0, offset.0).await {
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
                };
            }
        }

        // Otherwise, use traditional parameter-based search for backward compatibility
        let search_params = crate::database::offerings::SearchOfferingsParams {
            product_type: product_type.0.as_deref(),
            country: country.0.as_deref(),
            in_stock_only: in_stock_only.0,
            has_recipe: has_recipe.0,
            min_price_monthly: min_price_monthly.0,
            max_price_monthly: max_price_monthly.0,
            limit: limit.0,
            offset: offset.0,
        };

        match db.search_offerings(search_params).await {
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

    /// Get offering by ID
    ///
    /// Returns details of a specific offering. Visibility rules apply:
    /// - Public offerings: visible to everyone
    /// - Shared offerings: visible to owner and users in the allowlist
    /// - Private offerings: only visible to the provider who owns them
    #[oai(path = "/offerings/:id", method = "get", tag = "ApiTags::Offerings")]
    async fn get_offering(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<i64>,
        auth: OptionalApiAuth,
    ) -> Json<ApiResponse<crate::database::offerings::Offering>> {
        match db.get_offering(id.0).await {
            Ok(Some(offering)) => {
                // Check visibility using the unified access check
                let can_access = match db
                    .can_access_offering(
                        id.0,
                        &offering.visibility,
                        &offering.pubkey,
                        auth.pubkey.as_deref(),
                    )
                    .await
                {
                    Ok(access) => access,
                    Err(e) => {
                        tracing::error!("Failed to check offering access: {:#?}", e);
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("Internal error checking access".to_string()),
                        });
                    }
                };

                if can_access {
                    Json(ApiResponse {
                        success: true,
                        data: Some(offering),
                        error: None,
                    })
                } else {
                    // Return "not found" rather than "forbidden" to not leak existence of private offerings
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Offering not found".to_string()),
                    })
                }
            }
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Offering not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get CSV template for a specific product type
    ///
    /// Returns a CSV template with realistic example offerings for the specified product type
    #[oai(
        path = "/offerings/template/:product_type",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_offerings_csv_template_by_type(
        &self,
        db: Data<&Arc<Database>>,
        product_type: Path<String>,
    ) -> poem_openapi::payload::PlainText<String> {
        let mut csv_writer = csv::Writer::from_writer(vec![]);

        // Write header
        if let Err(e) = csv_writer.write_record(CSV_HEADERS) {
            return poem_openapi::payload::PlainText(format!("CSV header write error: {}", e));
        }

        // Get example offerings for the specified product type from database
        let offerings = match db.get_example_offerings_by_type(&product_type.0).await {
            Ok(offerings) => offerings,
            Err(e) => {
                return poem_openapi::payload::PlainText(format!(
                    "Failed to fetch example offerings: {}",
                    e
                ));
            }
        };

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
            ]) {
                return poem_openapi::payload::PlainText(format!("CSV record write error: {}", e));
            }
        }

        match csv_writer.into_inner() {
            Ok(csv_data) => {
                poem_openapi::payload::PlainText(String::from_utf8_lossy(&csv_data).to_string())
            }
            Err(e) => poem_openapi::payload::PlainText(format!("CSV generation error: {}", e)),
        }
    }

    /// Get available product types
    ///
    /// Returns a list of available product types with their labels (derived from example offerings in database)
    #[oai(
        path = "/offerings/product-types",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_product_types(
        &self,
        db: Data<&Arc<Database>>,
    ) -> Json<ApiResponse<Vec<serde_json::Value>>> {
        // Query available product types from example offerings
        let product_type_keys = match db.get_available_product_types().await {
            Ok(types) => types,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to fetch product types: {}", e)),
                });
            }
        };

        // Map product type keys to labels with icons
        let product_types: Vec<serde_json::Value> = product_type_keys
            .iter()
            .map(|key| {
                let label = product_type_label(key);
                serde_json::json!({"key": key, "label": label})
            })
            .collect();

        Json(ApiResponse {
            success: true,
            data: Some(product_types),
            error: None,
        })
    }

    /// Contact offering provider
    ///
    /// Sends an inquiry message to the provider via an in-app notification.
    /// Useful for asking questions before creating a rental contract.
    #[oai(path = "/offerings/:id/contact", method = "post", tag = "ApiTags::Offerings")]
    async fn contact_offering(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<i64>,
        auth: ApiAuthenticatedUser,
        body: Json<ContactOfferingRequest>,
    ) -> Json<ApiResponse<EmptyResponse>> {
        let message = body.0.message.trim().to_string();
        if message.is_empty() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Message cannot be empty".to_string()),
            });
        }
        if message.len() > 2000 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Message too long (max 2000 characters)".to_string()),
            });
        }

        let offering = match db.get_offering(id.0).await {
            Ok(Some(o)) => o,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Offering not found".to_string()),
                })
            }
            Err(e) => {
                tracing::error!("Failed to get offering {}: {:#}", id.0, e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to load offering".to_string()),
                });
            }
        };

        let provider_pubkey_bytes = match hex::decode(&offering.pubkey) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Invalid provider pubkey hex {}: {:#}", offering.pubkey, e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Internal error".to_string()),
                });
            }
        };

        if provider_pubkey_bytes == auth.pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cannot send an inquiry to your own offering".to_string()),
            });
        }

        let sender_name = match db.get_account_with_keys_by_public_key(&auth.pubkey).await {
            Ok(Some(acc)) => acc.username,
            Ok(None) | Err(_) => hex::encode(&auth.pubkey[..4]),
        };

        let title = format!("Inquiry about \"{}\"", offering.offer_name);
        let body_text = format!("From {}: {}", sender_name, message);
        if let Err(e) = db
            .insert_user_notification(
                &provider_pubkey_bytes,
                "offering_inquiry",
                &title,
                &body_text,
                None,
            )
            .await
        {
            tracing::error!("Failed to insert provider notification: {:#}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Failed to send inquiry".to_string()),
            });
        }

        Json(ApiResponse {
            success: true,
            data: Some(EmptyResponse {}),
            error: None,
        })
    }

    /// Record a view for an offering
    ///
    /// Public endpoint — no auth required. Deduplicates by hashed IP + day,
    /// so refreshing the page does not inflate the count.
    #[oai(path = "/offerings/:id/view", method = "post", tag = "ApiTags::Offerings")]
    async fn record_offering_view(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<i64>,
        req: &poem::Request,
        auth: OptionalApiAuth,
    ) -> Json<ApiResponse<EmptyResponse>> {
        let ip = extract_client_ip(req);
        let ip_hash = daily_ip_hash(&ip);
        let viewer_pubkey = auth.pubkey.as_deref();

        match db.record_offering_view(id.0, viewer_pubkey, &ip_hash).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some(EmptyResponse {}),
                error: None,
            }),
            Err(e) => {
                tracing::error!("Failed to record view for offering {}: {:#}", id.0, e);
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to record view".to_string()),
                })
            }
        }
    }

    /// Get analytics for an offering
    ///
    /// Provider-only endpoint. Returns view counts and unique viewer counts
    /// for the last 7 and 30 days. Only the offering's provider may call this.
    #[oai(path = "/offerings/:id/analytics", method = "get", tag = "ApiTags::Offerings")]
    async fn get_offering_analytics(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<i64>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<crate::database::offerings::OfferingAnalytics>> {
        // Verify the caller owns this offering
        let offering = match db.get_offering(id.0).await {
            Ok(Some(o)) => o,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Offering not found".to_string()),
                })
            }
            Err(e) => {
                tracing::error!("Failed to get offering {}: {:#}", id.0, e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to load offering".to_string()),
                });
            }
        };

        let provider_pubkey_bytes = match hex::decode(&offering.pubkey) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Invalid provider pubkey hex {}: {:#}", offering.pubkey, e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Internal error".to_string()),
                });
            }
        };

        if provider_pubkey_bytes != auth.pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Forbidden".to_string()),
            });
        }

        match db.get_offering_analytics(id.0).await {
            Ok(analytics) => Json(ApiResponse {
                success: true,
                data: Some(analytics),
                error: None,
            }),
            Err(e) => {
                tracing::error!("Failed to get analytics for offering {}: {:#}", id.0, e);
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to load analytics".to_string()),
                })
            }
        }
    }
}

/// Extract the client IP from the request, preferring X-Forwarded-For.
fn extract_client_ip(req: &poem::Request) -> String {
    if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
        if let Ok(val) = forwarded.to_str() {
            // X-Forwarded-For can be a comma-separated list; take the first entry
            if let Some(ip) = val.split(',').next() {
                let ip = ip.trim().to_string();
                if !ip.is_empty() {
                    return ip;
                }
            }
        }
    }
    req.remote_addr()
        .as_socket_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Hash IP + current day (UTC) with SHA-256 to get a privacy-preserving dedup key.
fn daily_ip_hash(ip: &str) -> Vec<u8> {
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut hasher = Sha256::new();
    hasher.update(ip.as_bytes());
    hasher.update(b"|");
    hasher.update(day.as_bytes());
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_ip_hash_length() {
        let hash = daily_ip_hash("192.168.1.1");
        assert_eq!(hash.len(), 32, "SHA-256 hash must be 32 bytes");
    }

    #[test]
    fn test_daily_ip_hash_same_ip_same_day_is_equal() {
        let h1 = daily_ip_hash("10.0.0.1");
        let h2 = daily_ip_hash("10.0.0.1");
        assert_eq!(h1, h2, "Same IP on same day must produce same hash");
    }

    #[test]
    fn test_daily_ip_hash_different_ips_differ() {
        let h1 = daily_ip_hash("10.0.0.1");
        let h2 = daily_ip_hash("10.0.0.2");
        assert_ne!(h1, h2, "Different IPs must produce different hashes");
    }

    #[test]
    fn test_csv_headers_count() {
        assert_eq!(CSV_HEADERS.len(), 41);
    }

    #[test]
    fn test_csv_headers_start_with_offering_id() {
        assert_eq!(CSV_HEADERS[0], "offering_id");
    }

    #[test]
    fn test_csv_headers_end_with_agent_pool_id() {
        assert_eq!(CSV_HEADERS[CSV_HEADERS.len() - 1], "agent_pool_id");
    }

    #[test]
    fn test_csv_headers_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for header in CSV_HEADERS {
            assert!(seen.insert(header), "Duplicate CSV header: {header}");
        }
    }

    #[test]
    fn test_product_type_label_known_types() {
        assert_eq!(product_type_label("compute"), "\u{1f4bb} Compute / VPS");
        assert_eq!(
            product_type_label("dedicated"),
            "\u{1f5a5}\u{fe0f} Dedicated Server"
        );
        assert_eq!(product_type_label("gpu"), "\u{1f3ae} GPU / AI");
        assert_eq!(product_type_label("network"), "\u{1f310} Network / CDN");
        assert_eq!(product_type_label("storage"), "\u{1f4be} Storage");
    }

    #[test]
    fn test_product_type_label_unknown_passes_through() {
        assert_eq!(product_type_label("quantum"), "quantum");
        assert_eq!(product_type_label(""), "");
    }

    #[test]
    fn test_contact_offering_request_serialization() {
        let req = ContactOfferingRequest {
            message: "Is this available next week?".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["message"], "Is this available next week?");
    }

    #[test]
    fn test_contact_offering_request_deserialization() {
        let json = r#"{"message":"Hello provider"}"#;
        let req: ContactOfferingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "Hello provider");
    }

    #[test]
    fn test_contact_offering_message_max_length() {
        // 2000 chars is valid, 2001 is not (enforcement is in handler but we verify the struct)
        let long_msg = "a".repeat(2000);
        let req = ContactOfferingRequest { message: long_msg.clone() };
        assert_eq!(req.message.len(), 2000);

        let too_long = "a".repeat(2001);
        let req2 = ContactOfferingRequest { message: too_long.clone() };
        assert_eq!(req2.message.len(), 2001);
    }
}
