use super::types::Database;
use crate::regions::{country_to_region, is_valid_country_code};
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    pub pubkey: String,
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
    // Usage-based billing fields
    pub billing_unit: String,          // 'minute', 'hour', 'day', 'month'
    pub pricing_model: Option<String>, // 'flat', 'usage_overage'
    #[ts(type = "number | undefined")]
    pub price_per_unit: Option<f64>,
    #[ts(type = "number | undefined")]
    pub included_units: Option<i64>,
    #[ts(type = "number | undefined")]
    pub overage_price_per_unit: Option<f64>,
    pub stripe_metered_price_id: Option<String>,
    // Subscription billing fields
    #[ts(type = "boolean")]
    #[sqlx(default)]
    pub is_subscription: bool,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub subscription_interval_days: Option<i64>,
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
    pub gpu_count: Option<i64>,
    #[ts(type = "number | undefined")]
    pub gpu_memory_gb: Option<i64>,
    #[ts(type = "number | undefined")]
    pub min_contract_hours: Option<i64>,
    #[ts(type = "number | undefined")]
    pub max_contract_hours: Option<i64>,
    pub payment_methods: Option<String>,
    pub features: Option<String>,
    pub operating_systems: Option<String>,
    // Trust fields - populated only in search results (from provider_profiles)
    #[ts(type = "number | undefined")]
    pub trust_score: Option<i64>,
    #[ts(type = "boolean | undefined")]
    #[sqlx(default)]
    pub has_critical_flags: Option<bool>,
    // Example flag - indicates if this is an example offering
    #[ts(type = "boolean")]
    #[sqlx(default)]
    pub is_example: bool,
    // Source of offering data: 'provider' (normal) or 'seeded' (scraped/curated)
    #[ts(type = "string | undefined")]
    #[sqlx(default)]
    pub offering_source: Option<String>,
    // External checkout URL for seeded offerings
    pub external_checkout_url: Option<String>,
    // Reseller information (if offering has an active reseller)
    #[ts(type = "string | undefined")]
    #[sqlx(default)]
    pub reseller_name: Option<String>,
    #[ts(type = "number | undefined")]
    #[sqlx(default)]
    pub reseller_commission_percent: Option<i64>,
    // Owner username from account_profiles (if they have an account)
    #[ts(type = "string | undefined")]
    #[sqlx(default)]
    pub owner_username: Option<String>,
    // Per-offering provisioner configuration
    // NULL = use agent's default provisioner
    pub provisioner_type: Option<String>,
    pub provisioner_config: Option<String>,
    // Agent pool override - if set, only agents in this pool can provision
    // NULL = auto-match by location
    pub agent_pool_id: Option<String>,
    // Provider agent online status (from provider_agent_status table)
    #[ts(type = "boolean | undefined")]
    #[sqlx(default)]
    pub provider_online: Option<bool>,
    // Resolved pool ID - computed from agent_pool_id or location matching
    #[ts(type = "string | undefined")]
    #[sqlx(default)]
    pub resolved_pool_id: Option<String>,
    // Resolved pool name - for display purposes
    #[ts(type = "string | undefined")]
    #[sqlx(default)]
    pub resolved_pool_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchOfferingsParams<'a> {
    pub product_type: Option<&'a str>,
    pub country: Option<&'a str>,
    pub in_stock_only: bool,
    pub min_price_monthly: Option<f64>,
    pub max_price_monthly: Option<f64>,
    pub limit: i64,
    pub offset: i64,
}

#[allow(dead_code)]
impl Database {
    /// Search offerings with filters.
    /// Excludes offerings that don't have a matching agent pool.
    pub async fn search_offerings(
        &self,
        params: SearchOfferingsParams<'_>,
    ) -> Result<Vec<Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let five_mins_ns = 5i64 * 60 * 1_000_000_000;
        let heartbeat_cutoff = now_ns - five_mins_ns;
        let mut query = String::from(
            "SELECT o.id, lower(encode(o.pubkey, 'hex')) as pubkey, o.offering_id, o.offer_name, o.description, o.product_page_url, o.currency, o.monthly_price, o.setup_fee, o.visibility, o.product_type, o.virtualization_type, o.billing_interval, o.billing_unit, o.pricing_model, o.price_per_unit, o.included_units, o.overage_price_per_unit, o.stripe_metered_price_id, o.is_subscription, o.subscription_interval_days, o.stock_status, o.processor_brand, o.processor_amount, o.processor_cores, o.processor_speed, o.processor_name, o.memory_error_correction, o.memory_type, o.memory_amount, o.hdd_amount, o.total_hdd_capacity, o.ssd_amount, o.total_ssd_capacity, o.unmetered_bandwidth, o.uplink_speed, o.traffic, o.datacenter_country, o.datacenter_city, o.datacenter_latitude, o.datacenter_longitude, o.control_panel, o.gpu_name, o.gpu_count, o.gpu_memory_gb, o.min_contract_hours, o.max_contract_hours, o.payment_methods, o.features, o.operating_systems, p.trust_score, CASE WHEN p.pubkey IS NULL THEN NULL WHEN p.has_critical_flags = 1 THEN 1 ELSE 0 END as has_critical_flags, CASE WHEN lower(encode(o.pubkey, 'hex')) = $1 THEN 1 ELSE 0 END as is_example, o.offering_source, o.external_checkout_url, rp.name as reseller_name, rr.commission_percent as reseller_commission_percent, acc.username as owner_username, o.provisioner_type, o.provisioner_config, o.agent_pool_id, COALESCE(pas.online = 1 AND pas.last_heartbeat_ns > $2, 0) as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name FROM provider_offerings o LEFT JOIN provider_profiles p ON o.pubkey = p.pubkey LEFT JOIN reseller_relationships rr ON o.pubkey = rr.external_provider_pubkey AND rr.status = 'active' LEFT JOIN provider_profiles rp ON rr.reseller_pubkey = rp.pubkey LEFT JOIN account_public_keys apk ON o.pubkey = apk.public_key AND apk.is_active = 1 LEFT JOIN accounts acc ON apk.account_id = acc.id LEFT JOIN provider_agent_status pas ON o.pubkey = pas.provider_pubkey WHERE LOWER(o.visibility) = 'public'"
        );

