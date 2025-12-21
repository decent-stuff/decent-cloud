use super::common::{default_false, default_limit, ApiResponse, ApiTags};
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct OfferingsApi;

#[OpenApi]
impl OfferingsApi {
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
    /// Returns details of a specific offering
    #[oai(path = "/offerings/:id", method = "get", tag = "ApiTags::Offerings")]
    async fn get_offering(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<i64>,
    ) -> Json<ApiResponse<crate::database::offerings::Offering>> {
        match db.get_offering(id.0).await {
            Ok(Some(offering)) => Json(ApiResponse {
                success: true,
                data: Some(offering),
                error: None,
            }),
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
            "gpu_count",
            "gpu_memory_gb",
            "min_contract_hours",
            "max_contract_hours",
            "payment_methods",
            "features",
            "operating_systems",
            "agent_pool_id",
        ]);

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
            ]);
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
                let label = match key.as_str() {
                    "compute" => "💻 Compute / VPS",
                    "dedicated" => "🖥️ Dedicated Server",
                    "gpu" => "🎮 GPU / AI",
                    "network" => "🌐 Network / CDN",
                    "storage" => "💾 Storage",
                    // If new types are added to DB, show them with generic label
                    _ => key.as_str(),
                };
                serde_json::json!({"key": key, "label": label})
            })
            .collect();

        Json(ApiResponse {
            success: true,
            data: Some(product_types),
            error: None,
        })
    }
}
