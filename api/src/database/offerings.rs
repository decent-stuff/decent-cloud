use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::offerings;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct Offering {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    #[oai(skip_serializing_if_is_none)]
    pub id: Option<i64>,
    #[ts(type = "string")]
    pub pubkey: Vec<u8>,
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
    #[ts(type = "number | undefined")]
    pub processor_amount: Option<i64>,
    #[ts(type = "number | undefined")]
    pub processor_cores: Option<i64>,
    pub processor_speed: Option<String>,
    pub processor_name: Option<String>,
    pub memory_error_correction: Option<String>,
    pub memory_type: Option<String>,
    pub memory_amount: Option<String>,
    #[ts(type = "number | undefined")]
    pub hdd_amount: Option<i64>,
    pub total_hdd_capacity: Option<String>,
    #[ts(type = "number | undefined")]
    pub ssd_amount: Option<i64>,
    pub total_ssd_capacity: Option<String>,
    pub unmetered_bandwidth: bool,
    pub uplink_speed: Option<String>,
    #[ts(type = "number | undefined")]
    pub traffic: Option<i64>,
    pub datacenter_country: String,
    pub datacenter_city: String,
    pub datacenter_latitude: Option<f64>,
    pub datacenter_longitude: Option<f64>,
    pub control_panel: Option<String>,
    pub gpu_name: Option<String>,
    #[ts(type = "number | undefined")]
    pub min_contract_hours: Option<i64>,
    #[ts(type = "number | undefined")]
    pub max_contract_hours: Option<i64>,
    pub payment_methods: Option<String>,
    pub features: Option<String>,
    pub operating_systems: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchOfferingsParams<'a> {
    pub product_type: Option<&'a str>,
    pub country: Option<&'a str>,
    pub in_stock_only: bool,
    pub limit: i64,
    pub offset: i64,
}

// CreateOfferingParams eliminated - use Offering with id: None for creation