        // Track placeholder index (starts at 3 since $1 and $2 are already used)
        let mut idx = 2;

        if params.product_type.is_some() {
            idx += 1;
            query.push_str(&format!(" AND o.product_type = ${}", idx));
        }
        if params.country.is_some() {
            idx += 1;
            query.push_str(&format!(" AND o.datacenter_country = ${}", idx));
        }
        if params.in_stock_only {
            idx += 1;
            query.push_str(&format!(" AND o.stock_status = ${}", idx));
        }
        if params.min_price_monthly.is_some() {
            idx += 1;
            query.push_str(&format!(" AND o.monthly_price >= ${}", idx));
        }
        if params.max_price_monthly.is_some() {
            idx += 1;
            query.push_str(&format!(" AND o.monthly_price <= ${}", idx));
        }

        idx += 1;
        let limit_idx = idx;
        idx += 1;
        let offset_idx = idx;
        query.push_str(&format!(" ORDER BY o.monthly_price ASC LIMIT ${} OFFSET ${}", limit_idx, offset_idx));

        let mut query_builder = sqlx::query_as::<_, Offering>(&query)
            .bind(example_provider_pubkey)
            .bind(heartbeat_cutoff);

        if let Some(pt) = params.product_type {
            query_builder = query_builder.bind(pt);
        }
        if let Some(c) = params.country {
            query_builder = query_builder.bind(c);
        }
        if params.in_stock_only {
            query_builder = query_builder.bind("in_stock");
        }
        if let Some(min_price) = params.min_price_monthly {
            query_builder = query_builder.bind(min_price);
        }
        if let Some(max_price) = params.max_price_monthly {
            query_builder = query_builder.bind(max_price);
        }

        // Fetch 3x the limit to account for filtering (offerings without pools)
        // This maintains pagination while filtering out offerings without matching pools
        let fetch_limit = params.limit * 3;
        let offerings = query_builder
            .bind(fetch_limit)
            .bind(params.offset)
            .fetch_all(&self.pool)
            .await?;

        // Compute online status for all offerings
        let with_status = self.compute_provider_online_status(offerings).await?;

        // Filter to only include offerings that have a matching pool
        // Offerings without pools have provider_online = Some(false) due to has_pool = false
        let filtered: Vec<Offering> = with_status
            .into_iter()
            .filter(|o| {
                // Include if provider_online is Some(true) OR Some(false) where the pool exists but is offline
                // Exclude if provider_online is Some(false) and has_pool was false (no pool)
                // We can detect "no pool" by checking if resolved_pool_id is None
                o.resolved_pool_id.is_some()
            })
            .take(params.limit as usize)
            .collect();

