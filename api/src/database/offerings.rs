use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{offerings, DC_TOKEN_DECIMALS_DIV};
use super::types::{Database, LedgerEntryData};

impl Database {
    // Helper function to calculate pricing from monthly price
    fn calculate_pricing(monthly_price: f64) -> (i64, i64) {
        let price_per_hour_e9s = (monthly_price / 30.0 / 24.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
        let price_per_day_e9s = (monthly_price / 30.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
        (price_per_hour_e9s, price_per_day_e9s)
    }

    // Helper function to insert offering metadata
    async fn insert_offering_metadata(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        offering_id: i64,
        payment_methods: &[String],
        features: &[String],
        operating_systems: &[String],
    ) -> Result<()> {
        // Insert payment methods in normalized table
        for payment_method in payment_methods {
            sqlx::query(
                "INSERT INTO provider_offerings_payment_methods (offering_id, payment_method) VALUES (?, ?)"
            )
            .bind(offering_id)
            .bind(payment_method)
            .execute(&mut **tx)
            .await?;
        }

        // Insert features in normalized table
        for feature in features {
            sqlx::query(
                "INSERT INTO provider_offerings_features (offering_id, feature) VALUES (?, ?)"
            )
            .bind(offering_id)
            .bind(feature)
            .execute(&mut **tx)
            .await?;
        }

        // Insert operating systems in normalized table
        for os in operating_systems {
            sqlx::query(
                "INSERT INTO provider_offerings_operating_systems (offering_id, operating_system) VALUES (?, ?)"
            )
            .bind(offering_id)
            .bind(os)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    // Provider offerings
    pub async fn insert_provider_offerings(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let offering_payload = offerings::UpdateOfferingsPayload::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse offering payload: {}", e))?;
            let provider_key = &entry.key;
            let provider_offerings = offering_payload
                .deserialize_offerings(provider_key)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize offering: {}", e))?;

            // Store each offering as a fully structured record
            for offering in &provider_offerings.server_offerings {
                let (price_per_hour_e9s, price_per_day_e9s) = 
                    Self::calculate_pricing(offering.monthly_price);

                // Insert main offering record
                let offering_id = sqlx::query_scalar::<_, i64>(
                    "INSERT INTO provider_offerings (
                        pubkey_hash, offering_id, offer_name, description, product_page_url,
                        currency, monthly_price, setup_fee, visibility, product_type,
                        virtualization_type, billing_interval, stock_status, processor_brand,
                        processor_amount, processor_cores, processor_speed, processor_name,
                        memory_error_correction, memory_type, memory_amount, hdd_amount,
                        total_hdd_capacity, ssd_amount, total_ssd_capacity, unmetered_bandwidth,
                        uplink_speed, traffic, datacenter_country, datacenter_city,
                        datacenter_latitude, datacenter_longitude, control_panel, gpu_name,
                        price_per_hour_e9s, price_per_day_e9s, min_contract_hours,
                        max_contract_hours, created_at_ns
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    RETURNING id"
                )
                .bind(provider_key)
                .bind(&offering.unique_internal_identifier)
                .bind(&offering.offer_name)
                .bind(&offering.description)
                .bind(&offering.product_page_url)
                .bind(offering.currency.to_string())
                .bind(offering.monthly_price)
                .bind(offering.setup_fee)
                .bind(offering.visibility.to_string())
                .bind(offering.product_type.to_string())
                .bind(offering.virtualization_type.as_ref().map(|t| t.to_string()))
                .bind(offering.billing_interval.to_string())
                .bind(offering.stock.to_string())
                .bind(&offering.processor_brand)
                .bind(offering.processor_amount)
                .bind(offering.processor_cores)
                .bind(&offering.processor_speed)
                .bind(&offering.processor_name)
                .bind(offering.memory_error_correction.as_ref().map(|e| e.to_string()))
                .bind(&offering.memory_type)
                .bind(&offering.memory_amount)
                .bind(offering.hdd_amount)
                .bind(&offering.total_hdd_capacity)
                .bind(offering.ssd_amount)
                .bind(&offering.total_ssd_capacity)
                .bind(!offering.unmetered.is_empty())
                .bind(&offering.uplink_speed)
                .bind(offering.traffic)
                .bind(&offering.datacenter_country)
                .bind(&offering.datacenter_city)
                .bind(offering.datacenter_coordinates.map(|c| c.0))
                .bind(offering.datacenter_coordinates.map(|c| c.1))
                .bind(&offering.control_panel)
                .bind(&offering.gpu_name)
                .bind(price_per_hour_e9s)
                .bind(price_per_day_e9s)
                .bind(Some(1)) // min contract hours
                .bind(None::<i64>) // max contract hours
                .bind(entry.block_timestamp_ns as i64)
                .fetch_one(&mut **tx)
                .await?;

                // Insert metadata (payment methods, features, operating systems)
                Database::insert_offering_metadata(
                    &mut *tx,
                    offering_id,
                    &offering.payment_methods,
                    &offering.features,
                    &offering.operating_systems,
                ).await?;
            }
        }
        Ok(())
    }
}
