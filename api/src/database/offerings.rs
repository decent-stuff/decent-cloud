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
    pub payment_methods: Option<String>,
    pub features: Option<String>,
    pub operating_systems: Option<String>,
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
    pub payment_methods: Option<String>,
    pub features: Option<String>,
    pub operating_systems: Option<String>,
}

#[allow(dead_code)]
impl Database {
    /// Search offerings with filters
    pub async fn search_offerings(
        &self,
        params: SearchOfferingsParams<'_>,
    ) -> Result<Vec<Offering>> {
        let mut query =
            String::from("SELECT * FROM provider_offerings WHERE visibility != 'example'");

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

    /// Get example offerings for CSV template generation
    pub async fn get_example_offerings(&self) -> Result<Vec<Offering>> {
        // Use the same distinctive hash as in migration 002
        let example_pubkey_hash =
            hex::decode("6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572")
                .unwrap();
        let offerings = sqlx::query_as::<_, Offering>(
            "SELECT * FROM provider_offerings WHERE pubkey_hash = ? ORDER BY offering_id ASC",
        )
        .bind(example_pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(offerings)
    }

    /// Count offerings
    pub async fn count_offerings(&self, filters: Option<&str>) -> Result<i64> {
        let query = if let Some(f) = filters {
            format!(
                "SELECT COUNT(*) FROM provider_offerings WHERE visibility != 'example' AND ({})",
                f
            )
        } else {
            "SELECT COUNT(*) FROM provider_offerings WHERE visibility != 'example'".to_string()
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
                max_contract_hours, payment_methods, features, operating_systems, created_at_ns
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(&params.payment_methods)
        .bind(&params.features)
        .bind(&params.operating_systems)
        .bind(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
        .fetch_one(&mut *tx)
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
                min_contract_hours = ?, max_contract_hours = ?,
                payment_methods = ?, features = ?, operating_systems = ?
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
        .bind(&params.payment_methods)
        .bind(&params.features)
        .bind(&params.operating_systems)
        .bind(offering_db_id)
        .execute(&mut *tx)
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

        // Get metadata directly from source offering

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
            payment_methods: source.payment_methods,
            features: source.features,
            operating_systems: source.operating_systems,
        };

        self.create_offering(pubkey_hash, params).await
    }

    /// Bulk update stock_status for multiple offerings
    pub async fn bulk_update_stock_status(
        &self,
        pubkey_hash: &[u8],
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
            "SELECT COUNT(*) as count FROM provider_offerings WHERE id IN ({}) AND pubkey_hash = ?",
            placeholders
        );

        let mut query_builder = sqlx::query_scalar::<_, i64>(&verify_query);
        for id in offering_ids {
            query_builder = query_builder.bind(id);
        }
        query_builder = query_builder.bind(pubkey_hash);

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

    // Helper function to calculate pricing from monthly price
    fn calculate_pricing(monthly_price: f64) -> (i64, i64) {
        let price_per_hour_e9s =
            (monthly_price / 30.0 / 24.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
        let price_per_day_e9s = (monthly_price / 30.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
        (price_per_hour_e9s, price_per_day_e9s)
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
            let provider_key = &entry.key;
            let provider_offerings = offering_payload
                .deserialize_offerings(provider_key)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize offering: {}", e))?;

            // Store each offering as a fully structured record
            for offering in &provider_offerings.server_offerings {
                let (price_per_hour_e9s, price_per_day_e9s) =
                    Self::calculate_pricing(offering.monthly_price);

                // Insert main offering record
                let _offering_id = sqlx::query_scalar::<_, i64>(
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
                        max_contract_hours, payment_methods, features, operating_systems, created_at_ns
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                .bind({
                    if offering.payment_methods.is_empty() {
                        None
                    } else {
                        Some(offering.payment_methods.join(","))
                    }
                })
                .bind({
                    if offering.features.is_empty() {
                        None
                    } else {
                        Some(offering.features.join(","))
                    }
                })
                .bind({
                    if offering.operating_systems.is_empty() {
                        None
                    } else {
                        Some(offering.operating_systems.join(","))
                    }
                })
                .bind(entry.block_timestamp_ns as i64)
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
        pubkey_hash: &[u8],
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
                            let result = if upsert {
                                // Try to find existing offering by offering_id
                                let existing = sqlx::query_scalar::<_, i64>(
                                    "SELECT id FROM provider_offerings WHERE offering_id = ? AND pubkey_hash = ?"
                                )
                                .bind(&params.offering_id)
                                .bind(pubkey_hash)
                                .fetch_optional(&self.pool)
                                .await;

                                match existing {
                                    Ok(Some(id)) => {
                                        self.update_offering(pubkey_hash, id, params).await
                                    }
                                    Ok(None) => {
                                        self.create_offering(pubkey_hash, params).await.map(|_| ())
                                    }
                                    Err(e) => Err(e.into()),
                                }
                            } else {
                                self.create_offering(pubkey_hash, params).await.map(|_| ())
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

    /// Parse a single CSV record into CreateOfferingParams
    fn parse_csv_record(record: &csv::StringRecord) -> Result<CreateOfferingParams, String> {
        if record.len() < 35 {
            return Err(format!(
                "Expected at least 35 columns, found {}",
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
        let get_array = |idx: usize| -> Vec<String> {
            record
                .get(idx)
                .map(|s| {
                    s.split(',')
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect()
                })
                .unwrap_or_default()
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

        Ok(CreateOfferingParams {
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
            payment_methods: {
                let arr = get_array(35);
                if arr.is_empty() {
                    None
                } else {
                    Some(arr.join(","))
                }
            },
            features: {
                let arr = get_array(36);
                if arr.is_empty() {
                    None
                } else {
                    Some(arr.join(","))
                }
            },
            operating_systems: {
                let arr = get_array(37);
                if arr.is_empty() {
                    None
                } else {
                    Some(arr.join(","))
                }
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> Database {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let migration1_sql = include_str!("../../migrations/001_original_schema.sql");
        sqlx::query(migration1_sql).execute(&pool).await.unwrap();
        let migration2_sql = include_str!("../../migrations/002_add_example_offerings.sql");
        sqlx::query(migration2_sql).execute(&pool).await.unwrap();
        let migration3_sql = include_str!("../../migrations/003_simplify_offering_metadata.sql");
        sqlx::query(migration3_sql).execute(&pool).await.unwrap();
        Database { pool }
    }

    async fn insert_test_offering(
        db: &Database,
        id: i64,
        pubkey: &[u8],
        country: &str,
        price: f64,
    ) {
        // Use IDs starting from 100 to avoid conflicts with example data from migration 002
        let db_id = id + 100;
        let offering_id = format!("off-{}", id);
        // Calculate price_per_hour_e9s from monthly price (rough approximation)
        let price_per_hour_e9s = (price * 1_000_000_000.0 / 30.0 / 24.0) as i64;
        sqlx::query("INSERT INTO provider_offerings (id, pubkey_hash, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, price_per_hour_e9s, payment_methods, features, operating_systems, created_at_ns) VALUES (?, ?, ?, 'Test Offer', 'USD', ?, 0, 'public', 'compute', 'monthly', 'in_stock', ?, 'City', 0, ?, NULL, NULL, NULL, 0)")
            .bind(db_id).bind(pubkey).bind(&offering_id).bind(price).bind(country).bind(price_per_hour_e9s).execute(&db.pool).await.unwrap();
    }

    // Helper to get the database ID from test ID (test IDs start from 1, DB IDs from 100)
    fn test_id_to_db_id(test_id: i64) -> i64 {
        test_id + 100
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

        let db_id = test_id_to_db_id(42);
        let offering = db.get_offering(db_id).await.unwrap();
        assert!(offering.is_some());
        assert_eq!(offering.unwrap().id, db_id);
    }

    #[tokio::test]
    async fn test_get_offering_not_found() {
        let db = setup_test_db().await;
        let offering = db.get_offering(999).await.unwrap();
        assert!(offering.is_none());
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
            payment_methods: Some("BTC,ETH".to_string()),
            features: Some("RAID,Backup".to_string()),
            operating_systems: Some("Ubuntu 22.04".to_string()),
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
        let methods: Vec<&str> = offering
            .payment_methods
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"BTC"));

        let features: Vec<&str> = offering
            .features
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(features.len(), 2);

        let oses: Vec<&str> = offering
            .operating_systems
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .collect();
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
            payment_methods: None,
            features: None,
            operating_systems: None,
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
            payment_methods: None,
            features: None,
            operating_systems: None,
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
            payment_methods: Some("ETH".to_string()),
            features: Some("Backup".to_string()),
            operating_systems: Some("Debian 12".to_string()),
        };

        let db_id = test_id_to_db_id(1);
        let result = db.update_offering(&pubkey, db_id, update_params).await;
        assert!(result.is_ok());

        // Verify update
        let offering = db.get_offering(db_id).await.unwrap().unwrap();
        assert_eq!(offering.offer_name, "Updated Server");
        assert_eq!(offering.monthly_price, 199.99);
        assert_eq!(offering.currency, "EUR");
        assert_eq!(offering.payment_methods, Some("ETH".to_string()));
        assert_eq!(offering.features, Some("Backup".to_string()));
        assert_eq!(offering.operating_systems, Some("Debian 12".to_string()));
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
            payment_methods: None,
            features: None,
            operating_systems: None,
        };

        let db_id = test_id_to_db_id(1);
        let result = db.update_offering(&pubkey2, db_id, params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_delete_offering_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;

        let db_id = test_id_to_db_id(1);
        let result = db.delete_offering(&pubkey, db_id).await;
        assert!(result.is_ok());

        // Verify deletion
        let offering = db.get_offering(db_id).await.unwrap();
        assert!(offering.is_none());
    }

    #[tokio::test]
    async fn test_delete_offering_unauthorized() {
        let db = setup_test_db().await;
        let pubkey1 = vec![1u8; 32];
        let pubkey2 = vec![2u8; 32];

        insert_test_offering(&db, 1, &pubkey1, "US", 100.0).await;

        let db_id = test_id_to_db_id(1);
        let result = db.delete_offering(&pubkey2, db_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_duplicate_offering_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        // Create offering with payment_methods
        let db_id = test_id_to_db_id(1);
        let offering_id = "off-1".to_string();
        let price_per_hour_e9s = (100.0 * 1_000_000_000.0 / 30.0 / 24.0) as i64;
        sqlx::query("INSERT INTO provider_offerings (id, pubkey_hash, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, price_per_hour_e9s, payment_methods, features, operating_systems, created_at_ns) VALUES (?, ?, ?, 'Test Offer', 'USD', ?, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, ?, 'BTC', NULL, NULL, 0)")
            .bind(db_id).bind(&pubkey).bind(&offering_id).bind(100.0).bind(price_per_hour_e9s).execute(&db.pool).await.unwrap();

        let new_id = db
            .duplicate_offering(&pubkey, db_id, "off-1-copy".to_string())
            .await
            .unwrap();

        assert!(new_id > db_id);

        // Verify duplication
        let duplicated = db.get_offering(new_id).await.unwrap().unwrap();
        assert_eq!(duplicated.offer_name, "Test Offer (Copy)");
        assert_eq!(duplicated.monthly_price, 100.0);
        assert_eq!(duplicated.datacenter_country, "US");

        // Verify metadata was duplicated
        let methods: Vec<&str> = duplicated
            .payment_methods
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0], "BTC");
    }

    #[tokio::test]
    async fn test_duplicate_offering_unauthorized() {
        let db = setup_test_db().await;
        let pubkey1 = vec![1u8; 32];
        let pubkey2 = vec![2u8; 32];

        insert_test_offering(&db, 1, &pubkey1, "US", 100.0).await;

        let db_id = test_id_to_db_id(1);
        let result = db
            .duplicate_offering(&pubkey2, db_id, "copy".to_string())
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));
    }

    #[tokio::test]
    async fn test_bulk_update_stock_status_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        // Create 3 offerings
        insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;
        insert_test_offering(&db, 2, &pubkey, "US", 200.0).await;
        insert_test_offering(&db, 3, &pubkey, "US", 300.0).await;

        // Bulk update status
        let test_ids = [1, 2, 3];
        let offering_ids: Vec<i64> = test_ids.iter().map(|&id| test_id_to_db_id(id)).collect();
        let result = db
            .bulk_update_stock_status(&pubkey, &offering_ids, "out_of_stock")
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);

        // Verify all updated
        for id in offering_ids {
            let offering = db.get_offering(id).await.unwrap().unwrap();
            assert_eq!(offering.stock_status, "out_of_stock");
        }
    }

    #[tokio::test]
    async fn test_bulk_update_stock_status_unauthorized() {
        let db = setup_test_db().await;
        let pubkey1 = vec![1u8; 32];
        let pubkey2 = vec![2u8; 32];

        // Create offerings with pubkey1
        insert_test_offering(&db, 1, &pubkey1, "US", 100.0).await;
        insert_test_offering(&db, 2, &pubkey1, "US", 200.0).await;

        // Try to update with pubkey2
        let test_ids = [1, 2];
        let offering_ids: Vec<i64> = test_ids.iter().map(|&id| test_id_to_db_id(id)).collect();
        let result = db
            .bulk_update_stock_status(&pubkey2, &offering_ids, "out_of_stock")
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not all offerings belong to this provider"));
    }

    #[tokio::test]
    async fn test_bulk_update_stock_status_empty() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        let result = db
            .bulk_update_stock_status(&pubkey, &[], "out_of_stock")
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_csv_import_success() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        let csv_data = "offering_id,offer_name,description,product_page_url,currency,monthly_price,setup_fee,visibility,product_type,virtualization_type,billing_interval,stock_status,processor_brand,processor_amount,processor_cores,processor_speed,processor_name,memory_error_correction,memory_type,memory_amount,hdd_amount,total_hdd_capacity,ssd_amount,total_ssd_capacity,unmetered_bandwidth,uplink_speed,traffic,datacenter_country,datacenter_city,datacenter_latitude,datacenter_longitude,control_panel,gpu_name,min_contract_hours,max_contract_hours,payment_methods,features,operating_systems
off-1,Test Server,Great server,https://example.com,USD,100.0,0.0,public,dedicated,,monthly,in_stock,Intel,2,8,3.5GHz,Xeon,ECC,DDR4,32GB,2,2TB,1,500GB,true,1Gbps,10000,US,New York,40.7128,-74.0060,cPanel,RTX 3090,1,720,BTC,SSD,Ubuntu
off-2,Test Server 2,Another server,,EUR,200.0,50.0,public,vps,kvm,monthly,in_stock,,,,,,,,,,,,,false,,,DE,Berlin,,,,,,,\"BTC,ETH\",\"SSD,NVMe\",\"Ubuntu,Debian\"";

        let (success_count, errors) = db
            .import_offerings_csv(&pubkey, csv_data, false)
            .await
            .unwrap();

        assert_eq!(success_count, 2);
        assert_eq!(errors.len(), 0);

        // Verify first offering
        let off1 =
            sqlx::query_scalar::<_, i64>("SELECT id FROM provider_offerings WHERE offering_id = ?")
                .bind("off-1")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        let offering = db.get_offering(off1).await.unwrap().unwrap();
        assert_eq!(offering.offer_name, "Test Server");
        assert_eq!(offering.monthly_price, 100.0);
        assert_eq!(offering.datacenter_country, "US");

        // Verify metadata
        let methods: Vec<&str> = offering
            .payment_methods
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0], "BTC");

        let features: Vec<&str> = offering
            .features
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0], "SSD");

        let os: Vec<&str> = offering
            .operating_systems
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(os.len(), 1);
        assert_eq!(os[0], "Ubuntu");
    }

    #[tokio::test]
    async fn test_csv_import_with_errors() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        let csv_data = "offering_id,offer_name,description,product_page_url,currency,monthly_price,setup_fee,visibility,product_type,virtualization_type,billing_interval,stock_status,processor_brand,processor_amount,processor_cores,processor_speed,processor_name,memory_error_correction,memory_type,memory_amount,hdd_amount,total_hdd_capacity,ssd_amount,total_ssd_capacity,unmetered_bandwidth,uplink_speed,traffic,datacenter_country,datacenter_city,datacenter_latitude,datacenter_longitude,control_panel,gpu_name,min_contract_hours,max_contract_hours,payment_methods,features,operating_systems
off-1,Test Server,desc,,USD,100.0,0.0,public,dedicated,,monthly,in_stock,,,,,,,,,,,,,false,,,US,NYC,,,,,,,,,
,Missing ID,desc,,USD,100.0,0.0,public,dedicated,,monthly,in_stock,,,,,,,,,,,,,false,,,US,NYC,,,,,,,,,
off-3,,desc,,USD,100.0,0.0,public,dedicated,,monthly,in_stock,,,,,,,,,,,,,false,,,US,NYC,,,,,,,,,
off-4,Bad Price,desc,,USD,invalid,0.0,public,dedicated,,monthly,in_stock,,,,,,,,,,,,,false,,,US,NYC,,,,,,,,,";

        let (success_count, errors) = db
            .import_offerings_csv(&pubkey, csv_data, false)
            .await
            .unwrap();

        assert_eq!(success_count, 1);
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].0, 3);
        assert!(errors[0].1.contains("offering_id is required"));
        assert_eq!(errors[1].0, 4);
        assert!(errors[1].1.contains("offer_name is required"));
        assert_eq!(errors[2].0, 5);
        assert!(errors[2].1.contains("Invalid number"));
    }

    #[tokio::test]
    async fn test_csv_import_upsert() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        // Insert initial offering
        insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;

        let csv_data = "offering_id,offer_name,description,product_page_url,currency,monthly_price,setup_fee,visibility,product_type,virtualization_type,billing_interval,stock_status,processor_brand,processor_amount,processor_cores,processor_speed,processor_name,memory_error_correction,memory_type,memory_amount,hdd_amount,total_hdd_capacity,ssd_amount,total_ssd_capacity,unmetered_bandwidth,uplink_speed,traffic,datacenter_country,datacenter_city,datacenter_latitude,datacenter_longitude,control_panel,gpu_name,min_contract_hours,max_contract_hours,payment_methods,features,operating_systems
off-1,Updated Offer,Updated desc,,USD,200.0,10.0,public,dedicated,,monthly,out_of_stock,,,,,,,,,,,,,false,,,US,NYC,,,,,,,,,
off-2,New Offer,New desc,,EUR,150.0,0.0,public,vps,,monthly,in_stock,,,,,,,,,,,,,false,,,DE,Berlin,,,,,,,,,";

        let (success_count, errors) = db
            .import_offerings_csv(&pubkey, csv_data, true)
            .await
            .unwrap();

        assert_eq!(success_count, 2);
        assert_eq!(errors.len(), 0);

        // Verify update
        let db_id = test_id_to_db_id(1);
        let offering = db.get_offering(db_id).await.unwrap().unwrap();
        assert_eq!(offering.offer_name, "Updated Offer");
        assert_eq!(offering.monthly_price, 200.0);
        assert_eq!(offering.stock_status, "out_of_stock");

        // Verify new offering was created
        let off2 =
            sqlx::query_scalar::<_, i64>("SELECT id FROM provider_offerings WHERE offering_id = ?")
                .bind("off-2")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert!(off2 > db_id);
    }