        Ok(filtered)
    }

    /// Compute provider_online status and resolved pool info for offerings.
    /// Sets provider_online based on whether the offering's pool has online agents.
    /// Also sets resolved_pool_id and resolved_pool_name.
    /// This is done in Rust because country_to_region mapping can't be done in SQL.
    async fn compute_provider_online_status(
        &self,
        offerings: Vec<Offering>,
    ) -> Result<Vec<Offering>> {
        // Group offerings by provider to minimize database queries
        let mut by_provider: HashMap<String, Vec<Offering>> = HashMap::new();
        for offering in offerings {
            by_provider
                .entry(offering.pubkey.clone())
                .or_default()
                .push(offering);
        }

        let mut result = Vec::new();

        // For each provider, fetch their pools and update offering online status
        for (provider_pubkey_hex, provider_offerings) in by_provider {
            // Decode hex pubkey
            let provider_pubkey = hex::decode(&provider_pubkey_hex)?;

            // Fetch all pools for this provider
            let pools = self.list_agent_pools_with_stats(&provider_pubkey).await?;

            // Build maps for efficient lookup
            let pool_ids: HashSet<String> = pools.iter().map(|p| p.pool.pool_id.clone()).collect();
            let pool_locations: HashSet<String> =
                pools.iter().map(|p| p.pool.location.clone()).collect();

            // Build map of pool_id -> (pool, online status)
            let pool_info_by_id: HashMap<String, (&super::agent_pools::AgentPoolWithStats, bool)> =
                pools
                    .iter()
                    .map(|p| (p.pool.pool_id.clone(), (p, p.online_count > 0)))
                    .collect();

            // Build map of location -> (pool, online status) (first online pool in location, or first pool if none online)
            let pool_info_by_location: HashMap<
                String,
                (&super::agent_pools::AgentPoolWithStats, bool),
            > = pools.iter().fold(HashMap::new(), |mut acc, p| {
                let is_online = p.online_count > 0;
                acc.entry(p.pool.location.clone())
                    .and_modify(|(existing_pool, existing_online)| {
                        // Prefer online pools
                        if !*existing_online && is_online {
                            *existing_pool = p;
                            *existing_online = is_online;
                        }
                    })
                    .or_insert((p, is_online));
                acc
            });

            // Update all offerings with pool-specific online status
            for mut offering in provider_offerings {
                let (has_pool, pool_is_online, resolved_pool) = if let Some(pool_id) =
                    &offering.agent_pool_id
                {
                    // Explicit pool_id - check if it exists and is online
                    let has_pool = !pool_id.is_empty() && pool_ids.contains(pool_id);
                    if let Some((pool_info, is_online)) = pool_info_by_id.get(pool_id) {
                        (has_pool, *is_online, Some(&pool_info.pool))
                    } else {
                        (false, false, None)
                    }
                } else {
                    // No explicit pool - check if location matches a pool
                    if let Some(region) = country_to_region(&offering.datacenter_country) {
                        let has_pool = pool_locations.contains(region);
                        if let Some((pool_info, is_online)) = pool_info_by_location.get(region) {
                            (has_pool, *is_online, Some(&pool_info.pool))
                        } else {
                            (false, false, None)
                        }
                    } else {
                        (false, false, None)
                    }
                };

                // Set provider_online based on whether the pool exists and has online agents
                // If no pool exists for this offering, mark as offline
                offering.provider_online = Some(has_pool && pool_is_online);

                // Set resolved pool info
                if let Some(pool) = resolved_pool {
                    offering.resolved_pool_id = Some(pool.pool_id.clone());
                    offering.resolved_pool_name = Some(pool.name.clone());
                }

                result.push(offering);
            }
        }

        // Re-sort by price to maintain original order
        result.sort_by(|a, b| {
            a.monthly_price
                .partial_cmp(&b.monthly_price)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(result)
    }

    /// Get offerings by provider with resolved pool information and online status
    pub async fn get_provider_offerings(&self, pubkey: &[u8]) -> Result<Vec<Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let offerings = sqlx::query_as::<_, Offering>(
            r#"SELECT id, lower(encode(pubkey, 'hex')) as pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval,
               billing_unit, pricing_model, price_per_unit, included_units, overage_price_per_unit, stripe_metered_price_id,
               is_subscription, subscription_interval_days,
               stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems,
               NULL as trust_score, NULL as has_critical_flags, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN 1 ELSE 0 END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, agent_pool_id, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings WHERE pubkey = $2 ORDER BY monthly_price ASC"#
        )
        .bind(example_provider_pubkey)
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        // Compute resolved pool and online status for each offering
        // This uses the same logic as marketplace to ensure consistency
        let result = self.compute_provider_online_status(offerings).await?;

        Ok(result)
    }

    /// Resolve which pool an offering maps to.
    /// Returns the pool if found, None otherwise.
    async fn resolve_pool_for_offering(
        &self,
        provider_pubkey: &[u8],
        offering: &Offering,
    ) -> Result<Option<super::agent_pools::AgentPool>> {
        // If offering has explicit agent_pool_id, use that
        if let Some(pool_id) = &offering.agent_pool_id {
            if !pool_id.is_empty() {
                return self.get_agent_pool(pool_id).await;
            }
        }

        // Otherwise, try to match by location
        if let Some(region) = country_to_region(&offering.datacenter_country) {
            return self.find_pool_by_location(provider_pubkey, region).await;
        }

        Ok(None)
    }

    /// Get single offering by id
    pub async fn get_offering(&self, offering_id: i64) -> Result<Option<Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let offering =
            sqlx::query_as::<_, Offering>(r#"SELECT id, lower(encode(pubkey, 'hex')) as pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price,
                setup_fee, visibility, product_type, virtualization_type, billing_interval,
                billing_unit, pricing_model, price_per_unit, included_units, overage_price_per_unit, stripe_metered_price_id,
                is_subscription, subscription_interval_days,
                stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
                memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems,
               NULL as trust_score, NULL as has_critical_flags, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN 1 ELSE 0 END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, agent_pool_id, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings WHERE id = $2"#)
                .bind(example_provider_pubkey)
                .bind(offering_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(offering)
    }

    /// Get example offerings for CSV template generation
    pub async fn get_example_offerings(&self) -> Result<Vec<Offering>> {
        let example_provider_pubkey = Self::example_provider_pubkey();
        let example_provider_pubkey_hex = hex::encode(&example_provider_pubkey);
        let offerings = sqlx::query_as::<_, Offering>(
            r#"SELECT id, lower(encode(pubkey, 'hex')) as pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval,
               billing_unit, pricing_model, price_per_unit, included_units, overage_price_per_unit, stripe_metered_price_id,
               is_subscription, subscription_interval_days,
               stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems,
               NULL as trust_score, NULL as has_critical_flags, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN 1 ELSE 0 END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, agent_pool_id, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings WHERE pubkey = $2 ORDER BY offering_id ASC"#
        )
        .bind(&example_provider_pubkey_hex)
        .bind(&example_provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(offerings)
    }

    /// Get example offerings filtered by product type
    pub async fn get_example_offerings_by_type(&self, product_type: &str) -> Result<Vec<Offering>> {
        let example_provider_pubkey = Self::example_provider_pubkey();
        let example_provider_pubkey_hex = hex::encode(&example_provider_pubkey);
        let offerings = sqlx::query_as::<_, Offering>(
            r#"SELECT id, lower(encode(pubkey, 'hex')) as pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval,
               billing_unit, pricing_model, price_per_unit, included_units, overage_price_per_unit, stripe_metered_price_id,
               is_subscription, subscription_interval_days,
               stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems,
               NULL as trust_score, NULL as has_critical_flags, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN 1 ELSE 0 END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, agent_pool_id, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings WHERE pubkey = $2 AND product_type = $3 ORDER BY offering_id ASC"#
        )
        .bind(&example_provider_pubkey_hex)
        .bind(&example_provider_pubkey)
        .bind(product_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(offerings)
    }

    /// Get available product types from example offerings
    pub async fn get_available_product_types(&self) -> Result<Vec<String>> {
        let example_provider_pubkey = Self::example_provider_pubkey();
        let product_types = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT product_type FROM provider_offerings WHERE pubkey = $1 ORDER BY product_type"
        )
        .bind(&example_provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(product_types)
    }

    /// Returns the example provider pubkey for identifying example offerings
    pub fn example_provider_pubkey() -> Vec<u8> {
        hex::decode("6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572")
            .expect("Example provider pubkey hex should always decode successfully")
    }

    /// Search offerings using DSL query
    pub async fn search_offerings_dsl(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let five_mins_ns = 5i64 * 60 * 1_000_000_000;
        let heartbeat_cutoff = now_ns - five_mins_ns;

        // Parse DSL query
        let filters = crate::search::parse_dsl(query)
            .map_err(|e| anyhow::anyhow!("DSL parse error: {}", e))?;

        // Build SQL WHERE clause and bind values (starting from $3 since $1 and $2 are used below)
        let (dsl_where, dsl_values) = crate::search::build_sql_with_offset(&filters, 2)
            .map_err(|e| anyhow::anyhow!("SQL build error: {}", e))?;

        // Base SELECT with same fields as search_offerings
        let base_select = "SELECT o.id, lower(encode(o.pubkey, 'hex')) as pubkey, o.offering_id, o.offer_name, o.description, o.product_page_url, o.currency, o.monthly_price, o.setup_fee, o.visibility, o.product_type, o.virtualization_type, o.billing_interval, o.billing_unit, o.pricing_model, o.price_per_unit, o.included_units, o.overage_price_per_unit, o.stripe_metered_price_id, o.is_subscription, o.subscription_interval_days, o.stock_status, o.processor_brand, o.processor_amount, o.processor_cores, o.processor_speed, o.processor_name, o.memory_error_correction, o.memory_type, o.memory_amount, o.hdd_amount, o.total_hdd_capacity, o.ssd_amount, o.total_ssd_capacity, o.unmetered_bandwidth, o.uplink_speed, o.traffic, o.datacenter_country, o.datacenter_city, o.datacenter_latitude, o.datacenter_longitude, o.control_panel, o.gpu_name, o.gpu_count, o.gpu_memory_gb, o.min_contract_hours, o.max_contract_hours, o.payment_methods, o.features, o.operating_systems, p.trust_score, CASE WHEN p.pubkey IS NULL THEN NULL WHEN p.has_critical_flags = 1 THEN 1 ELSE 0 END as has_critical_flags, CASE WHEN lower(encode(o.pubkey, 'hex')) = $1 THEN 1 ELSE 0 END as is_example, o.offering_source, o.external_checkout_url, rp.name as reseller_name, rr.commission_percent as reseller_commission_percent, acc.username as owner_username, o.provisioner_type, o.provisioner_config, o.agent_pool_id, COALESCE(pas.online = 1 AND pas.last_heartbeat_ns > $2, 0) as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name FROM provider_offerings o LEFT JOIN provider_profiles p ON o.pubkey = p.pubkey LEFT JOIN reseller_relationships rr ON o.pubkey = rr.external_provider_pubkey AND rr.status = 'active' LEFT JOIN provider_profiles rp ON rr.reseller_pubkey = rp.pubkey LEFT JOIN account_public_keys apk ON o.pubkey = apk.public_key AND apk.is_active = 1 LEFT JOIN accounts acc ON apk.account_id = acc.id LEFT JOIN provider_agent_status pas ON o.pubkey = pas.provider_pubkey";

        // Build WHERE clause: base filters + DSL filters
        let where_clause = if dsl_where.is_empty() {
            "WHERE LOWER(o.visibility) = 'public'".to_string()
        } else {
            format!("WHERE LOWER(o.visibility) = 'public' AND ({})", dsl_where)
        };

        // Calculate LIMIT/OFFSET placeholder indices (after fixed bindings + DSL bindings)
        let limit_idx = 2 + dsl_values.len() + 1;
        let offset_idx = limit_idx + 1;

        // Complete query with ORDER BY and pagination
        let query_sql = format!(
            "{} {} ORDER BY o.monthly_price ASC LIMIT ${} OFFSET ${}",
            base_select, where_clause, limit_idx, offset_idx
        );

        // Build query with bindings
        let mut query_builder = sqlx::query_as::<_, Offering>(&query_sql)
            .bind(&example_provider_pubkey)
            .bind(heartbeat_cutoff);

        // Bind DSL values
        for value in dsl_values {
            query_builder = match value {
                crate::search::SqlValue::String(s) => query_builder.bind(s),
                crate::search::SqlValue::Integer(i) => query_builder.bind(i),
                crate::search::SqlValue::Float(f) => query_builder.bind(f),
                crate::search::SqlValue::Bool(b) => query_builder.bind(b),
            };
        }

        // Bind pagination
        query_builder = query_builder.bind(limit).bind(offset);

        let offerings = query_builder.fetch_all(&self.pool).await?;

        // Compute online status for all offerings
        let with_status = self.compute_provider_online_status(offerings).await?;

        // Filter to only include offerings that have a matching pool
        let filtered: Vec<Offering> = with_status
            .into_iter()
            .filter(|o| o.resolved_pool_id.is_some())
            .collect();

        Ok(filtered)
    }
}

#[allow(dead_code)]
impl Database {
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
        // Validate datacenter_country is a known ISO country code
        if !params.datacenter_country.is_empty()
            && !is_valid_country_code(&params.datacenter_country)
        {
            return Err(anyhow::anyhow!(
                "Invalid datacenter_country '{}': must be a valid ISO 3166-1 alpha-2 country code (e.g., US, DE, JP)",
                params.datacenter_country
            ));
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
            billing_unit,
            pricing_model,
            price_per_unit,
            included_units,
            overage_price_per_unit,
            stripe_metered_price_id,
            is_subscription,
            subscription_interval_days,
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
            gpu_count,
            gpu_memory_gb,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
            trust_score: _,
            has_critical_flags: _,
            is_example: _,
            offering_source,
            external_checkout_url,
            reseller_name: _,
            reseller_commission_percent: _,
            owner_username: _,
            provisioner_type,
            provisioner_config,
            agent_pool_id,
            provider_online: _,
            resolved_pool_id: _,
            resolved_pool_name: _,
        } = params;

        let mut tx = self.pool.begin().await?;

        // Check for duplicate offering_id for this provider
        let existing: Option<i64> = sqlx::query_scalar!(
            r#"SELECT id as "id!: i64" FROM provider_offerings WHERE pubkey = $1 AND offering_id = $2"#,
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
                virtualization_type, billing_interval, billing_unit, pricing_model,
                price_per_unit, included_units, overage_price_per_unit, stripe_metered_price_id,
                is_subscription, subscription_interval_days,
                stock_status, processor_brand,
                processor_amount, processor_cores, processor_speed, processor_name,
                memory_error_correction, memory_type, memory_amount, hdd_amount,
                total_hdd_capacity, ssd_amount, total_ssd_capacity, unmetered_bandwidth,
                uplink_speed, traffic, datacenter_country, datacenter_city,
                datacenter_latitude, datacenter_longitude, control_panel, gpu_name,
                gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours,
                payment_methods, features, operating_systems, offering_source,
                external_checkout_url, provisioner_type, provisioner_config, agent_pool_id, created_at_ns
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, $13, $14,
                $15, $16, $17, $18,
                $19, $20,
                $21, $22,
                $23, $24, $25, $26,
                $27, $28, $29, $30,
                $31, $32, $33, $34,
                $35, $36, $37, $38,
                $39, $40, $41, $42,
                $43, $44, $45, $46,
                $47, $48, $49, $50,
                $51, $52, $53, $54, $55
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
            billing_unit,
            pricing_model,
            price_per_unit,
            included_units,
            overage_price_per_unit,
            stripe_metered_price_id,
            is_subscription,
            subscription_interval_days,
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
            gpu_count,
            gpu_memory_gb,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
            offering_source,
            external_checkout_url,
            provisioner_type,
            provisioner_config,
            agent_pool_id,
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
            "SELECT pubkey FROM provider_offerings WHERE id = $1",
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

        // Validate datacenter_country is a known ISO country code
        if !params.datacenter_country.is_empty()
            && !is_valid_country_code(&params.datacenter_country)
        {
            return Err(anyhow::anyhow!(
                "Invalid datacenter_country '{}': must be a valid ISO 3166-1 alpha-2 country code (e.g., US, DE, JP)",
                params.datacenter_country
            ));
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
            billing_unit,
            pricing_model,
            price_per_unit,
            included_units,
            overage_price_per_unit,
            stripe_metered_price_id,
            is_subscription,
            subscription_interval_days,
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
            gpu_count,
            gpu_memory_gb,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
            trust_score: _,
            has_critical_flags: _,
            is_example: _,
            offering_source,
            external_checkout_url,
            reseller_name: _,
            reseller_commission_percent: _,
            owner_username: _,
            provisioner_type,
            provisioner_config,
            agent_pool_id,
            provider_online: _,
            resolved_pool_id: _,
            resolved_pool_name: _,
        } = params;

        sqlx::query!(
            r#"UPDATE provider_offerings SET
                offering_id = $1, offer_name = $2, description = $3, product_page_url = $4,
                currency = $5, monthly_price = $6, setup_fee = $7, visibility = $8, product_type = $9,
                virtualization_type = $10, billing_interval = $11,
                billing_unit = $12, pricing_model = $13, price_per_unit = $14,
                included_units = $15, overage_price_per_unit = $16, stripe_metered_price_id = $17,
                is_subscription = $18, subscription_interval_days = $19,
                stock_status = $20,
                processor_brand = $21, processor_amount = $22, processor_cores = $23, processor_speed = $24,
                processor_name = $25, memory_error_correction = $26, memory_type = $27, memory_amount = $28,
                hdd_amount = $29, total_hdd_capacity = $30, ssd_amount = $31, total_ssd_capacity = $32,
                unmetered_bandwidth = $33, uplink_speed = $34, traffic = $35, datacenter_country = $36,
                datacenter_city = $37, datacenter_latitude = $38, datacenter_longitude = $39,
                control_panel = $40, gpu_name = $41, gpu_count = $42, gpu_memory_gb = $43,
                min_contract_hours = $44, max_contract_hours = $45,
                payment_methods = $46, features = $47, operating_systems = $48,
                offering_source = $49, external_checkout_url = $50,
                provisioner_type = $51, provisioner_config = $52, agent_pool_id = $53
            WHERE id = $54"#,
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
            billing_unit,
            pricing_model,
            price_per_unit,
            included_units,
            overage_price_per_unit,
            stripe_metered_price_id,
            is_subscription,
            subscription_interval_days,
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
            gpu_count,
            gpu_memory_gb,
            min_contract_hours,
            max_contract_hours,
            payment_methods,
            features,
            operating_systems,
            offering_source,
            external_checkout_url,
            provisioner_type,
            provisioner_config,
            agent_pool_id,
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
            "SELECT pubkey FROM provider_offerings WHERE id = $1",
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
            "DELETE FROM provider_offerings WHERE id = $1",
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
        let source_pubkey_bytes = hex::decode(&source.pubkey)
            .map_err(|_| anyhow::anyhow!("Invalid pubkey hex in source offering"))?;
        if source_pubkey_bytes != pubkey {
            return Err(anyhow::anyhow!(
                "Unauthorized: You do not own this offering"
            ));
        }

        // Get metadata directly from source offering

        // Create new offering with duplicated data
        let params = Offering {
            id: None,
            pubkey: hex::encode(pubkey),
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
            billing_unit: source.billing_unit,
            pricing_model: source.pricing_model,
            price_per_unit: source.price_per_unit,
            included_units: source.included_units,
            overage_price_per_unit: source.overage_price_per_unit,
            stripe_metered_price_id: source.stripe_metered_price_id,
            is_subscription: source.is_subscription,
            subscription_interval_days: source.subscription_interval_days,
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
            gpu_count: source.gpu_count,
            gpu_memory_gb: source.gpu_memory_gb,
            min_contract_hours: source.min_contract_hours,
            max_contract_hours: source.max_contract_hours,
            payment_methods: source.payment_methods,
            features: source.features,
            operating_systems: source.operating_systems,
            trust_score: None,
            has_critical_flags: None,
            is_example: false,
            offering_source: source.offering_source,
            external_checkout_url: source.external_checkout_url,
            reseller_name: None,
            reseller_commission_percent: None,
            owner_username: None,
            provisioner_type: source.provisioner_type,
            provisioner_config: source.provisioner_config,
            agent_pool_id: source.agent_pool_id,
            provider_online: None,
            resolved_pool_id: None,
            resolved_pool_name: None,
        };

        self.create_offering(pubkey, params).await
    }

    /// Bulk update stock_status for multiple offerings
    pub async fn bulk_update_stock_status(
        &self,
        pubkey: &[u8],
        offering_ids: &[i64],
        new_status: &str,
    ) -> Result<u64> {
        if offering_ids.is_empty() {
            return Ok(0);
        }

        // Verify all offerings belong to this provider
        let id_placeholders: Vec<String> = (1..=offering_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        let pubkey_placeholder = format!("${}", offering_ids.len() + 1);
        let verify_query = format!(
            "SELECT COUNT(*) as count FROM provider_offerings WHERE id IN ({}) AND pubkey = {}",
            id_placeholders.join(","),
            pubkey_placeholder
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
        let update_id_placeholders: Vec<String> = (2..=offering_ids.len() + 1)
            .map(|i| format!("${}", i))
            .collect();
        let update_query = format!(
            "UPDATE provider_offerings SET stock_status = $1 WHERE id IN ({})",
            update_id_placeholders.join(",")
        );

        let mut update_builder = sqlx::query(&update_query);
        update_builder = update_builder.bind(new_status);
        for id in offering_ids {
            update_builder = update_builder.bind(id);
        }

        let result = update_builder.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    // Helper function to convert Vec<String> to Option<String> (comma-separated)
    fn vec_to_csv(vec: &[String]) -> Option<String> {
        if vec.is_empty() {
            None
        } else {
            Some(vec.join(","))
        }
    }

    /// Import offerings from CSV data
    /// Returns (success_count, errors) where errors is Vec<(row_number, error_message)>
    pub async fn import_offerings_csv(
        &self,
        pubkey: &[u8],
        csv_data: &str,
        upsert: bool,
    ) -> Result<(usize, Vec<(usize, String)>)> {
        self.import_offerings_csv_internal(pubkey, csv_data, upsert, None)
            .await
    }

    /// Import seeded offerings from CSV data with offering_source='seeded'
    /// Returns (success_count, errors) where errors is Vec<(row_number, error_message)>
    pub async fn import_seeded_offerings_csv(
        &self,
        pubkey: &[u8],
        csv_data: &str,
        upsert: bool,
    ) -> Result<(usize, Vec<(usize, String)>)> {
        self.import_offerings_csv_internal(pubkey, csv_data, upsert, Some("seeded"))
            .await
    }

    /// Internal CSV import with optional offering_source override
    async fn import_offerings_csv_internal(
        &self,
        pubkey: &[u8],
        csv_data: &str,
        upsert: bool,
        offering_source_override: Option<&str>,
    ) -> Result<(usize, Vec<(usize, String)>)> {
        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let mut success_count = 0;
        let mut errors = Vec::new();

        // Build header->index map for column-order-agnostic parsing
        let headers = reader.headers()?.clone();
        let col_map: HashMap<&str, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.trim(), i))
            .collect();

        for (row_idx, result) in reader.records().enumerate() {
            let row_number = row_idx + 2; // +2 because row 1 is header, 0-indexed

            match result {
                Ok(record) => {
                    match Self::parse_csv_record(&record, &col_map) {
                        Ok(mut params) => {
                            // Override offering_source if specified
                            if let Some(source) = offering_source_override {
                                params.offering_source = Some(source.to_string());
                                // For seeded offerings, copy product_page_url to external_checkout_url
                                if source == "seeded" && params.external_checkout_url.is_none() {
                                    params.external_checkout_url = params.product_page_url.clone();
                                }
                            }

                            // Validate agent_pool_id if provided
                            if let Some(pool_id) = &params.agent_pool_id {
                                if !pool_id.is_empty() {
                                    match self.get_agent_pool(pool_id).await {
                                        Err(e) => {
                                            errors.push((
                                                row_number,
                                                format!("Failed to validate agent_pool_id: {}", e),
                                            ));
                                            continue;
                                        }
                                        Ok(None) => {
                                            errors.push((
                                                row_number,
                                                format!("Pool '{}' does not exist", pool_id),
                                            ));
                                            continue;
                                        }
                                        Ok(Some(pool)) => {
                                            let provider_hex = hex::encode(pubkey);
                                            if pool.provider_pubkey != provider_hex {
                                                errors.push((
                                                    row_number,
                                                    format!(
                                                        "Pool '{}' belongs to different provider",
                                                        pool_id
                                                    ),
                                                ));
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }

                            let result: Result<()> = if upsert {
                                // Try to find existing offering by offering_id
                                let existing_offering_id = &params.offering_id;
                                match sqlx::query_scalar!(
                                    r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1 AND (pubkey) = $2"#,
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

    /// Parse a single CSV record into Offering using header-based column lookup
    fn parse_csv_record(
        record: &csv::StringRecord,
        col_map: &HashMap<&str, usize>,
    ) -> Result<Offering, String> {
        let get = |name: &str| col_map.get(name).and_then(|&i| record.get(i));

        let get_str = |name: &str| get(name).unwrap_or("").to_string();
        let get_opt_str = |name: &str| {
            get(name).and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
        };
        let get_opt_i64 = |name: &str| {
            get(name).and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.parse::<i64>().ok()
                }
            })
        };
        let get_opt_f64 = |name: &str| {
            get(name).and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.parse::<f64>().ok()
                }
            })
        };
        let get_f64 = |name: &str| -> Result<f64, String> {
            get(name)
                .ok_or_else(|| format!("Missing column '{}'", name))?
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("Invalid number in column '{}'", name))
        };
        let get_bool = |name: &str| {
            get(name)
                .map(|s| {
                    let lower = s.trim().to_lowercase();
                    lower == "true" || lower == "1" || lower == "yes"
                })
                .unwrap_or(false)
        };
        let get_opt_csv = |name: &str| -> Option<String> {
            get(name).and_then(|s| {
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
        let offering_id = get_str("offering_id");
        let offer_name = get_str("offer_name");

        if offering_id.trim().is_empty() {
            return Err("offering_id is required".to_string());
        }
        if offer_name.trim().is_empty() {
            return Err("offer_name is required".to_string());
        }

        Ok(Offering {
            id: None,
            pubkey: String::new(), // Will be set by caller
            offering_id,
            offer_name,
            description: get_opt_str("description"),
            product_page_url: get_opt_str("product_page_url"),
            currency: get_str("currency"),
            monthly_price: get_f64("monthly_price")?,
            setup_fee: get_f64("setup_fee")?,
            visibility: get_str("visibility"),
            product_type: get_str("product_type"),
            virtualization_type: get_opt_str("virtualization_type"),
            billing_interval: get_str("billing_interval"),
            billing_unit: {
                let unit = get_str("billing_unit");
                if unit.is_empty() {
                    "month".to_string()
                } else {
                    unit
                }
            },
            pricing_model: get_opt_str("pricing_model"),
            price_per_unit: get_opt_f64("price_per_unit"),
            included_units: get_opt_i64("included_units"),
            overage_price_per_unit: get_opt_f64("overage_price_per_unit"),
            stripe_metered_price_id: get_opt_str("stripe_metered_price_id"),
            is_subscription: get_bool("is_subscription"),
            subscription_interval_days: get_opt_i64("subscription_interval_days"),
            stock_status: get_str("stock_status"),
            processor_brand: get_opt_str("processor_brand"),
            processor_amount: get_opt_i64("processor_amount"),
            processor_cores: get_opt_i64("processor_cores"),
            processor_speed: get_opt_str("processor_speed"),
            processor_name: get_opt_str("processor_name"),
            memory_error_correction: get_opt_str("memory_error_correction"),
            memory_type: get_opt_str("memory_type"),
            memory_amount: get_opt_str("memory_amount"),
            hdd_amount: get_opt_i64("hdd_amount"),
            total_hdd_capacity: get_opt_str("total_hdd_capacity"),
            ssd_amount: get_opt_i64("ssd_amount"),
            total_ssd_capacity: get_opt_str("total_ssd_capacity"),
            unmetered_bandwidth: get_bool("unmetered_bandwidth"),
            uplink_speed: get_opt_str("uplink_speed"),
            traffic: get_opt_i64("traffic"),
            datacenter_country: get_str("datacenter_country"),
            datacenter_city: get_str("datacenter_city"),
            datacenter_latitude: get_opt_f64("datacenter_latitude"),
            datacenter_longitude: get_opt_f64("datacenter_longitude"),
            control_panel: get_opt_str("control_panel"),
            gpu_name: get_opt_str("gpu_name"),
            gpu_count: get_opt_i64("gpu_count"),
            gpu_memory_gb: get_opt_i64("gpu_memory_gb"),
            min_contract_hours: get_opt_i64("min_contract_hours"),
            max_contract_hours: get_opt_i64("max_contract_hours"),
            payment_methods: get_opt_csv("payment_methods"),
            features: get_opt_csv("features"),
            operating_systems: get_opt_csv("operating_systems"),
            trust_score: None,
            has_critical_flags: None,
            is_example: false,
            offering_source: get_opt_str("offering_source"),
            external_checkout_url: get_opt_str("external_checkout_url"),
            reseller_name: None,
            reseller_commission_percent: None,
            owner_username: None,
            provisioner_type: get_opt_str("provisioner_type"),
            provisioner_config: get_opt_str("provisioner_config"),
            agent_pool_id: get_opt_str("agent_pool_id"),
            provider_online: None,
            resolved_pool_id: None,
            resolved_pool_name: None,
        })
    }
}

#[cfg(test)]
mod tests;
