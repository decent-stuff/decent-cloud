use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{offerings, DC_TOKEN_DECIMALS_DIV};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Offering {
    pub id: i64,
    pub pubkey_hash: Vec<u8>,
    pub offering_id: String,
    pub offer_name: String,
    pub description: Option<String>,
    pub product_page_url: Option<String>,
    pub currency: String,
    pub monthly_price: f64,
    pub setup_fee: f64,
    pub visibility: String,
    pub product_type: String,
    pub virtualization_type: Option<String>,
    pub billing_interval: String,
    pub stock_status: String,
    pub processor_brand: Option<String>,
    pub processor_amount: Option<i64>,
    pub processor_cores: Option<i64>,
    pub processor_speed: Option<String>,
    pub processor_name: Option<String>,
    pub memory_error_correction: Option<String>,
    pub memory_type: Option<String>,
    pub memory_amount: Option<String>,
    pub hdd_amount: Option<i64>,
    pub total_hdd_capacity: Option<String>,
    pub ssd_amount: Option<i64>,
    pub total_ssd_capacity: Option<String>,
    pub unmetered_bandwidth: bool,
    pub uplink_speed: Option<String>,
    pub traffic: Option<i64>,
    pub datacenter_country: String,
    pub datacenter_city: String,
    pub datacenter_latitude: Option<f64>,
    pub datacenter_longitude: Option<f64>,
    pub control_panel: Option<String>,
    pub gpu_name: Option<String>,
    pub price_per_hour_e9s: Option<i64>,
    pub price_per_day_e9s: Option<i64>,
    pub min_contract_hours: Option<i64>,
    pub max_contract_hours: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SearchOfferingsParams<'a> {
    pub product_type: Option<&'a str>,
    pub country: Option<&'a str>,
    pub min_price_e9s: Option<i64>,
    pub max_price_e9s: Option<i64>,
    pub in_stock_only: bool,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateOfferingParams {
    pub offering_id: String,
    pub offer_name: String,
    pub description: Option<String>,
    pub product_page_url: Option<String>,
    pub currency: String,
    pub monthly_price: f64,
    pub setup_fee: f64,
    pub visibility: String,
    pub product_type: String,
    pub virtualization_type: Option<String>,
    pub billing_interval: String,
    pub stock_status: String,
    pub processor_brand: Option<String>,
    pub processor_amount: Option<i64>,
    pub processor_cores: Option<i64>,
    pub processor_speed: Option<String>,
    pub processor_name: Option<String>,
    pub memory_error_correction: Option<String>,
    pub memory_type: Option<String>,
    pub memory_amount: Option<String>,
    pub hdd_amount: Option<i64>,
    pub total_hdd_capacity: Option<String>,
    pub ssd_amount: Option<i64>,
    pub total_ssd_capacity: Option<String>,
    pub unmetered_bandwidth: bool,
    pub uplink_speed: Option<String>,
    pub traffic: Option<i64>,
    pub datacenter_country: String,
    pub datacenter_city: String,
    pub datacenter_latitude: Option<f64>,
    pub datacenter_longitude: Option<f64>,
    pub control_panel: Option<String>,
    pub gpu_name: Option<String>,
    pub min_contract_hours: Option<i64>,
    pub max_contract_hours: Option<i64>,
    pub payment_methods: Vec<String>,
    pub features: Vec<String>,
    pub operating_systems: Vec<String>,
}

#[allow(dead_code)]
impl Database {
    /// Search offerings with filters
    pub async fn search_offerings(
        &self,
        params: SearchOfferingsParams<'_>,
    ) -> Result<Vec<Offering>> {
        let mut query = String::from("SELECT * FROM provider_offerings WHERE 1=1");

        if params.product_type.is_some() {
            query.push_str(" AND product_type = ?");
        }
        if params.country.is_some() {
            query.push_str(" AND datacenter_country = ?");
        }
        if params.min_price_e9s.is_some() {
            query.push_str(" AND price_per_hour_e9s >= ?");
        }
        if params.max_price_e9s.is_some() {
            query.push_str(" AND price_per_hour_e9s <= ?");
        }
        if params.in_stock_only {
            query.push_str(" AND stock_status = ?");
        }

        query.push_str(" ORDER BY price_per_hour_e9s ASC LIMIT ? OFFSET ?");

        let mut query_builder = sqlx::query_as::<_, Offering>(&query);

        if let Some(pt) = params.product_type {
            query_builder = query_builder.bind(pt);
        }
        if let Some(c) = params.country {
            query_builder = query_builder.bind(c);
        }
        if let Some(min) = params.min_price_e9s {
            query_builder = query_builder.bind(min);
        }
        if let Some(max) = params.max_price_e9s {
            query_builder = query_builder.bind(max);
        }
        if params.in_stock_only {
            query_builder = query_builder.bind("in_stock");
        }

        let offerings = query_builder
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(offerings)
    }

    /// Get offerings by provider
    pub async fn get_provider_offerings(&self, pubkey_hash: &[u8]) -> Result<Vec<Offering>> {
        let offerings = sqlx::query_as::<_, Offering>(
            "SELECT * FROM provider_offerings WHERE pubkey_hash = ? ORDER BY monthly_price ASC",
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(offerings)
    }

    /// Get single offering by id
    pub async fn get_offering(&self, offering_id: i64) -> Result<Option<Offering>> {
        let offering =
            sqlx::query_as::<_, Offering>("SELECT * FROM provider_offerings WHERE id = ?")
                .bind(offering_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(offering)
    }

    /// Get offering features
    pub async fn get_offering_features(&self, offering_id: i64) -> Result<Vec<String>> {
        let features: Vec<(String,)> =
            sqlx::query_as("SELECT feature FROM provider_offerings_features WHERE offering_id = ?")
                .bind(offering_id)
                .fetch_all(&self.pool)
                .await?;

        Ok(features.into_iter().map(|(f,)| f).collect())
    }

    /// Get offering payment methods
    pub async fn get_offering_payment_methods(&self, offering_id: i64) -> Result<Vec<String>> {
        let methods: Vec<(String,)> = sqlx::query_as(
            "SELECT payment_method FROM provider_offerings_payment_methods WHERE offering_id = ?",
        )
        .bind(offering_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(methods.into_iter().map(|(m,)| m).collect())
    }

    /// Get offering operating systems
    pub async fn get_offering_operating_systems(&self, offering_id: i64) -> Result<Vec<String>> {
        let oses: Vec<(String,)> = sqlx::query_as(
            "SELECT operating_system FROM provider_offerings_operating_systems WHERE offering_id = ?"
        )
        .bind(offering_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(oses.into_iter().map(|(os,)| os).collect())
    }

    /// Count offerings
    pub async fn count_offerings(&self, filters: Option<&str>) -> Result<i64> {
        let query = if let Some(f) = filters {
            format!("SELECT COUNT(*) FROM provider_offerings WHERE {}", f)
        } else {
            "SELECT COUNT(*) FROM provider_offerings".to_string()
        };

        let count: (i64,) = sqlx::query_as(&query).fetch_one(&self.pool).await?;

        Ok(count.0)
    }

    /// Create a new offering
    pub async fn create_offering(
        &self,
        pubkey_hash: &[u8],
        params: CreateOfferingParams,
    ) -> Result<i64> {
        // Validate required fields
        if params.offering_id.trim().is_empty() {
            return Err(anyhow::anyhow!("offering_id is required"));
        }
        if params.offer_name.trim().is_empty() {
            return Err(anyhow::anyhow!("offer_name is required"));
        }

        let mut tx = self.pool.begin().await?;

        // Check for duplicate offering_id for this provider
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM provider_offerings WHERE pubkey_hash = ? AND offering_id = ?",
        )
        .bind(pubkey_hash)
        .bind(&params.offering_id)
        .fetch_optional(&mut *tx)
        .await?;

        if existing.is_some() {
            return Err(anyhow::anyhow!(
                "Offering with ID '{}' already exists for this provider",
                params.offering_id
            ));
        }

        // Calculate pricing
        let (price_per_hour_e9s, price_per_day_e9s) = Self::calculate_pricing(params.monthly_price);

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
            RETURNING id",
        )
        .bind(pubkey_hash)
        .bind(&params.offering_id)
        .bind(&params.offer_name)
        .bind(&params.description)
        .bind(&params.product_page_url)
        .bind(&params.currency)
        .bind(params.monthly_price)
        .bind(params.setup_fee)
        .bind(&params.visibility)
        .bind(&params.product_type)
        .bind(&params.virtualization_type)
        .bind(&params.billing_interval)
        .bind(&params.stock_status)
        .bind(&params.processor_brand)
        .bind(params.processor_amount)
        .bind(params.processor_cores)
        .bind(&params.processor_speed)
        .bind(&params.processor_name)
        .bind(&params.memory_error_correction)
        .bind(&params.memory_type)
        .bind(&params.memory_amount)
        .bind(params.hdd_amount)
        .bind(&params.total_hdd_capacity)
        .bind(params.ssd_amount)
        .bind(&params.total_ssd_capacity)
        .bind(params.unmetered_bandwidth)
        .bind(&params.uplink_speed)
        .bind(params.traffic)
        .bind(&params.datacenter_country)
        .bind(&params.datacenter_city)
        .bind(params.datacenter_latitude)
        .bind(params.datacenter_longitude)
        .bind(&params.control_panel)
        .bind(&params.gpu_name)
        .bind(price_per_hour_e9s)
        .bind(price_per_day_e9s)
        .bind(params.min_contract_hours)
        .bind(params.max_contract_hours)
        .bind(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
        .fetch_one(&mut *tx)
        .await?;

        // Insert metadata
        Self::insert_offering_metadata(
            &mut tx,
            offering_id,
            &params.payment_methods,
            &params.features,
            &params.operating_systems,
        )
        .await?;

        tx.commit().await?;
        Ok(offering_id)
    }

    /// Update an existing offering
    pub async fn update_offering(
        &self,
        pubkey_hash: &[u8],
        offering_db_id: i64,
        params: CreateOfferingParams,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Verify ownership
        let owner: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT pubkey_hash FROM provider_offerings WHERE id = ?")
                .bind(offering_db_id)
                .fetch_optional(&mut *tx)
                .await?;

        match owner {
            None => return Err(anyhow::anyhow!("Offering not found")),
            Some((owner_pubkey,)) if owner_pubkey != pubkey_hash => {
                return Err(anyhow::anyhow!(
                    "Unauthorized: You do not own this offering"
                ))
            }
            _ => {}
        }

        let (price_per_hour_e9s, price_per_day_e9s) = Self::calculate_pricing(params.monthly_price);

        sqlx::query(
            "UPDATE provider_offerings SET
                offering_id = ?, offer_name = ?, description = ?, product_page_url = ?,
                currency = ?, monthly_price = ?, setup_fee = ?, visibility = ?, product_type = ?,
                virtualization_type = ?, billing_interval = ?, stock_status = ?,
                processor_brand = ?, processor_amount = ?, processor_cores = ?, processor_speed = ?,
                processor_name = ?, memory_error_correction = ?, memory_type = ?, memory_amount = ?,
                hdd_amount = ?, total_hdd_capacity = ?, ssd_amount = ?, total_ssd_capacity = ?,
                unmetered_bandwidth = ?, uplink_speed = ?, traffic = ?, datacenter_country = ?,
                datacenter_city = ?, datacenter_latitude = ?, datacenter_longitude = ?,
                control_panel = ?, gpu_name = ?, price_per_hour_e9s = ?, price_per_day_e9s = ?,
                min_contract_hours = ?, max_contract_hours = ?
            WHERE id = ?",
        )
        .bind(&params.offering_id)
        .bind(&params.offer_name)
        .bind(&params.description)
        .bind(&params.product_page_url)
        .bind(&params.currency)
        .bind(params.monthly_price)
        .bind(params.setup_fee)
        .bind(&params.visibility)
        .bind(&params.product_type)
        .bind(&params.virtualization_type)
        .bind(&params.billing_interval)
        .bind(&params.stock_status)
        .bind(&params.processor_brand)
        .bind(params.processor_amount)
        .bind(params.processor_cores)
        .bind(&params.processor_speed)
        .bind(&params.processor_name)
        .bind(&params.memory_error_correction)
        .bind(&params.memory_type)
        .bind(&params.memory_amount)
        .bind(params.hdd_amount)
        .bind(&params.total_hdd_capacity)
        .bind(params.ssd_amount)
        .bind(&params.total_ssd_capacity)
        .bind(params.unmetered_bandwidth)
        .bind(&params.uplink_speed)
        .bind(params.traffic)
        .bind(&params.datacenter_country)
        .bind(&params.datacenter_city)
        .bind(params.datacenter_latitude)
        .bind(params.datacenter_longitude)
        .bind(&params.control_panel)
        .bind(&params.gpu_name)
        .bind(price_per_hour_e9s)
        .bind(price_per_day_e9s)
        .bind(params.min_contract_hours)
        .bind(params.max_contract_hours)
        .bind(offering_db_id)
        .execute(&mut *tx)
        .await?;

        // Delete old metadata
        sqlx::query("DELETE FROM provider_offerings_payment_methods WHERE offering_id = ?")
            .bind(offering_db_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM provider_offerings_features WHERE offering_id = ?")
            .bind(offering_db_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM provider_offerings_operating_systems WHERE offering_id = ?")
            .bind(offering_db_id)
            .execute(&mut *tx)
            .await?;

        // Insert new metadata
        Self::insert_offering_metadata(
            &mut tx,
            offering_db_id,
            &params.payment_methods,
            &params.features,
            &params.operating_systems,
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Delete an offering
    pub async fn delete_offering(&self, pubkey_hash: &[u8], offering_db_id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Verify ownership
        let owner: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT pubkey_hash FROM provider_offerings WHERE id = ?")
                .bind(offering_db_id)
                .fetch_optional(&mut *tx)
                .await?;

        match owner {
            None => return Err(anyhow::anyhow!("Offering not found")),
            Some((owner_pubkey,)) if owner_pubkey != pubkey_hash => {
                return Err(anyhow::anyhow!(
                    "Unauthorized: You do not own this offering"
                ))
            }
            _ => {}
        }

        // Delete offering (CASCADE will handle metadata tables)
        sqlx::query("DELETE FROM provider_offerings WHERE id = ?")
            .bind(offering_db_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Duplicate an offering
    pub async fn duplicate_offering(
        &self,
        pubkey_hash: &[u8],
        source_offering_id: i64,
        new_offering_id: String,
    ) -> Result<i64> {
        // Get source offering
        let source = self.get_offering(source_offering_id).await?;
        let source = source.ok_or_else(|| anyhow::anyhow!("Source offering not found"))?;

        // Verify ownership
        if source.pubkey_hash != pubkey_hash {
            return Err(anyhow::anyhow!(
                "Unauthorized: You do not own this offering"
            ));
        }

        // Get metadata
        let payment_methods = self
            .get_offering_payment_methods(source_offering_id)
            .await?;
        let features = self.get_offering_features(source_offering_id).await?;
        let operating_systems = self
            .get_offering_operating_systems(source_offering_id)
            .await?;

        // Create new offering with duplicated data
        let params = CreateOfferingParams {
            offering_id: new_offering_id,
            offer_name: format!("{} (Copy)", source.offer_name),
            description: source.description,
            product_page_url: source.product_page_url,
            currency: source.currency,
            monthly_price: source.monthly_price,
            setup_fee: source.setup_fee,
            visibility: source.visibility,
            product_type: source.product_type,
            virtualization_type: source.virtualization_type,
            billing_interval: source.billing_interval,
            stock_status: source.stock_status,
            processor_brand: source.processor_brand,
            processor_amount: source.processor_amount,
            processor_cores: source.processor_cores,
            processor_speed: source.processor_speed,
            processor_name: source.processor_name,
            memory_error_correction: source.memory_error_correction,
            memory_type: source.memory_type,
            memory_amount: source.memory_amount,
            hdd_amount: source.hdd_amount,
            total_hdd_capacity: source.total_hdd_capacity,
            ssd_amount: source.ssd_amount,
            total_ssd_capacity: source.total_ssd_capacity,
            unmetered_bandwidth: source.unmetered_bandwidth,
            uplink_speed: source.uplink_speed,
            traffic: source.traffic,
            datacenter_country: source.datacenter_country,
            datacenter_city: source.datacenter_city,
            datacenter_latitude: source.datacenter_latitude,
            datacenter_longitude: source.datacenter_longitude,
            control_panel: source.control_panel,
            gpu_name: source.gpu_name,
            min_contract_hours: source.min_contract_hours,
            max_contract_hours: source.max_contract_hours,
            payment_methods,
            features,
            operating_systems,
        };

        self.create_offering(pubkey_hash, params).await
    }

    // Helper function to calculate pricing from monthly price
    #[allow(dead_code)]
    fn calculate_pricing(monthly_price: f64) -> (i64, i64) {
        let price_per_hour_e9s =
            (monthly_price / 30.0 / 24.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
        let price_per_day_e9s = (monthly_price / 30.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
        (price_per_hour_e9s, price_per_day_e9s)
    }

    // Helper function to insert offering metadata
    #[allow(dead_code)]
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
                "INSERT INTO provider_offerings_features (offering_id, feature) VALUES (?, ?)",
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
    #[allow(dead_code)]
    pub async fn insert_provider_offerings(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let offering_payload = match offerings::UpdateOfferingsPayload::try_from_slice(
                &entry.value,
            ) {
                Ok(payload) => payload,
                Err(e) => {
                    tracing::warn!("Skipping malformed offering entry with key {:?}: {}. Raw data (first 50 bytes): {:?}", 
                        &entry.key, e, &entry.value.get(..50));
                    continue; // Skip this entry and continue with others
                }
            };
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
                )
                .await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> Database {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let migration_sql = include_str!("../../migrations/001_original_schema.sql");
        sqlx::query(migration_sql).execute(&pool).await.unwrap();
        Database { pool }
    }

    async fn insert_test_offering(
        db: &Database,
        id: i64,
        pubkey: &[u8],
        country: &str,
        price: f64,
    ) {
        let offering_id = format!("off-{}", id);
        // Calculate price_per_hour_e9s from monthly price (rough approximation)
        let price_per_hour_e9s = (price * 1_000_000_000.0 / 30.0 / 24.0) as i64;
        sqlx::query("INSERT INTO provider_offerings (id, pubkey_hash, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, price_per_hour_e9s, created_at_ns) VALUES (?, ?, ?, 'Test Offer', 'USD', ?, 0, 'public', 'compute', 'monthly', 'in_stock', ?, 'City', 0, ?, 0)")
            .bind(id).bind(pubkey).bind(&offering_id).bind(price).bind(country).bind(price_per_hour_e9s).execute(&db.pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_provider_offerings_empty() {
        let db = setup_test_db().await;
        let offerings = db.get_provider_offerings(&[1u8; 32]).await.unwrap();
        assert_eq!(offerings.len(), 0);
    }

    #[tokio::test]
    async fn test_get_provider_offerings() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;
        insert_test_offering(&db, 2, &pubkey, "EU", 200.0).await;

        let offerings = db.get_provider_offerings(&pubkey).await.unwrap();
        assert_eq!(offerings.len(), 2);
    }

    #[tokio::test]
    async fn test_get_offering_by_id() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 42, &[1u8; 32], "US", 100.0).await;

        let offering = db.get_offering(42).await.unwrap();
        assert!(offering.is_some());
        assert_eq!(offering.unwrap().id, 42);
    }

    #[tokio::test]
    async fn test_get_offering_not_found() {
        let db = setup_test_db().await;
        let offering = db.get_offering(999).await.unwrap();
        assert!(offering.is_none());
    }

    #[tokio::test]
    async fn test_get_offering_features() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;

        sqlx::query(
            "INSERT INTO provider_offerings_features (offering_id, feature) VALUES (?, 'SSD')",
        )
        .bind(1)
        .execute(&db.pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO provider_offerings_features (offering_id, feature) VALUES (?, 'Backup')",
        )
        .bind(1)
        .execute(&db.pool)
        .await
        .unwrap();

        let features = db.get_offering_features(1).await.unwrap();
        assert_eq!(features.len(), 2);
        assert!(features.contains(&"SSD".to_string()));
    }

    #[tokio::test]
    async fn test_get_offering_payment_methods() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;

        sqlx::query("INSERT INTO provider_offerings_payment_methods (offering_id, payment_method) VALUES (?, 'BTC')")
            .bind(1).execute(&db.pool).await.unwrap();

        let methods = db.get_offering_payment_methods(1).await.unwrap();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0], "BTC");
    }

    #[tokio::test]
    async fn test_get_offering_operating_systems() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;

        sqlx::query("INSERT INTO provider_offerings_operating_systems (offering_id, operating_system) VALUES (?, 'Ubuntu')")
            .bind(1).execute(&db.pool).await.unwrap();

        let oses = db.get_offering_operating_systems(1).await.unwrap();
        assert_eq!(oses.len(), 1);
        assert_eq!(oses[0], "Ubuntu");
    }

    #[tokio::test]
    async fn test_count_offerings_no_filters() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;
        insert_test_offering(&db, 2, &[2u8; 32], "EU", 200.0).await;

        let count = db.count_offerings(None).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_search_offerings_no_filters() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;
        insert_test_offering(&db, 2, &[2u8; 32], "EU", 200.0).await;

        let results = db
            .search_offerings(SearchOfferingsParams {
                product_type: None,
                country: None,
                min_price_e9s: None,
                max_price_e9s: None,
                in_stock_only: false,
                limit: 10,
                offset: 0,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_offerings_by_country() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;
        insert_test_offering(&db, 2, &[2u8; 32], "EU", 200.0).await;

        let results = db
            .search_offerings(SearchOfferingsParams {
                product_type: None,
                country: Some("US"),
                min_price_e9s: None,
                max_price_e9s: None,
                in_stock_only: false,
                limit: 10,
                offset: 0,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].datacenter_country, "US");
    }

    #[tokio::test]
    async fn test_search_offerings_price_range() {
        let db = setup_test_db().await;
        insert_test_offering(&db, 1, &[1u8; 32], "US", 50.0).await;
        insert_test_offering(&db, 2, &[2u8; 32], "US", 150.0).await;
        insert_test_offering(&db, 3, &[3u8; 32], "US", 250.0).await;

        // Filter by price_per_hour_e9s (150 / 30 / 24 * 1e9 = ~208M)
        let min_price = (100.0 * 1_000_000_000.0 / 30.0 / 24.0) as i64;
        let max_price = (200.0 * 1_000_000_000.0 / 30.0 / 24.0) as i64;
        let results = db
            .search_offerings(SearchOfferingsParams {
                product_type: None,
                country: None,
                min_price_e9s: Some(min_price),
                max_price_e9s: Some(max_price),
                in_stock_only: false,
                limit: 10,
                offset: 0,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].monthly_price, 150.0);
    }

    #[tokio::test]
    async fn test_search_offerings_pagination() {
        let db = setup_test_db().await;
        for i in 0..5 {
            insert_test_offering(&db, i, &[i as u8; 32], "US", 100.0).await;
        }

        let page1 = db
            .search_offerings(SearchOfferingsParams {
                product_type: None,
                country: None,
                min_price_e9s: None,
                max_price_e9s: None,
                in_stock_only: false,
                limit: 2,
                offset: 0,
            })
            .await
            .unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = db
            .search_offerings(SearchOfferingsParams {
                product_type: None,
                country: None,
                min_price_e9s: None,
                max_price_e9s: None,
                in_stock_only: false,
                limit: 2,
                offset: 2,
            })
            .await
            .unwrap();
        assert_eq!(page2.len(), 2);
    }

    // CRUD Tests
    #[tokio::test]
    async fn test_create_offering_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        let params = CreateOfferingParams {
            offering_id: "test-offer-1".to_string(),
            offer_name: "Test Server".to_string(),
            description: Some("Test description".to_string()),
            product_page_url: None,
            currency: "USD".to_string(),
            monthly_price: 99.99,
            setup_fee: 0.0,
            visibility: "public".to_string(),
            product_type: "dedicated_server".to_string(),
            virtualization_type: None,
            billing_interval: "monthly".to_string(),
            stock_status: "in_stock".to_string(),
            processor_brand: Some("Intel".to_string()),
            processor_amount: Some(2),
            processor_cores: Some(16),
            processor_speed: Some("3.0GHz".to_string()),
            processor_name: Some("Xeon E5-2670".to_string()),
            memory_error_correction: None,
            memory_type: Some("DDR4".to_string()),
            memory_amount: Some("64GB".to_string()),
            hdd_amount: Some(0),
            total_hdd_capacity: None,
            ssd_amount: Some(2),
            total_ssd_capacity: Some("1TB".to_string()),
            unmetered_bandwidth: true,
            uplink_speed: Some("1Gbps".to_string()),
            traffic: None,
            datacenter_country: "US".to_string(),
            datacenter_city: "New York".to_string(),
            datacenter_latitude: Some(40.7128),
            datacenter_longitude: Some(-74.0060),
            control_panel: None,
            gpu_name: None,
            min_contract_hours: Some(1),
            max_contract_hours: None,
            payment_methods: vec!["BTC".to_string(), "ETH".to_string()],
            features: vec!["RAID".to_string(), "Backup".to_string()],
            operating_systems: vec!["Ubuntu 22.04".to_string()],
        };

        let offering_id = db.create_offering(&pubkey, params).await.unwrap();
        assert!(offering_id > 0);

        // Verify the offering was created
        let offering = db.get_offering(offering_id).await.unwrap();
        assert!(offering.is_some());
        let offering = offering.unwrap();
        assert_eq!(offering.offer_name, "Test Server");
        assert_eq!(offering.monthly_price, 99.99);

        // Verify metadata
        let methods = db.get_offering_payment_methods(offering_id).await.unwrap();
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"BTC".to_string()));

        let features = db.get_offering_features(offering_id).await.unwrap();
        assert_eq!(features.len(), 2);

        let oses = db
            .get_offering_operating_systems(offering_id)
            .await
            .unwrap();
        assert_eq!(oses.len(), 1);
    }

    #[tokio::test]
    async fn test_create_offering_duplicate_id() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        let params = CreateOfferingParams {
            offering_id: "duplicate-offer".to_string(),
            offer_name: "First Offer".to_string(),
            description: None,
            product_page_url: None,
            currency: "USD".to_string(),
            monthly_price: 50.0,
            setup_fee: 0.0,
            visibility: "public".to_string(),
            product_type: "vps".to_string(),
            virtualization_type: Some("kvm".to_string()),
            billing_interval: "monthly".to_string(),
            stock_status: "in_stock".to_string(),
            processor_brand: None,
            processor_amount: None,
            processor_cores: Some(2),
            processor_speed: None,
            processor_name: None,
            memory_error_correction: None,
            memory_type: None,
            memory_amount: Some("4GB".to_string()),
            hdd_amount: None,
            total_hdd_capacity: None,
            ssd_amount: Some(1),
            total_ssd_capacity: Some("50GB".to_string()),
            unmetered_bandwidth: false,
            uplink_speed: None,
            traffic: Some(1000),
            datacenter_country: "US".to_string(),
            datacenter_city: "Dallas".to_string(),
            datacenter_latitude: None,
            datacenter_longitude: None,
            control_panel: None,
            gpu_name: None,
            min_contract_hours: Some(1),
            max_contract_hours: None,
            payment_methods: vec![],
            features: vec![],
            operating_systems: vec![],
        };

        // First creation should succeed
        let result1 = db.create_offering(&pubkey, params.clone()).await;
        assert!(result1.is_ok());

        // Second creation with same offering_id should fail
        let result2 = db.create_offering(&pubkey, params).await;
        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_offering_missing_required_fields() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        let params = CreateOfferingParams {
            offering_id: "".to_string(), // Empty offering_id
            offer_name: "Test".to_string(),
            description: None,
            product_page_url: None,
            currency: "USD".to_string(),
            monthly_price: 10.0,
            setup_fee: 0.0,
            visibility: "public".to_string(),
            product_type: "vps".to_string(),
            virtualization_type: None,
            billing_interval: "monthly".to_string(),
            stock_status: "in_stock".to_string(),
            processor_brand: None,
            processor_amount: None,
            processor_cores: None,
            processor_speed: None,
            processor_name: None,
            memory_error_correction: None,
            memory_type: None,
            memory_amount: None,
            hdd_amount: None,
            total_hdd_capacity: None,
            ssd_amount: None,
            total_ssd_capacity: None,
            unmetered_bandwidth: false,
            uplink_speed: None,
            traffic: None,
            datacenter_country: "US".to_string(),
            datacenter_city: "Test".to_string(),
            datacenter_latitude: None,
            datacenter_longitude: None,
            control_panel: None,
            gpu_name: None,
            min_contract_hours: None,
            max_contract_hours: None,
            payment_methods: vec![],
            features: vec![],
            operating_systems: vec![],
        };

        let result = db.create_offering(&pubkey, params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_offering_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        // Create offering first
        insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;

        // Update it
        let update_params = CreateOfferingParams {
            offering_id: "off-1".to_string(),
            offer_name: "Updated Server".to_string(),
            description: Some("Updated description".to_string()),
            product_page_url: None,
            currency: "EUR".to_string(),
            monthly_price: 199.99,
            setup_fee: 50.0,
            visibility: "private".to_string(),
            product_type: "vps".to_string(),
            virtualization_type: Some("kvm".to_string()),
            billing_interval: "monthly".to_string(),
            stock_status: "out_of_stock".to_string(),
            processor_brand: None,
            processor_amount: None,
            processor_cores: Some(4),
            processor_speed: None,
            processor_name: None,
            memory_error_correction: None,
            memory_type: None,
            memory_amount: Some("16GB".to_string()),
            hdd_amount: None,
            total_hdd_capacity: None,
            ssd_amount: Some(1),
            total_ssd_capacity: Some("500GB".to_string()),
            unmetered_bandwidth: false,
            uplink_speed: None,
            traffic: Some(500),
            datacenter_country: "DE".to_string(),
            datacenter_city: "Berlin".to_string(),
            datacenter_latitude: None,
            datacenter_longitude: None,
            control_panel: None,
            gpu_name: None,
            min_contract_hours: None,
            max_contract_hours: None,
            payment_methods: vec!["ETH".to_string()],
            features: vec!["Backup".to_string()],
            operating_systems: vec!["Debian 12".to_string()],
        };

        let result = db.update_offering(&pubkey, 1, update_params).await;
        assert!(result.is_ok());

        // Verify update
        let offering = db.get_offering(1).await.unwrap().unwrap();
        assert_eq!(offering.offer_name, "Updated Server");
        assert_eq!(offering.monthly_price, 199.99);
        assert_eq!(offering.currency, "EUR");
    }

    #[tokio::test]
    async fn test_update_offering_unauthorized() {
        let db = setup_test_db().await;
        let pubkey1 = vec![1u8; 32];
        let pubkey2 = vec![2u8; 32];

        insert_test_offering(&db, 1, &pubkey1, "US", 100.0).await;

        let params = CreateOfferingParams {
            offering_id: "off-1".to_string(),
            offer_name: "Hacker".to_string(),
            description: None,
            product_page_url: None,
            currency: "USD".to_string(),
            monthly_price: 1.0,
            setup_fee: 0.0,
            visibility: "public".to_string(),
            product_type: "vps".to_string(),
            virtualization_type: None,
            billing_interval: "monthly".to_string(),
            stock_status: "in_stock".to_string(),
            processor_brand: None,
            processor_amount: None,
            processor_cores: None,
            processor_speed: None,
            processor_name: None,
            memory_error_correction: None,
            memory_type: None,
            memory_amount: None,
            hdd_amount: None,
            total_hdd_capacity: None,
            ssd_amount: None,
            total_ssd_capacity: None,
            unmetered_bandwidth: false,
            uplink_speed: None,
            traffic: None,
            datacenter_country: "US".to_string(),
            datacenter_city: "Test".to_string(),
            datacenter_latitude: None,
            datacenter_longitude: None,
            control_panel: None,
            gpu_name: None,
            min_contract_hours: None,
            max_contract_hours: None,
            payment_methods: vec![],
            features: vec![],
            operating_systems: vec![],
        };

        let result = db.update_offering(&pubkey2, 1, params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_delete_offering_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;

        let result = db.delete_offering(&pubkey, 1).await;
        assert!(result.is_ok());

        // Verify deletion
        let offering = db.get_offering(1).await.unwrap();
        assert!(offering.is_none());
    }

    #[tokio::test]
    async fn test_delete_offering_unauthorized() {
        let db = setup_test_db().await;
        let pubkey1 = vec![1u8; 32];
        let pubkey2 = vec![2u8; 32];

        insert_test_offering(&db, 1, &pubkey1, "US", 100.0).await;

        let result = db.delete_offering(&pubkey2, 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_duplicate_offering_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;
        sqlx::query("INSERT INTO provider_offerings_payment_methods (offering_id, payment_method) VALUES (?, 'BTC')")
            .bind(1).execute(&db.pool).await.unwrap();

        let new_id = db
            .duplicate_offering(&pubkey, 1, "off-1-copy".to_string())
            .await
            .unwrap();

        assert!(new_id > 1);

        // Verify duplication
        let duplicated = db.get_offering(new_id).await.unwrap().unwrap();
        assert_eq!(duplicated.offer_name, "Test Offer (Copy)");
        assert_eq!(duplicated.monthly_price, 100.0);
        assert_eq!(duplicated.datacenter_country, "US");

        // Verify metadata was duplicated
        let methods = db.get_offering_payment_methods(new_id).await.unwrap();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0], "BTC");
    }

    #[tokio::test]
    async fn test_duplicate_offering_unauthorized() {
        let db = setup_test_db().await;
        let pubkey1 = vec![1u8; 32];
        let pubkey2 = vec![2u8; 32];

        insert_test_offering(&db, 1, &pubkey1, "US", 100.0).await;

        let result = db.duplicate_offering(&pubkey2, 1, "copy".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));
    }
}