    #[tokio::test]
    async fn test_csv_import_unauthorized() {
        let db = setup_test_db().await;
        let pubkey1 = vec![1u8; 32];
        let pubkey2 = vec![2u8; 32];

        // Create offering for pubkey1
        insert_test_offering(&db, 1, &pubkey1, "US", 100.0).await;

        // Try to upsert with pubkey2
        let csv_data = "offering_id,offer_name,description,product_page_url,currency,monthly_price,setup_fee,visibility,product_type,virtualization_type,billing_interval,stock_status,processor_brand,processor_amount,processor_cores,processor_speed,processor_name,memory_error_correction,memory_type,memory_amount,hdd_amount,total_hdd_capacity,ssd_amount,total_ssd_capacity,unmetered_bandwidth,uplink_speed,traffic,datacenter_country,datacenter_city,datacenter_latitude,datacenter_longitude,control_panel,gpu_name,min_contract_hours,max_contract_hours,payment_methods,features,operating_systems
off-1,Hacked,Unauthorized update,,USD,1.0,0.0,public,dedicated,,monthly,in_stock,,,,,,,,,,,,,false,,,US,NYC,,,,,,,,,";

        let (success_count, errors) = db
            .import_offerings_csv(&pubkey2, csv_data, true)
            .await
            .unwrap();

        // Should create new offering for pubkey2, not update pubkey1's offering
        assert_eq!(success_count, 1);
        assert_eq!(errors.len(), 0);

        // Verify original offering unchanged
        let db_id = test_id_to_db_id(1);
        let original = db.get_offering(db_id).await.unwrap().unwrap();
        assert_eq!(original.offer_name, "Test Offer");
        assert_eq!(original.monthly_price, 100.0);
    }
}