#[allow(dead_code)]
impl Database {
    /// Search offerings with filters
    pub async fn search_offerings(
        &self,
        params: SearchOfferingsParams<'_>,
    ) -> Result<Vec<Offering>> {
        let mut query =
            String::from("SELECT id, hex(pubkey) as \"pubkey!: String\", offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee, visibility, product_type, virtualization_type, billing_interval, stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name, memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity, ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic, datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude, control_panel, gpu_name, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems FROM provider_offerings WHERE LOWER(visibility) = 'public'");

        if params.product_type.is_some() {
            query.push_str(" AND product_type = ?");
        }
        if params.country.is_some() {
            query.push_str(" AND datacenter_country = ?");
        }
        if params.in_stock_only {
            query.push_str(" AND stock_status = ?");
        }

        query.push_str(" ORDER BY monthly_price ASC LIMIT ? OFFSET ?");

        let mut query_builder = sqlx::query_as::<_, Offering>(&query);

        if let Some(pt) = params.product_type {
            query_builder = query_builder.bind(pt);
        }
        if let Some(c) = params.country {
            query_builder = query_builder.bind(c);
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
    pub async fn get_provider_offerings(&self, pubkey: &[u8]) -> Result<Vec<Offering>> {
        let offerings = sqlx::query_as::<_, Offering>(
            r#"SELECT id, hex(pubkey) as "pubkey!: String", offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval, stock_status,
               processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems
               FROM provider_offerings WHERE pubkey = ? ORDER BY monthly_price ASC"#
        )
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(offerings)
    }

    /// Get single offering by id
    pub async fn get_offering(&self, offering_id: i64) -> Result<Option<Offering>> {
        let offering =
            sqlx::query_as::<_, Offering>(r#"SELECT id, hex(pubkey) as "pubkey!: String", offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval, stock_status,
               processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems
               FROM provider_offerings WHERE id = ?"#)
                .bind(offering_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(offering)
    }

    /// Get example offerings for CSV template generation
    pub async fn get_example_offerings(&self) -> Result<Vec<Offering>> {
        // Use the same distinctive hash as in migration 002
        let example_provider_pubkey =
            hex::decode("6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572")
                .unwrap();
        let offerings = sqlx::query_as::<_, Offering>(
            r#"SELECT id, hex(pubkey) as "pubkey!: String", offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval, stock_status,
               processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems
               FROM provider_offerings WHERE pubkey = ? ORDER BY offering_id ASC"#
        )
        .bind(&example_provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(offerings)
    }

    /// Count offerings
    pub async fn count_offerings(&self, filters: Option<&str>) -> Result<i64> {
        let query = if let Some(f) = filters {
            format!(
                "SELECT COUNT(*) FROM provider_offerings WHERE LOWER(visibility) = 'public' AND ({})",
                f
            )
        } else {
            "SELECT COUNT(*) FROM provider_offerings WHERE LOWER(visibility) = 'public'".to_string()
        };

        let count: (i64,) = sqlx::query_as(&query).fetch_one(&self.pool).await?;

        Ok(count.0)
    }

    /// Create a new offering
    pub async fn create_offering(&self, pubkey: &[u8], params: Offering) -> Result<i64> {
        // Validate required fields
        if params.offering_id.trim().is_empty() {
            return Err(anyhow::anyhow!("offering_id is required"));
        }
        if params.offer_name.trim().is_empty() {
            return Err(anyhow::anyhow!("offer_name is required"));
        }

        let Offering {
            id: _,
            pubkey: _,
            offering_id,
            offer_name,
            description,
            product_page_url,
            currency,
            monthly_price,
            setup_fee,
            visibility,
            product_type,
            virtualization_type,
            billing_interval,
            stock_status,
            processor_brand,
            processor_amount,
            processor_cores,
            processor_speed,
            processor_name,
            memory_error_correction,
            memory_type,
            memory_amount,
            hdd_amount,
            total_hdd_capacity,
            ssd_amount,
            total_ssd_capacity,
            unmetered_bandwidth,
            uplink_speed,
            traffic,
            datacenter_country,
            datacenter_city,
            datacenter_latitude,
            datacenter_longitude,
            control_panel,
            gpu_name,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
        } = params;

        let mut tx = self.pool.begin().await?;

        // Check for duplicate offering_id for this provider
        let existing: Option<i64> = sqlx::query_scalar!(
            r#"SELECT id as "id!: i64" FROM provider_offerings WHERE pubkey = ? AND offering_id = ?"#,
            pubkey,
            offering_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        if existing.is_some() {
            return Err(anyhow::anyhow!(
                "Offering with ID '{}' already exists for this provider",
                offering_id.as_str()
            ));
        }

        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // Insert main offering record
        let offering_id = sqlx::query_scalar!(
            r#"INSERT INTO provider_offerings (
                pubkey, offering_id, offer_name, description, product_page_url,
                currency, monthly_price, setup_fee, visibility, product_type,
                virtualization_type, billing_interval, stock_status, processor_brand,
                processor_amount, processor_cores, processor_speed, processor_name,
                memory_error_correction, memory_type, memory_amount, hdd_amount,
                total_hdd_capacity, ssd_amount, total_ssd_capacity, unmetered_bandwidth,
                uplink_speed, traffic, datacenter_country, datacenter_city,
                datacenter_latitude, datacenter_longitude, control_panel, gpu_name,
                min_contract_hours, max_contract_hours, payment_methods, features,
                operating_systems, created_at_ns
            ) VALUES (
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?
            )
            RETURNING id"#,
            pubkey,
            offering_id,
            offer_name,
            description,
            product_page_url,
            currency,
            monthly_price,
            setup_fee,
            visibility,
            product_type,
            virtualization_type,
            billing_interval,
            stock_status,
            processor_brand,
            processor_amount,
            processor_cores,
            processor_speed,
            processor_name,
            memory_error_correction,
            memory_type,
            memory_amount,
            hdd_amount,
            total_hdd_capacity,
            ssd_amount,
            total_ssd_capacity,
            unmetered_bandwidth,
            uplink_speed,
            traffic,
            datacenter_country,
            datacenter_city,
            datacenter_latitude,
            datacenter_longitude,
            control_panel,
            gpu_name,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
            created_at_ns
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(offering_id)
    }

    /// Update an existing offering
    pub async fn update_offering(
        &self,
        pubkey: &[u8],
        offering_db_id: i64,
        params: Offering,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Verify ownership
        let owner: Option<Vec<u8>> = sqlx::query_scalar!(
            "SELECT pubkey FROM provider_offerings WHERE id = ?",
            offering_db_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        match owner {
            None => return Err(anyhow::anyhow!("Offering not found")),
            Some(owner_pubkey) if owner_pubkey != pubkey => {
                return Err(anyhow::anyhow!(
                    "Unauthorized: You do not own this offering"
                ))
            }
            _ => {}
        }

        let Offering {
            id: _,
            pubkey: _,
            offering_id,
            offer_name,
            description,
            product_page_url,
            currency,
            monthly_price,
            setup_fee,
            visibility,
            product_type,
            virtualization_type,
            billing_interval,
            stock_status,
            processor_brand,
            processor_amount,
            processor_cores,
            processor_speed,
            processor_name,
            memory_error_correction,
            memory_type,
            memory_amount,
            hdd_amount,
            total_hdd_capacity,
            ssd_amount,
            total_ssd_capacity,
            unmetered_bandwidth,
            uplink_speed,
            traffic,
            datacenter_country,
            datacenter_city,
            datacenter_latitude,
            datacenter_longitude,
            control_panel,
            gpu_name,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
        } = params;

        sqlx::query!(
            r#"UPDATE provider_offerings SET
                offering_id = ?, offer_name = ?, description = ?, product_page_url = ?,
                currency = ?, monthly_price = ?, setup_fee = ?, visibility = ?, product_type = ?,
                virtualization_type = ?, billing_interval = ?, stock_status = ?,
                processor_brand = ?, processor_amount = ?, processor_cores = ?, processor_speed = ?,
                processor_name = ?, memory_error_correction = ?, memory_type = ?, memory_amount = ?,
                hdd_amount = ?, total_hdd_capacity = ?, ssd_amount = ?, total_ssd_capacity = ?,
                unmetered_bandwidth = ?, uplink_speed = ?, traffic = ?, datacenter_country = ?,
                datacenter_city = ?, datacenter_latitude = ?, datacenter_longitude = ?,
                control_panel = ?, gpu_name = ?, min_contract_hours = ?, max_contract_hours = ?,
                payment_methods = ?, features = ?, operating_systems = ?
            WHERE id = ?"#,
            offering_id,
            offer_name,
            description,
            product_page_url,
            currency,
            monthly_price,
            setup_fee,
            visibility,
            product_type,
            virtualization_type,
            billing_interval,
            stock_status,
            processor_brand,
            processor_amount,
            processor_cores,
            processor_speed,
            processor_name,
            memory_error_correction,
            memory_type,
            memory_amount,
            hdd_amount,
            total_hdd_capacity,
            ssd_amount,
            total_ssd_capacity,
            unmetered_bandwidth,
            uplink_speed,
            traffic,
            datacenter_country,
            datacenter_city,
            datacenter_latitude,
            datacenter_longitude,
            control_panel,
            gpu_name,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
            offering_db_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Delete an offering
    pub async fn delete_offering(&self, pubkey: &[u8], offering_db_id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Verify ownership
        let owner: Option<Vec<u8>> = sqlx::query_scalar!(
            "SELECT pubkey FROM provider_offerings WHERE id = ?",
            offering_db_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        match owner {
            None => return Err(anyhow::anyhow!("Offering not found")),
            Some(owner_pubkey) if owner_pubkey != pubkey => {
                return Err(anyhow::anyhow!(
                    "Unauthorized: You do not own this offering"
                ))
            }
            _ => {}
        }

        // Delete offering (CASCADE will handle metadata tables)
        sqlx::query!(
            "DELETE FROM provider_offerings WHERE id = ?",
            offering_db_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Duplicate an offering
    pub async fn duplicate_offering(
        &self,
        pubkey: &[u8],
        source_offering_id: i64,
        new_offering_id: String,
    ) -> Result<i64> {
        // Get source offering
        let source = self.get_offering(source_offering_id).await?;
        let source = source.ok_or_else(|| anyhow::anyhow!("Source offering not found"))?;

        // Verify ownership
        if source.pubkey != (pubkey) {
            return Err(anyhow::anyhow!(
                "Unauthorized: You do not own this offering"
            ));
        }

        // Get metadata directly from source offering

        // Create new offering with duplicated data
        let params = Offering {
            id: None,
            pubkey: (pubkey).to_vec(),
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
            payment_methods: source.payment_methods,
            features: source.features,
            operating_systems: source.operating_systems,
        };

        self.create_offering(pubkey, params).await
    }

    /// Bulk update stock_status for multiple offerings
    pub async fn bulk_update_stock_status(
        &self,
        pubkey: &[u8],
        offering_ids: &[i64],
        new_status: &str,
    ) -> Result<usize> {
        if offering_ids.is_empty() {
            return Ok(0);
        }

        // Verify all offerings belong to this provider
        let placeholders = offering_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let verify_query = format!(
            "SELECT COUNT(*) as count FROM provider_offerings WHERE id IN ({}) AND (pubkey) = ?",
            placeholders
        );

        let mut query_builder = sqlx::query_scalar::<_, i64>(&verify_query);
        for id in offering_ids {
            query_builder = query_builder.bind(id);
        }
        query_builder = query_builder.bind(pubkey);

        let count: i64 = query_builder.fetch_one(&self.pool).await?;

        if count != offering_ids.len() as i64 {
            anyhow::bail!("Not all offerings belong to this provider or some IDs are invalid");
        }

        // Update stock_status
        let update_query = format!(
            "UPDATE provider_offerings SET stock_status = ? WHERE id IN ({})",
            placeholders
        );

        let mut update_builder = sqlx::query(&update_query);
        update_builder = update_builder.bind(new_status);
        for id in offering_ids {
            update_builder = update_builder.bind(id);
        }

        let result = update_builder.execute(&self.pool).await?;
        Ok(result.rows_affected() as usize)
    }

    // Helper function to convert Vec<String> to Option<String> (comma-separated)
    fn vec_to_csv(vec: &[String]) -> Option<String> {
        if vec.is_empty() {
            None
        } else {
            Some(vec.join(","))
        }
    }

    // Provider offerings
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
            let provider_key = entry.key.clone();
            let provider_offerings = offering_payload
                .deserialize_offerings(&provider_key)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize offering: {}", e))?;

            // Store each offering as a fully structured record
            for offering in &provider_offerings.server_offerings {
                let currency = offering.currency.to_string();
                let visibility = offering.visibility.to_string();
                let product_type = offering.product_type.to_string();
                let virtualization_type =
                    offering.virtualization_type.as_ref().map(|t| t.to_string());
                let billing_interval = offering.billing_interval.to_string();
                let stock = offering.stock.to_string();
                let memory_error_correction = offering
                    .memory_error_correction
                    .as_ref()
                    .map(|e| e.to_string());
                let datacenter_latitude = offering.datacenter_coordinates.map(|c| c.0);
                let datacenter_longitude = offering.datacenter_coordinates.map(|c| c.1);
                let unmetered_bandwidth = !offering.unmetered.is_empty();
                let min_contract_hours: Option<i64> = Some(1);
                let max_contract_hours: Option<i64> = None;
                let payment_methods = Self::vec_to_csv(&offering.payment_methods);
                let features = Self::vec_to_csv(&offering.features);
                let operating_systems = Self::vec_to_csv(&offering.operating_systems);
                let created_at_ns = entry.block_timestamp_ns as i64;
                let offering_identifier = offering.unique_internal_identifier.clone();
                let offer_name = offering.offer_name.clone();
                let description = offering.description.clone();
                let product_page_url = offering.product_page_url.clone();
                let processor_brand = offering.processor_brand.clone();
                let processor_speed = offering.processor_speed.clone();
                let processor_name = offering.processor_name.clone();
                let memory_type = offering.memory_type.clone();
                let memory_amount = offering.memory_amount.clone();
                let total_hdd_capacity = offering.total_hdd_capacity.clone();
                let total_ssd_capacity = offering.total_ssd_capacity.clone();
                let uplink_speed = offering.uplink_speed.clone();
                let datacenter_country = offering.datacenter_country.clone();
                let datacenter_city = offering.datacenter_city.clone();
                let control_panel = offering.control_panel.clone();
                let gpu_name = offering.gpu_name.clone();

                // Insert main offering record
                let _offering_id = sqlx::query_scalar!(
                    r#"INSERT INTO provider_offerings (
                        pubkey, offering_id, offer_name, description, product_page_url,
                        currency, monthly_price, setup_fee, visibility, product_type,
                        virtualization_type, billing_interval, stock_status, processor_brand,
                        processor_amount, processor_cores, processor_speed, processor_name,
                        memory_error_correction, memory_type, memory_amount, hdd_amount,
                        total_hdd_capacity, ssd_amount, total_ssd_capacity, unmetered_bandwidth,
                        uplink_speed, traffic, datacenter_country, datacenter_city,
                        datacenter_latitude, datacenter_longitude, control_panel, gpu_name,
                        min_contract_hours, max_contract_hours, payment_methods, features,
                        operating_systems, created_at_ns
                    ) VALUES (
                        ?, ?, ?, ?, ?,
                        ?, ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?, ?, ?
                    )
                    RETURNING id"#,
                    provider_key,
                    offering_identifier,
                    offer_name,
                    description,
                    product_page_url,
                    currency,
                    offering.monthly_price,
                    offering.setup_fee,
                    visibility,
                    product_type,
                    virtualization_type,
                    billing_interval,
                    stock,
                    processor_brand,
                    offering.processor_amount,
                    offering.processor_cores,
                    processor_speed,
                    processor_name,
                    memory_error_correction,
                    memory_type,
                    memory_amount,
                    offering.hdd_amount,
                    total_hdd_capacity,
                    offering.ssd_amount,
                    total_ssd_capacity,
                    unmetered_bandwidth,
                    uplink_speed,
                    offering.traffic,
                    datacenter_country,
                    datacenter_city,
                    datacenter_latitude,
                    datacenter_longitude,
                    control_panel,
                    gpu_name,
                    min_contract_hours,
                    max_contract_hours,
                    payment_methods,
                    features,
                    operating_systems,
                    created_at_ns
                )
                .fetch_one(&mut **tx)
                .await?;
            }
        }
        Ok(())
    }

    /// Import offerings from CSV data
    /// Returns (success_count, errors) where errors is Vec<(row_number, error_message)>
    pub async fn import_offerings_csv(
        &self,
        pubkey: &[u8],
        csv_data: &str,
        upsert: bool,
    ) -> Result<(usize, Vec<(usize, String)>)> {
        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let mut success_count = 0;
        let mut errors = Vec::new();

        for (row_idx, result) in reader.records().enumerate() {
            let row_number = row_idx + 2; // +2 because row 1 is header, 0-indexed

            match result {
                Ok(record) => {
                    match Self::parse_csv_record(&record) {
                        Ok(params) => {
                            let result: Result<()> = if upsert {
                                // Try to find existing offering by offering_id
                                let existing_offering_id = &params.offering_id;
                                match sqlx::query_scalar!(
                                    r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = ? AND (pubkey) = ?"#,
                                    existing_offering_id,
                                    pubkey
                                )
                                .fetch_optional(&self.pool)
                                .await {
                                    Ok(Some(id)) => self.update_offering(pubkey, id, params).await.map(|_| ()),
                                    Ok(None) => self.create_offering(pubkey, params).await.map(|_| ()),
                                    Err(e) => Err(anyhow::Error::from(e)),
                                }
                            } else {
                                self.create_offering(pubkey, params).await.map(|_| ())
                            };

                            match result {
                                Ok(_) => success_count += 1,
                                Err(e) => errors.push((row_number, e.to_string())),
                            }
                        }
                        Err(e) => errors.push((row_number, e)),
                    }
                }
                Err(e) => errors.push((row_number, format!("CSV parse error: {}", e))),
            }
        }

        Ok((success_count, errors))
    }

    /// Parse a single CSV record into Offering
    fn parse_csv_record(record: &csv::StringRecord) -> Result<Offering, String> {
        if record.len() < 38 {
            return Err(format!(
                "Expected at least 38 columns, found {}",
                record.len()
            ));
        }

        let get_str = |idx: usize| record.get(idx).unwrap_or("").to_string();
        let get_opt_str = |idx: usize| {
            let val = record.get(idx).unwrap_or("").trim();
            if val.is_empty() {
                None
            } else {
                Some(val.to_string())
            }
        };
        let get_opt_i64 = |idx: usize| {
            record.get(idx).and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.parse::<i64>().ok()
                }
            })
        };
        let get_opt_f64 = |idx: usize| {
            record.get(idx).and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.parse::<f64>().ok()
                }
            })
        };
        let get_f64 = |idx: usize| -> Result<f64, String> {
            record
                .get(idx)
                .ok_or_else(|| format!("Missing column {}", idx))?
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("Invalid number at column {}", idx))
        };
        let get_bool = |idx: usize| {
            record
                .get(idx)
                .map(|s| {
                    let lower = s.trim().to_lowercase();
                    lower == "true" || lower == "1" || lower == "yes"
                })
                .unwrap_or(false)
        };
        let get_opt_csv = |idx: usize| -> Option<String> {
            record.get(idx).and_then(|s| {
                let items: Vec<&str> = s
                    .split(',')
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .collect();
                if items.is_empty() {
                    None
                } else {
                    Some(items.join(","))
                }
            })
        };

        // Required fields validation
        let offering_id = get_str(0);
        let offer_name = get_str(1);

        if offering_id.trim().is_empty() {
            return Err("offering_id is required".to_string());
        }
        if offer_name.trim().is_empty() {
            return Err("offer_name is required".to_string());
        }

        Ok(Offering {
            id: None,
            pubkey: vec![], // Will be set by caller
            offering_id,
            offer_name,
            description: get_opt_str(2),
            product_page_url: get_opt_str(3),
            currency: get_str(4),
            monthly_price: get_f64(5)?,
            setup_fee: get_f64(6)?,
            visibility: get_str(7),
            product_type: get_str(8),
            virtualization_type: get_opt_str(9),
            billing_interval: get_str(10),
            stock_status: get_str(11),
            processor_brand: get_opt_str(12),
            processor_amount: get_opt_i64(13),
            processor_cores: get_opt_i64(14),
            processor_speed: get_opt_str(15),
            processor_name: get_opt_str(16),
            memory_error_correction: get_opt_str(17),
            memory_type: get_opt_str(18),
            memory_amount: get_opt_str(19),
            hdd_amount: get_opt_i64(20),
            total_hdd_capacity: get_opt_str(21),
            ssd_amount: get_opt_i64(22),
            total_ssd_capacity: get_opt_str(23),
            unmetered_bandwidth: get_bool(24),
            uplink_speed: get_opt_str(25),
            traffic: get_opt_i64(26),
            datacenter_country: get_str(27),
            datacenter_city: get_str(28),
            datacenter_latitude: get_opt_f64(29),
            datacenter_longitude: get_opt_f64(30),
            control_panel: get_opt_str(31),
            gpu_name: get_opt_str(32),
            min_contract_hours: get_opt_i64(33),
            max_contract_hours: get_opt_i64(34),
            payment_methods: get_opt_csv(35),
            features: get_opt_csv(36),
            operating_systems: get_opt_csv(37),
        })
    }
}

#[cfg(test)]
mod tests;
