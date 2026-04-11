use super::types::Database;
use super::user_notifications::insert_notification;
use crate::regions::{country_to_region, is_valid_country_code};
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::Row;
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Composite reliability score 0-100: uptime 40% + completion rate 35% + response rate 25%.
    /// None if insufficient data.
    #[ts(type = "number | undefined")]
    #[sqlx(default)]
    pub reliability_score: Option<f64>,
    // Draft flag - draft offerings are hidden from public marketplace search
    #[ts(type = "boolean")]
    #[sqlx(default)]
    pub is_draft: bool,
    // Scheduled publish time: when is_draft=true and publish_at <= NOW(), the offering is auto-published
    #[ts(type = "string | undefined")]
    #[sqlx(default)]
    pub publish_at: Option<chrono::DateTime<chrono::Utc>>,
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
    // Template name for provisioning (e.g. "ubuntu-22.04")
    pub template_name: Option<String>,
    // Agent pool override - if set, only agents in this pool can provision
    // NULL = auto-match by location
    pub agent_pool_id: Option<String>,
    // Post-provision script to execute via SSH after VM provisioning
    pub post_provision_script: Option<String>,
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
    // Creation timestamp in nanoseconds since epoch
    #[ts(type = "number | undefined")]
    #[sqlx(default)]
    pub created_at_ns: Option<i64>,
}

struct SavedOfferingPriceChange {
    offer_name: String,
    old_currency: String,
    old_monthly_price: f64,
    new_currency: String,
    new_monthly_price: f64,
    old_setup_fee: f64,
    new_setup_fee: f64,
    old_price_per_unit: Option<f64>,
    new_price_per_unit: Option<f64>,
    old_overage_price_per_unit: Option<f64>,
    new_overage_price_per_unit: Option<f64>,
}

fn opt_f64_changed(old: Option<f64>, new: Option<f64>) -> bool {
    match (old, new) {
        (Some(o), Some(n)) => (o - n).abs() >= 1e-9,
        _ => false,
    }
}

/// Pricing statistics for offerings matching given filters
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct OfferingPricingStats {
    pub count: i64,
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
    pub median_price: f64,
}

/// A trending offering with its 7-day view count
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct TrendingOffering {
    pub offering_id: i64,
    pub offer_name: String,
    pub pubkey: String,
    pub product_type: String,
    pub monthly_price: f64,
    pub currency: String,
    pub datacenter_country: Option<String>,
    pub datacenter_city: Option<String>,
    pub trust_score: Option<f64>,
    pub views_7d: i64,
}

/// A recommended offering with a relevance score
///
/// PoC(338): Content-based recommendation engine.
/// Score is computed from attribute similarity to the user's viewed/saved offerings:
///   - product_type match: +3 per signal
///   - datacenter_country match: +2 per signal
///   - gpu_name match: +4 per signal
///   - price within 1 stddev of user's average: +1
/// Dev stage next steps:
///   1. Consider collaborative filtering (users who viewed X also viewed Y)
///   2. Add time-decay weighting (recent views weighted higher)
///   3. Tune scoring weights based on real usage data
///   4. Consider caching the user profile for performance
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct RecommendedOffering {
    pub offering_id: i64,
    pub offer_name: String,
    pub pubkey: String,
    pub product_type: String,
    pub monthly_price: f64,
    pub currency: String,
    pub datacenter_country: Option<String>,
    pub datacenter_city: Option<String>,
    pub trust_score: Option<f64>,
    pub gpu_name: Option<String>,
    pub score: f64,
}

/// Internal struct for user preference profile built from viewed/saved offerings
struct UserPreferenceProfile {
    preferred_types: HashMap<String, f64>,
    preferred_countries: HashMap<String, f64>,
    preferred_gpus: HashMap<String, f64>,
    avg_price: Option<f64>,
    price_stddev: Option<f64>,
}

/// Internal struct for a signal offering (viewed or saved by the user)
#[derive(sqlx::FromRow)]
struct SignalOffering {
    product_type: String,
    datacenter_country: String,
    gpu_name: Option<String>,
    monthly_price: f64,
}

/// View analytics for a single offering
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct OfferingAnalytics {
    pub views_7d: i64,
    pub views_30d: i64,
    pub unique_viewers_7d: i64,
    pub unique_viewers_30d: i64,
}

/// Daily view counts for trend analysis
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct DailyViewTrend {
    pub day: String,
    pub views: i64,
    pub unique_viewers: i64,
}

/// A tier definition for auto-generating offerings
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct OfferingTier {
    /// Tier identifier (e.g., "small", "medium", "large", "gpu-small")
    pub name: String,
    /// Human-readable tier name (e.g., "Basic VPS", "Standard VPS")
    pub display_name: String,
    /// Number of CPU cores for this tier
    pub cpu_cores: u32,
    /// Memory in GB for this tier
    pub memory_gb: u32,
    /// Storage in GB for this tier
    pub storage_gb: u32,
    /// Number of GPUs (None for non-GPU tiers)
    pub gpu_count: Option<u32>,
    /// Minimum pool CPU cores required to offer this tier
    pub min_pool_cpu: u32,
    /// Minimum pool memory (GB) required to offer this tier
    pub min_pool_memory_gb: u32,
    /// Minimum pool storage (GB) required to offer this tier
    pub min_pool_storage_gb: u32,
}

/// Get default compute tiers
pub fn default_compute_tiers() -> Vec<OfferingTier> {
    vec![
        OfferingTier {
            name: "small".to_string(),
            display_name: "Basic VPS".to_string(),
            cpu_cores: 1,
            memory_gb: 2,
            storage_gb: 25,
            gpu_count: None,
            min_pool_cpu: 4,
            min_pool_memory_gb: 8,
            min_pool_storage_gb: 100,
        },
        OfferingTier {
            name: "medium".to_string(),
            display_name: "Standard VPS".to_string(),
            cpu_cores: 2,
            memory_gb: 4,
            storage_gb: 50,
            gpu_count: None,
            min_pool_cpu: 8,
            min_pool_memory_gb: 16,
            min_pool_storage_gb: 200,
        },
        OfferingTier {
            name: "large".to_string(),
            display_name: "Performance VPS".to_string(),
            cpu_cores: 4,
            memory_gb: 8,
            storage_gb: 100,
            gpu_count: None,
            min_pool_cpu: 16,
            min_pool_memory_gb: 32,
            min_pool_storage_gb: 400,
        },
        OfferingTier {
            name: "xlarge".to_string(),
            display_name: "High Performance VPS".to_string(),
            cpu_cores: 8,
            memory_gb: 16,
            storage_gb: 200,
            gpu_count: None,
            min_pool_cpu: 32,
            min_pool_memory_gb: 64,
            min_pool_storage_gb: 800,
        },
    ]
}

/// Get default GPU tiers
pub fn default_gpu_tiers() -> Vec<OfferingTier> {
    vec![OfferingTier {
        name: "gpu-small".to_string(),
        display_name: "GPU Instance".to_string(),
        cpu_cores: 4,
        memory_gb: 16,
        storage_gb: 100,
        gpu_count: Some(1),
        min_pool_cpu: 8,
        min_pool_memory_gb: 32,
        min_pool_storage_gb: 200,
    }]
}

/// Reason why a tier is unavailable
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UnavailableTier {
    /// Tier name
    pub tier: String,
    /// Human-readable reason why tier is unavailable
    pub reason: String,
}

/// Suggested offering based on pool capabilities
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct OfferingSuggestion {
    /// Tier this suggestion is based on
    pub tier_name: String,
    /// Suggested offering ID (format: "{pool_id}-{tier_name}")
    pub offering_id: String,
    /// Suggested display name
    pub offer_name: String,
    /// CPU brand from pool capabilities
    pub processor_brand: Option<String>,
    /// CPU model from pool capabilities
    pub processor_name: Option<String>,
    /// Number of CPU cores
    #[ts(type = "number")]
    pub processor_cores: i64,
    /// Memory amount as string (e.g., "2 GB")
    pub memory_amount: String,
    /// Storage amount as string (e.g., "25 GB")
    pub total_ssd_capacity: String,
    /// GPU name if applicable
    pub gpu_name: Option<String>,
    /// GPU count if applicable
    #[ts(type = "number | undefined")]
    pub gpu_count: Option<i64>,
    /// Available templates/operating systems
    pub operating_systems: Option<String>,
    /// Country code from pool location
    pub datacenter_country: String,
    /// Requires pricing to be set
    pub needs_pricing: bool,
}

use super::agent_pools::PoolCapabilities;

/// Select applicable tiers based on pool capabilities
pub fn select_applicable_tiers(
    capabilities: &PoolCapabilities,
) -> (Vec<OfferingTier>, Vec<UnavailableTier>) {
    let mut applicable = Vec::new();
    let mut unavailable = Vec::new();

    // Check compute tiers
    for tier in default_compute_tiers() {
        match check_tier_eligibility(capabilities, &tier) {
            Ok(()) => applicable.push(tier),
            Err(reason) => unavailable.push(UnavailableTier {
                tier: tier.name,
                reason,
            }),
        }
    }

    // Check GPU tiers only if pool has GPUs
    if capabilities.has_gpu {
        for tier in default_gpu_tiers() {
            match check_tier_eligibility(capabilities, &tier) {
                Ok(()) => applicable.push(tier),
                Err(reason) => unavailable.push(UnavailableTier {
                    tier: tier.name,
                    reason,
                }),
            }
        }
    } else {
        // Add GPU tiers as unavailable with "No GPU" reason
        for tier in default_gpu_tiers() {
            unavailable.push(UnavailableTier {
                tier: tier.name,
                reason: "No GPU devices available in pool".to_string(),
            });
        }
    }

    (applicable, unavailable)
}

/// Check if a tier can be offered based on pool capabilities
fn check_tier_eligibility(
    capabilities: &PoolCapabilities,
    tier: &OfferingTier,
) -> Result<(), String> {
    // Check pool has enough total resources
    if capabilities.total_cpu_cores < tier.min_pool_cpu {
        return Err(format!(
            "Need {} total CPU cores (have {})",
            tier.min_pool_cpu, capabilities.total_cpu_cores
        ));
    }

    let total_memory_gb = capabilities.total_memory_mb / 1024;
    if total_memory_gb < tier.min_pool_memory_gb as u64 {
        return Err(format!(
            "Need {} GB total memory (have {} GB)",
            tier.min_pool_memory_gb, total_memory_gb
        ));
    }

    if capabilities.total_storage_gb < tier.min_pool_storage_gb as u64 {
        return Err(format!(
            "Need {} GB total storage (have {} GB)",
            tier.min_pool_storage_gb, capabilities.total_storage_gb
        ));
    }

    // Check smallest agent can host this tier
    if capabilities.min_agent_cpu_cores < tier.cpu_cores {
        return Err(format!(
            "Smallest agent has {} cores, tier needs {}",
            capabilities.min_agent_cpu_cores, tier.cpu_cores
        ));
    }

    let min_agent_memory_gb = capabilities.min_agent_memory_mb / 1024;
    if min_agent_memory_gb < tier.memory_gb as u64 {
        return Err(format!(
            "Smallest agent has {} GB memory, tier needs {} GB",
            min_agent_memory_gb, tier.memory_gb
        ));
    }

    if capabilities.min_agent_storage_gb < tier.storage_gb as u64 {
        return Err(format!(
            "Smallest agent has {} GB storage, tier needs {} GB",
            capabilities.min_agent_storage_gb, tier.storage_gb
        ));
    }

    // Check GPU requirement
    if tier.gpu_count.is_some() && !capabilities.has_gpu {
        return Err("No GPU devices available in pool".to_string());
    }

    Ok(())
}

/// Region to country code mapping for offerings
fn region_to_country_code(region: &str) -> &'static str {
    match region {
        "europe" => "DE",  // Default to Germany for Europe
        "na" => "US",      // Default to US for North America
        "apac" => "SG",    // Default to Singapore for APAC
        "sa" => "BR",      // Default to Brazil for South America
        "africa" => "ZA",  // Default to South Africa
        "oceania" => "AU", // Default to Australia
        "mena" => "AE",    // Default to UAE for Middle East
        _ => "US",         // Fallback
    }
}

/// Generate offering suggestions from pool capabilities
pub fn generate_suggestions(
    pool_id: &str,
    pool_name: &str,
    pool_location: &str,
    capabilities: &PoolCapabilities,
    tiers: &[OfferingTier],
) -> Vec<OfferingSuggestion> {
    let cpu_brand = capabilities
        .cpu_models
        .first()
        .and_then(|m| {
            if m.to_lowercase().contains("amd") {
                Some("AMD")
            } else if m.to_lowercase().contains("intel") {
                Some("Intel")
            } else {
                None
            }
        })
        .map(String::from);

    let cpu_model = capabilities.cpu_models.first().cloned();
    let gpu_model = capabilities.gpu_models.first().cloned();
    let templates_csv = if capabilities.available_templates.is_empty() {
        None
    } else {
        Some(capabilities.available_templates.join(", "))
    };

    let country_code = region_to_country_code(pool_location);

    tiers
        .iter()
        .map(|tier| OfferingSuggestion {
            tier_name: tier.name.clone(),
            offering_id: format!("{}-{}", pool_id, tier.name),
            offer_name: format!("{} ({})", tier.display_name, pool_name),
            processor_brand: cpu_brand.clone(),
            processor_name: cpu_model.clone(),
            processor_cores: tier.cpu_cores as i64,
            memory_amount: format!("{} GB", tier.memory_gb),
            total_ssd_capacity: format!("{} GB", tier.storage_gb),
            gpu_name: if tier.gpu_count.is_some() {
                gpu_model.clone()
            } else {
                None
            },
            gpu_count: tier.gpu_count.map(|c| c as i64),
            operating_systems: templates_csv.clone(),
            datacenter_country: country_code.to_string(),
            needs_pricing: true,
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct SearchOfferingsParams<'a> {
    pub product_type: Option<&'a str>,
    pub country: Option<&'a str>,
    pub in_stock_only: bool,
    pub has_recipe: bool,
    pub min_price_monthly: Option<f64>,
    pub max_price_monthly: Option<f64>,
    pub limit: i64,
    pub offset: i64,
    /// Plain-text ILIKE search across offer_name, description, and product_type
    pub text_search: Option<&'a str>,
}

impl Database {
    /// Search offerings with filters.
    /// Excludes offerings that don't have a matching agent pool.
    pub async fn search_offerings(
        &self,
        params: SearchOfferingsParams<'_>,
    ) -> Result<Vec<Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let now_ns = crate::now_ns()?;
        let five_mins_ns = 5i64 * 60 * 1_000_000_000;
        let heartbeat_cutoff = now_ns - five_mins_ns;
        let mut query = String::from(
            "SELECT o.id, lower(encode(o.pubkey, 'hex')) as pubkey, o.offering_id, o.offer_name, o.description, o.product_page_url, o.currency, o.monthly_price, o.setup_fee, o.visibility, o.product_type, o.virtualization_type, o.billing_interval, o.billing_unit, o.pricing_model, o.price_per_unit, o.included_units, o.overage_price_per_unit, o.stripe_metered_price_id, o.is_subscription, o.subscription_interval_days, o.stock_status, o.processor_brand, o.processor_amount, o.processor_cores, o.processor_speed, o.processor_name, o.memory_error_correction, o.memory_type, o.memory_amount, o.hdd_amount, o.total_hdd_capacity, o.ssd_amount, o.total_ssd_capacity, o.unmetered_bandwidth, o.uplink_speed, o.traffic, o.datacenter_country, o.datacenter_city, o.datacenter_latitude, o.datacenter_longitude, o.control_panel, o.gpu_name, o.gpu_count, o.gpu_memory_gb, o.min_contract_hours, o.max_contract_hours, o.payment_methods, o.features, o.operating_systems, p.trust_score, CASE WHEN p.pubkey IS NULL THEN NULL WHEN p.has_critical_flags THEN TRUE ELSE FALSE END as has_critical_flags, p.reliability_score, o.is_draft, o.publish_at, CASE WHEN lower(encode(o.pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example, o.offering_source, o.external_checkout_url, rp.name as reseller_name, rr.commission_percent as reseller_commission_percent, acc.username as owner_username, o.provisioner_type, o.provisioner_config, o.template_name, o.agent_pool_id, o.post_provision_script, EXISTS(SELECT 1 FROM provider_agent_status s WHERE s.provider_pubkey = o.pubkey AND s.online = TRUE AND s.last_heartbeat_ns > $2) as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name FROM provider_offerings o LEFT JOIN provider_profiles p ON o.pubkey = p.pubkey LEFT JOIN reseller_relationships rr ON o.pubkey = rr.external_provider_pubkey AND rr.status = 'active' LEFT JOIN provider_profiles rp ON rr.reseller_pubkey = rp.pubkey LEFT JOIN account_public_keys apk ON o.pubkey = apk.public_key AND apk.is_active = TRUE LEFT JOIN accounts acc ON apk.account_id = acc.id WHERE LOWER(o.visibility) = 'public' AND o.is_draft = FALSE"
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
        if params.has_recipe {
            query.push_str(" AND o.post_provision_script IS NOT NULL");
        }
        if params.text_search.is_some() {
            idx += 1;
            query.push_str(&format!(
                " AND (o.offer_name ILIKE ${idx} OR o.description ILIKE ${idx} OR o.product_type ILIKE ${idx})"
            ));
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
        query.push_str(&format!(
            " ORDER BY p.reliability_score DESC NULLS LAST, p.trust_score DESC NULLS LAST, o.monthly_price ASC, o.id ASC LIMIT ${} OFFSET ${}",
            limit_idx, offset_idx
        ));

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
        if let Some(ts) = params.text_search {
            query_builder = query_builder.bind(format!("%{}%", ts));
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

        // Filter to only include offerings that have a matching pool or are self-provisioned
        let filtered: Vec<Offering> = with_status
            .into_iter()
            .filter(|o| {
                o.resolved_pool_id.is_some()
                    || o.offering_source.as_deref() == Some("self_provisioned")
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
        let mut by_provider: HashMap<String, Vec<(usize, Offering)>> = HashMap::new();
        for (index, offering) in offerings.into_iter().enumerate() {
            by_provider
                .entry(offering.pubkey.clone())
                .or_default()
                .push((index, offering));
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
            for (original_index, mut offering) in provider_offerings {
                // Self-provisioned offerings are always "online" — the VM is already running
                if offering.offering_source.as_deref() == Some("self_provisioned") {
                    offering.provider_online = Some(true);
                    result.push((original_index, offering));
                    continue;
                }

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

                result.push((original_index, offering));
            }
        }

        result.sort_by_key(|(original_index, _)| *original_index);

        Ok(result.into_iter().map(|(_, offering)| offering).collect())
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
               NULL as trust_score, NULL as has_critical_flags, NULL::DOUBLE PRECISION as reliability_score, is_draft, publish_at, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, template_name, agent_pool_id, post_provision_script, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
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

    /// Get public offerings by provider (for public API - respects visibility)
    pub async fn get_provider_offerings_public(&self, pubkey: &[u8]) -> Result<Vec<Offering>> {
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
               NULL as trust_score, NULL as has_critical_flags, NULL::DOUBLE PRECISION as reliability_score, is_draft, publish_at, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, template_name, agent_pool_id, post_provision_script, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings WHERE pubkey = $2 AND LOWER(visibility) = 'public' ORDER BY monthly_price ASC"#
        )
        .bind(example_provider_pubkey)
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        let result = self.compute_provider_online_status(offerings).await?;

        Ok(result)
    }

    /// Get single offering by id
    pub async fn get_offering(&self, offering_id: i64) -> Result<Option<Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let offering =
            sqlx::query_as::<_, Offering>(r#"SELECT provider_offerings.id, lower(encode(pubkey, 'hex')) as pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price,
                setup_fee, visibility, product_type, virtualization_type, billing_interval,
                billing_unit, pricing_model, price_per_unit, included_units, overage_price_per_unit, stripe_metered_price_id,
                is_subscription, subscription_interval_days,
                stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
                memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems,
               NULL as trust_score, NULL as has_critical_flags, NULL::DOUBLE PRECISION as reliability_score, is_draft, publish_at, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, acc.username as owner_username,
               provisioner_type, provisioner_config, template_name, agent_pool_id, post_provision_script, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings
               LEFT JOIN account_public_keys apk ON provider_offerings.pubkey = apk.public_key AND apk.is_active = TRUE
               LEFT JOIN accounts acc ON apk.account_id = acc.id
               WHERE provider_offerings.id = $2"#)
                .bind(example_provider_pubkey)
                .bind(offering_id)
                .fetch_optional(&self.pool)
                .await?;

        let offering = match offering {
            Some(o) => o,
            None => return Ok(None),
        };

        let result = self.compute_provider_online_status(vec![offering]).await?;
        Ok(result.into_iter().next())
    }

    /// Get example offerings for CSV template generation.
    /// Used by: database tests for example data verification
    #[cfg(test)]
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
               NULL as trust_score, NULL as has_critical_flags, NULL::DOUBLE PRECISION as reliability_score, is_draft, publish_at, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, template_name, agent_pool_id, post_provision_script, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings WHERE pubkey = $2 ORDER BY offering_id ASC"#
        )
        .bind(&example_provider_pubkey_hex)
        .bind(&example_provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(offerings)
    }

    /// Get example offerings filtered by product type.
    /// Used by: GET /offerings/csv-template endpoint
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
               NULL as trust_score, NULL as has_critical_flags, NULL::DOUBLE PRECISION as reliability_score, is_draft, publish_at, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, template_name, agent_pool_id, post_provision_script, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
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
        let now_ns = crate::now_ns()?;
        let five_mins_ns = 5i64 * 60 * 1_000_000_000;
        let heartbeat_cutoff = now_ns - five_mins_ns;

        // Parse DSL query
        let filters = crate::search::parse_dsl(query)
            .map_err(|e| anyhow::anyhow!("DSL parse error: {}", e))?;

        // Build SQL WHERE clause and bind values (starting from $3 since $1 and $2 are used below)
        let (dsl_where, dsl_values) = crate::search::build_sql_with_offset(&filters, 2)
            .map_err(|e| anyhow::anyhow!("SQL build error: {}", e))?;

        // Base SELECT with same fields as search_offerings
        let base_select = "SELECT o.id, lower(encode(o.pubkey, 'hex')) as pubkey, o.offering_id, o.offer_name, o.description, o.product_page_url, o.currency, o.monthly_price, o.setup_fee, o.visibility, o.product_type, o.virtualization_type, o.billing_interval, o.billing_unit, o.pricing_model, o.price_per_unit, o.included_units, o.overage_price_per_unit, o.stripe_metered_price_id, o.is_subscription, o.subscription_interval_days, o.stock_status, o.processor_brand, o.processor_amount, o.processor_cores, o.processor_speed, o.processor_name, o.memory_error_correction, o.memory_type, o.memory_amount, o.hdd_amount, o.total_hdd_capacity, o.ssd_amount, o.total_ssd_capacity, o.unmetered_bandwidth, o.uplink_speed, o.traffic, o.datacenter_country, o.datacenter_city, o.datacenter_latitude, o.datacenter_longitude, o.control_panel, o.gpu_name, o.gpu_count, o.gpu_memory_gb, o.min_contract_hours, o.max_contract_hours, o.payment_methods, o.features, o.operating_systems, p.trust_score, CASE WHEN p.pubkey IS NULL THEN NULL WHEN p.has_critical_flags THEN TRUE ELSE FALSE END as has_critical_flags, p.reliability_score, o.is_draft, o.publish_at, CASE WHEN lower(encode(o.pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example, o.offering_source, o.external_checkout_url, rp.name as reseller_name, rr.commission_percent as reseller_commission_percent, acc.username as owner_username, o.provisioner_type, o.provisioner_config, o.template_name, o.agent_pool_id, o.post_provision_script, EXISTS(SELECT 1 FROM provider_agent_status s WHERE s.provider_pubkey = o.pubkey AND s.online = TRUE AND s.last_heartbeat_ns > $2) as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name FROM provider_offerings o LEFT JOIN provider_profiles p ON o.pubkey = p.pubkey LEFT JOIN reseller_relationships rr ON o.pubkey = rr.external_provider_pubkey AND rr.status = 'active' LEFT JOIN provider_profiles rp ON rr.reseller_pubkey = rp.pubkey LEFT JOIN account_public_keys apk ON o.pubkey = apk.public_key AND apk.is_active = TRUE LEFT JOIN accounts acc ON apk.account_id = acc.id";

        // Build WHERE clause: base filters + DSL filters
        let where_clause = if dsl_where.is_empty() {
            "WHERE LOWER(o.visibility) = 'public' AND o.is_draft = FALSE".to_string()
        } else {
            format!(
                "WHERE LOWER(o.visibility) = 'public' AND o.is_draft = FALSE AND ({})",
                dsl_where
            )
        };

        // Calculate LIMIT/OFFSET placeholder indices (after fixed bindings + DSL bindings)
        let limit_idx = 2 + dsl_values.len() + 1;
        let offset_idx = limit_idx + 1;

        // Complete query with ORDER BY and pagination
        let query_sql = format!(
            "{} {} ORDER BY p.reliability_score DESC NULLS LAST, p.trust_score DESC NULLS LAST, o.monthly_price ASC, o.id ASC LIMIT ${} OFFSET ${}",
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

        // Filter to only include offerings that have a matching pool or are self-provisioned
        let filtered: Vec<Offering> = with_status
            .into_iter()
            .filter(|o| {
                o.resolved_pool_id.is_some()
                    || o.offering_source.as_deref() == Some("self_provisioned")
            })
            .collect();

        Ok(filtered)
    }
}

impl Database {
    /// Count offerings.
    /// Used by: database tests for count verification
    #[cfg(test)]
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

    async fn notify_saved_offering_price_change(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        offering_id: i64,
        change: &SavedOfferingPriceChange,
    ) -> Result<()> {
        let mut changes: Vec<String> = Vec::new();

        if (change.old_monthly_price - change.new_monthly_price).abs() >= 1e-9 {
            changes.push(format!(
                "monthly_price from {} {:.2} to {} {:.2}",
                change.old_currency, change.old_monthly_price,
                change.new_currency, change.new_monthly_price
            ));
        }
        if (change.old_setup_fee - change.new_setup_fee).abs() >= 1e-9 {
            changes.push(format!(
                "setup_fee from {} {:.2} to {} {:.2}",
                change.old_currency, change.old_setup_fee,
                change.new_currency, change.new_setup_fee
            ));
        }
        if opt_f64_changed(change.old_price_per_unit, change.new_price_per_unit) {
            changes.push(format!(
                "price_per_unit from {} {:.2} to {} {:.2}",
                change.old_currency, change.old_price_per_unit.unwrap(),
                change.new_currency, change.new_price_per_unit.unwrap()
            ));
        }
        if opt_f64_changed(change.old_overage_price_per_unit, change.new_overage_price_per_unit) {
            changes.push(format!(
                "overage_price_per_unit from {} {:.2} to {} {:.2}",
                change.old_currency, change.old_overage_price_per_unit.unwrap(),
                change.new_currency, change.new_overage_price_per_unit.unwrap()
            ));
        }

        if changes.is_empty() {
            return Ok(());
        }

        let saved_user_pubkeys = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT user_pubkey FROM saved_offerings WHERE offering_id = $1",
        )
        .bind(offering_id)
        .fetch_all(&mut **tx)
        .await?;

        if saved_user_pubkeys.is_empty() {
            return Ok(());
        }

        let monthly_dropped = change.new_monthly_price < change.old_monthly_price;
        let setup_dropped = change.new_setup_fee < change.old_setup_fee;
        let ppu_dropped = change.old_price_per_unit
            .zip(change.new_price_per_unit)
            .is_some_and(|(o, n)| n < o);
        let opu_dropped = change.old_overage_price_per_unit
            .zip(change.new_overage_price_per_unit)
            .is_some_and(|(o, n)| n < o);
        let direction = if monthly_dropped || setup_dropped || ppu_dropped || opu_dropped {
            "down"
        } else {
            "up"
        };
        let title = format!("Saved offering price {}", direction);

        let body = if changes.len() == 1 {
            format!("{}: {}.", change.offer_name, changes[0])
        } else {
            format!("{} pricing changed: {}.", change.offer_name, changes.join("; "))
        };

        for user_pubkey in &saved_user_pubkeys {
            insert_notification(
                &mut **tx,
                user_pubkey,
                "saved_offering_price_change",
                &title,
                &body,
                None,
                Some(offering_id),
                Some(direction),
            )
            .await?;
        }

        Ok(())
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
            reliability_score: _,
            is_draft,
            publish_at,
            is_example: _,
            offering_source,
            external_checkout_url,
            reseller_name: _,
            reseller_commission_percent: _,
            owner_username: _,
            provisioner_type,
            provisioner_config,
            template_name,
            agent_pool_id,
            post_provision_script,
            provider_online: _,
            resolved_pool_id: _,
            resolved_pool_name: _,
            created_at_ns: _,
        } = params;

        // If template_name is provided and provisioner_config is empty, build it
        let provisioner_config = if template_name.is_some() && provisioner_config.is_none() {
            // If template_name is a number, use it as template_vmid
            if let Some(ref name) = template_name {
                if let Ok(vmid) = name.parse::<u32>() {
                    Some(serde_json::json!({"template_vmid": vmid}).to_string())
                } else {
                    // For non-numeric names, let the provider specify mapping via provisioner_config
                    provisioner_config
                }
            } else {
                provisioner_config
            }
        } else {
            provisioner_config
        };

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

        let created_at_ns = crate::now_ns()?;

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
                external_checkout_url, provisioner_type, provisioner_config, template_name, agent_pool_id,
                post_provision_script, is_draft, publish_at, created_at_ns
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
                $51, $52, $53, $54, $55,
                $56, $57, $58, $59
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
            template_name,
            agent_pool_id,
            post_provision_script,
            is_draft,
            publish_at,
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

        // Verify ownership and capture the current price for notifications.
        let existing_offering = sqlx::query_as::<_, (Vec<u8>, String, String, f64, f64, Option<f64>, Option<f64>)>(
            "SELECT pubkey, offer_name, currency, monthly_price, setup_fee, price_per_unit, overage_price_per_unit FROM provider_offerings WHERE id = $1",
        )
        .bind(offering_db_id)
        .fetch_optional(&mut *tx)
        .await?;

        let existing_offering = match existing_offering {
            None => return Err(anyhow::anyhow!("Offering not found")),
            Some((owner_pubkey, _, _, _, _, _, _)) if owner_pubkey != pubkey => {
                return Err(anyhow::anyhow!(
                    "Unauthorized: You do not own this offering"
                ))
            }
            Some(existing_offering) => existing_offering,
        };

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
            reliability_score: _,
            is_draft,
            publish_at,
            is_example: _,
            offering_source,
            external_checkout_url,
            reseller_name: _,
            reseller_commission_percent: _,
            owner_username: _,
            provisioner_type,
            provisioner_config,
            template_name,
            agent_pool_id,
            post_provision_script,
            provider_online: _,
            resolved_pool_id: _,
            resolved_pool_name: _,
            created_at_ns: _,
        } = params;

        // If template_name is provided and provisioner_config is empty, build it
        let provisioner_config = if template_name.is_some() && provisioner_config.is_none() {
            // If template_name is a number, use it as template_vmid
            if let Some(ref name) = template_name {
                if let Ok(vmid) = name.parse::<u32>() {
                    Some(serde_json::json!({"template_vmid": vmid}).to_string())
                } else {
                    // For non-numeric names, let the provider specify mapping via provisioner_config
                    provisioner_config
                }
            } else {
                provisioner_config
            }
        } else {
            provisioner_config
        };

        let (_, _, existing_currency, existing_monthly_price, existing_setup_fee, existing_price_per_unit, existing_overage_price_per_unit) = &existing_offering;
        let price_changed = (*existing_monthly_price - monthly_price).abs() >= 1e-9
            || (*existing_setup_fee - setup_fee).abs() >= 1e-9
            || opt_f64_changed(*existing_price_per_unit, price_per_unit)
            || opt_f64_changed(*existing_overage_price_per_unit, overage_price_per_unit);
        let price_change = SavedOfferingPriceChange {
            offer_name: offer_name.clone(),
            old_currency: existing_currency.clone(),
            old_monthly_price: *existing_monthly_price,
            new_currency: currency.clone(),
            new_monthly_price: monthly_price,
            old_setup_fee: *existing_setup_fee,
            new_setup_fee: setup_fee,
            old_price_per_unit: *existing_price_per_unit,
            new_price_per_unit: price_per_unit,
            old_overage_price_per_unit: *existing_overage_price_per_unit,
            new_overage_price_per_unit: overage_price_per_unit,
        };

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
                provisioner_type = $51, provisioner_config = $52, template_name = $53, agent_pool_id = $54,
                post_provision_script = $55, is_draft = $56, publish_at = $57
            WHERE id = $58"#,
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
            template_name,
            agent_pool_id,
            post_provision_script,
            is_draft,
            publish_at,
            offering_db_id
        )
        .execute(&mut *tx)
        .await?;

        if price_changed {
            self.notify_saved_offering_price_change(&mut tx, offering_db_id, &price_change)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Publish all draft offerings whose publish_at timestamp has passed.
    /// Returns the number of offerings published.
    pub async fn publish_scheduled_offerings(&self) -> Result<u64> {
        let result = sqlx::query!(
            "UPDATE provider_offerings SET is_draft = false, publish_at = NULL \
             WHERE is_draft = true AND publish_at IS NOT NULL AND publish_at <= NOW()"
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Publish multiple draft offerings belonging to the given provider.
    ///
    /// Sets `is_draft = false` and `publish_at = NULL` for all matching IDs.
    /// Only offerings that are currently drafts and owned by the provider are affected.
    /// Returns the list of IDs that were actually published (IDs that were drafts).
    pub async fn bulk_publish_offerings(
        &self,
        provider_pubkey: &[u8],
        offering_ids: &[i64],
    ) -> Result<Vec<i64>> {
        if offering_ids.is_empty() {
            return Err(anyhow::anyhow!("offering_ids must not be empty"));
        }

        let result = sqlx::query_scalar!(
            "UPDATE provider_offerings SET is_draft = false, publish_at = NULL \
             WHERE id = ANY($1) AND pubkey = $2 AND is_draft = TRUE \
             RETURNING id",
            offering_ids,
            provider_pubkey,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
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
            reliability_score: None,
            is_draft: source.is_draft,
            publish_at: None,
            is_example: false,
            offering_source: source.offering_source,
            external_checkout_url: source.external_checkout_url,
            reseller_name: None,
            reseller_commission_percent: None,
            owner_username: None,
            provisioner_type: source.provisioner_type,
            provisioner_config: source.provisioner_config,
            template_name: source.template_name,
            agent_pool_id: source.agent_pool_id,
            post_provision_script: source.post_provision_script,
            provider_online: None,
            resolved_pool_id: None,
            resolved_pool_name: None,
            created_at_ns: None,
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

    /// Bulk update monthly_price for multiple offerings.
    ///
    /// Accepts a list of `(id, price_e9s)` pairs where `price_e9s` is the price in nanocents
    /// (1 USD = 1_000_000_000 price_e9s). Converts to `monthly_price` float for storage.
    /// All updates execute in a single transaction; ownership is verified atomically.
    /// Returns the count of rows updated.
    pub async fn bulk_update_offering_prices(
        &self,
        pubkey: &[u8],
        updates: &[(i64, i64)],
    ) -> Result<u64> {
        if updates.is_empty() {
            return Ok(0);
        }

        let ids: Vec<i64> = updates.iter().map(|(id, _)| *id).collect();

        let mut tx = self.pool.begin().await?;

        // Verify all offerings belong to this provider atomically and capture current prices.
        let id_placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${}", i)).collect();
        let pubkey_placeholder = format!("${}", ids.len() + 1);
        let verify_query = format!(
            "SELECT id, offer_name, currency, monthly_price, setup_fee, price_per_unit, overage_price_per_unit FROM provider_offerings WHERE id IN ({}) AND pubkey = {}",
            id_placeholders.join(","),
            pubkey_placeholder
        );

        type ExistingOfferingRow = (i64, String, String, f64, f64, Option<f64>, Option<f64>);
        type ExistingOfferingPrices = (String, String, f64, f64, Option<f64>, Option<f64>);

        let mut verify_builder = sqlx::query_as::<_, ExistingOfferingRow>(&verify_query);
        for id in &ids {
            verify_builder = verify_builder.bind(id);
        }
        verify_builder = verify_builder.bind(pubkey);

        let existing_offerings = verify_builder.fetch_all(&mut *tx).await?;
        if existing_offerings.len() != ids.len() {
            return Err(anyhow::anyhow!(
                "Not all offerings belong to this provider or some IDs are invalid"
            ));
        }

        let existing_offerings: HashMap<i64, ExistingOfferingPrices> = existing_offerings
            .into_iter()
            .map(|(id, offer_name, currency, monthly_price, setup_fee, price_per_unit, overage_price_per_unit)| {
                (id, (offer_name, currency, monthly_price, setup_fee, price_per_unit, overage_price_per_unit))
            })
            .collect();

        // Update each offering's price within the transaction
        let mut rows_affected = 0u64;
        for (id, price_e9s) in updates {
            let (offer_name, currency, old_monthly_price, old_setup_fee, old_price_per_unit, old_overage_price_per_unit) = existing_offerings
                .get(id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Offering {} missing after ownership verification", id))?;
            let monthly_price = *price_e9s as f64 / 1_000_000_000.0;
            let result = sqlx::query!(
                "UPDATE provider_offerings SET monthly_price = $1 WHERE id = $2",
                monthly_price,
                id
            )
            .execute(&mut *tx)
            .await?;

            if (old_monthly_price - monthly_price).abs() >= 1e-9 {
                let price_change = SavedOfferingPriceChange {
                    offer_name,
                    old_currency: currency.clone(),
                    old_monthly_price,
                    new_currency: currency,
                    new_monthly_price: monthly_price,
                    old_setup_fee,
                    new_setup_fee: old_setup_fee,
                    old_price_per_unit,
                    new_price_per_unit: old_price_per_unit,
                    old_overage_price_per_unit,
                    new_overage_price_per_unit: old_overage_price_per_unit,
                };
                self.notify_saved_offering_price_change(&mut tx, *id, &price_change)
                    .await?;
            }

            rows_affected += result.rows_affected();
        }

        tx.commit().await?;
        Ok(rows_affected)
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

    /// Import seeded offerings from CSV data with offering_source='seeded'.
    /// Used by: `api-cli import-seeded-offerings` command
    /// Returns (success_count, errors) where errors is Vec<(row_number, error_message)>
    #[allow(dead_code)] // Used by api-cli binary, not api-server
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
            reliability_score: None,
            is_draft: get_bool("is_draft"),
            publish_at: None,
            is_example: false,
            offering_source: get_opt_str("offering_source"),
            external_checkout_url: get_opt_str("external_checkout_url"),
            reseller_name: None,
            reseller_commission_percent: None,
            owner_username: None,
            provisioner_type: get_opt_str("provisioner_type"),
            provisioner_config: get_opt_str("provisioner_config"),
            template_name: get_opt_str("template_name"),
            agent_pool_id: get_opt_str("agent_pool_id"),
            post_provision_script: get_opt_str("post_provision_script"),
            provider_online: None,
            resolved_pool_id: None,
            resolved_pool_name: None,
            created_at_ns: None,
        })
    }
}

impl Database {
    /// Save an offering to a user's watchlist (upsert — silently succeeds if already saved).
    pub async fn save_offering(&self, user_pubkey: &[u8], offering_id: i64) -> Result<()> {
        let saved_at = crate::now_ns()?;
        sqlx::query!(
            "INSERT INTO saved_offerings (user_pubkey, offering_id, saved_at) VALUES ($1, $2, $3) ON CONFLICT (user_pubkey, offering_id) DO NOTHING",
            user_pubkey,
            offering_id,
            saved_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Remove an offering from a user's watchlist.
    pub async fn unsave_offering(&self, user_pubkey: &[u8], offering_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM saved_offerings WHERE user_pubkey = $1 AND offering_id = $2",
            user_pubkey,
            offering_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get all saved offerings for a user, joined with full offering data.
    pub async fn get_saved_offerings(&self, user_pubkey: &[u8]) -> Result<Vec<Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let offerings = sqlx::query_as::<_, Offering>(
            r#"SELECT o.id, lower(encode(o.pubkey, 'hex')) as pubkey, o.offering_id, o.offer_name, o.description, o.product_page_url, o.currency, o.monthly_price,
               o.setup_fee, o.visibility, o.product_type, o.virtualization_type, o.billing_interval,
               o.billing_unit, o.pricing_model, o.price_per_unit, o.included_units, o.overage_price_per_unit, o.stripe_metered_price_id,
               o.is_subscription, o.subscription_interval_days,
               o.stock_status, o.processor_brand, o.processor_amount, o.processor_cores, o.processor_speed, o.processor_name,
               o.memory_error_correction, o.memory_type, o.memory_amount, o.hdd_amount, o.total_hdd_capacity,
               o.ssd_amount, o.total_ssd_capacity, o.unmetered_bandwidth, o.uplink_speed, o.traffic,
               o.datacenter_country, o.datacenter_city, o.datacenter_latitude, o.datacenter_longitude,
               o.control_panel, o.gpu_name, o.gpu_count, o.gpu_memory_gb, o.min_contract_hours, o.max_contract_hours,
               o.payment_methods, o.features, o.operating_systems,
               NULL as trust_score, NULL as has_critical_flags, NULL::DOUBLE PRECISION as reliability_score,
               o.is_draft, o.publish_at, CASE WHEN lower(encode(o.pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example,
               o.offering_source, o.external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               o.provisioner_type, o.provisioner_config, o.template_name, o.agent_pool_id, o.post_provision_script,
               NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings o
               INNER JOIN saved_offerings s ON o.id = s.offering_id AND s.user_pubkey = $2
               ORDER BY s.saved_at DESC"#
        )
        .bind(example_provider_pubkey)
        .bind(user_pubkey)
        .fetch_all(&self.pool)
        .await?;
        Ok(offerings)
    }

    /// Check whether a specific offering is saved by the user.
    #[cfg(test)]
    pub async fn is_offering_saved(&self, user_pubkey: &[u8], offering_id: i64) -> Result<bool> {
        let count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64" FROM saved_offerings WHERE user_pubkey = $1 AND offering_id = $2"#,
            user_pubkey,
            offering_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// Get IDs of all offerings saved by the user (for bulk highlighting).
    pub async fn get_saved_offering_ids(&self, user_pubkey: &[u8]) -> Result<Vec<i64>> {
        let ids = sqlx::query_scalar!(
            r#"SELECT offering_id as "offering_id!: i64" FROM saved_offerings WHERE user_pubkey = $1 ORDER BY saved_at DESC"#,
            user_pubkey
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(ids)
    }

    /// Record a view for an offering. Deduplicates by (offering_id, ip_hash, day).
    /// Returns true if a new view was recorded, false if it was a duplicate.
    pub async fn record_offering_view(
        &self,
        offering_id: i64,
        viewer_pubkey: Option<&[u8]>,
        ip_hash: &[u8],
    ) -> Result<bool> {
        let viewed_at = chrono::Utc::now().timestamp_millis();
        let rows_affected = sqlx::query(
            r#"INSERT INTO offering_views (offering_id, viewer_pubkey, ip_hash, viewed_at)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (offering_id, ip_hash, (viewed_at / 86400000)) DO NOTHING"#,
        )
        .bind(offering_id)
        .bind(viewer_pubkey)
        .bind(ip_hash)
        .bind(viewed_at)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(rows_affected > 0)
    }

    /// Get analytics for an offering: view counts and unique viewer counts for 7d and 30d windows.
    pub async fn get_offering_analytics(&self, offering_id: i64) -> Result<OfferingAnalytics> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff_7d = now_ms - 7 * 24 * 60 * 60 * 1000i64;
        let cutoff_30d = now_ms - 30 * 24 * 60 * 60 * 1000i64;
        let row = sqlx::query_as::<_, OfferingAnalytics>(
            r#"SELECT
                COUNT(*) FILTER (WHERE viewed_at >= $2) AS views_7d,
                COUNT(*) FILTER (WHERE viewed_at >= $3) AS views_30d,
                COUNT(DISTINCT ip_hash) FILTER (WHERE viewed_at >= $2) AS unique_viewers_7d,
                COUNT(DISTINCT ip_hash) FILTER (WHERE viewed_at >= $3) AS unique_viewers_30d
               FROM offering_views WHERE offering_id = $1"#,
        )
        .bind(offering_id)
        .bind(cutoff_7d)
        .bind(cutoff_30d)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Get daily view trends for an offering over the last `days` days, ordered by day ASC.
    pub async fn get_offering_view_trends(
        &self,
        offering_id: i64,
        days: i64,
    ) -> Result<Vec<DailyViewTrend>> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff_ms = now_ms - days * 86_400_000i64;
        let rows = sqlx::query_as::<_, DailyViewTrend>(
            r#"SELECT
                to_char(to_timestamp(viewed_at / 1000), 'YYYY-MM-DD') AS day,
                COUNT(*)::BIGINT AS views,
                COUNT(DISTINCT ip_hash)::BIGINT AS unique_viewers
               FROM offering_views
               WHERE offering_id = $1
                 AND viewed_at >= $2
               GROUP BY day
               ORDER BY day ASC"#,
        )
        .bind(offering_id)
        .bind(cutoff_ms)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Get the top N offerings by view count in the last 7 days.
    /// Only public, non-draft, in-stock offerings are considered.
    /// `limit` is capped at 10.
    pub async fn get_trending_offerings(&self, limit: i64) -> Result<Vec<TrendingOffering>> {
        let limit = limit.min(10);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff_7d = now_ms - 7 * 24 * 60 * 60 * 1000i64;
        let rows = sqlx::query_as::<_, TrendingOffering>(
            r#"SELECT
                o.id AS offering_id,
                o.offer_name,
                lower(encode(o.pubkey, 'hex')) AS pubkey,
                o.product_type,
                o.monthly_price,
                o.currency,
                o.datacenter_country,
                o.datacenter_city,
                NULL::DOUBLE PRECISION AS trust_score,
                COUNT(v.id)::BIGINT AS views_7d
               FROM provider_offerings o
               JOIN offering_views v ON v.offering_id = o.id AND v.viewed_at >= $1
               WHERE LOWER(o.visibility) = 'public'
                 AND o.is_draft = false
                 AND o.stock_status != 'out_of_stock'
               GROUP BY o.id
               ORDER BY views_7d DESC
               LIMIT $2"#,
        )
        .bind(cutoff_7d)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Get pricing statistics (min, max, avg, median) for offerings matching the given filters.
    pub async fn get_offering_pricing_stats(
        &self,
        product_type: &str,
        country: Option<&str>,
    ) -> Result<OfferingPricingStats> {
        let stats = sqlx::query_as::<_, OfferingPricingStats>(
            r#"SELECT
                COUNT(*)::bigint as count,
                COALESCE(MIN(monthly_price), 0) as min_price,
                COALESCE(MAX(monthly_price), 0) as max_price,
                COALESCE(AVG(monthly_price), 0) as avg_price,
                COALESCE(PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY monthly_price), 0) as median_price
            FROM provider_offerings
            WHERE LOWER(visibility) = 'public'
              AND is_draft = false
              AND product_type = $1
              AND ($2::text IS NULL OR datacenter_country = $2)"#,
        )
        .bind(product_type)
        .bind(country)
        .fetch_one(&self.pool)
        .await?;
        Ok(stats)
    }

    /// PoC(338): Get personalized recommended offerings for a user.
    ///
    /// Content-based approach:
    ///   1. Gather user signals from `offering_views` and `saved_offerings`
    ///   2. Build a preference profile (preferred types, countries, GPUs, price range)
    ///   3. Score all eligible offerings by attribute similarity
    ///   4. Exclude already-seen offerings, return top N by score
    ///
    /// Returns empty vec if the user has no viewing/saving history.
    pub async fn get_recommended_offerings(
        &self,
        user_pubkey: &[u8],
        limit: i64,
    ) -> Result<Vec<RecommendedOffering>> {
        let limit = limit.min(10);

        let signals = self.fetch_user_signal_offerings(user_pubkey).await?;
        if signals.is_empty() {
            return Ok(vec![]);
        }

        let profile = build_preference_profile(&signals);
        let seen_ids = self.fetch_seen_offering_ids(user_pubkey).await?;
        let candidates = self.fetch_candidate_offerings(&seen_ids, limit * 5).await?;

        let mut scored: Vec<RecommendedOffering> = candidates
            .into_iter()
            .map(|c| score_candidate(&c, &profile))
            .filter(|r| r.score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit as usize);

        Ok(scored)
    }

    /// Fetch attributes of offerings the user has viewed or saved
    async fn fetch_user_signal_offerings(
        &self,
        user_pubkey: &[u8],
    ) -> Result<Vec<SignalOffering>> {
        let rows = sqlx::query_as::<_, SignalOffering>(
            r#"SELECT o.product_type, o.datacenter_country, o.gpu_name, o.monthly_price
               FROM provider_offerings o
               WHERE o.id IN (
                   SELECT v.offering_id FROM offering_views v WHERE v.viewer_pubkey = $1
                   UNION
                   SELECT s.offering_id FROM saved_offerings s WHERE s.user_pubkey = $1
               )
               AND LOWER(o.visibility) = 'public' AND o.is_draft = false"#,
        )
        .bind(user_pubkey)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Fetch IDs of offerings the user has already seen (viewed or saved)
    async fn fetch_seen_offering_ids(
        &self,
        user_pubkey: &[u8],
    ) -> Result<HashSet<i64>> {
        let rows = sqlx::query(
            r#"SELECT offering_id FROM offering_views WHERE viewer_pubkey = $1
               UNION
               SELECT offering_id FROM saved_offerings WHERE user_pubkey = $1"#,
        )
        .bind(user_pubkey)
        .fetch_all(&self.pool)
        .await?;
        let ids: HashSet<i64> = rows.iter().filter_map(|r| r.try_get("offering_id").ok()).collect();
        Ok(ids)
    }

    /// Fetch candidate offerings to score (public, non-draft, in-stock, excluding seen)
    async fn fetch_candidate_offerings(
        &self,
        seen_ids: &HashSet<i64>,
        limit: i64,
    ) -> Result<Vec<CandidateOffering>> {
        let mut rows = sqlx::query_as::<_, CandidateOffering>(
            r#"SELECT o.id as offering_id, o.offer_name, lower(encode(o.pubkey, 'hex')) as pubkey,
                      o.product_type, o.monthly_price, o.currency,
                      o.datacenter_country, o.datacenter_city,
                      p.trust_score as trust_score,
                      o.gpu_name
               FROM provider_offerings o
               LEFT JOIN provider_profiles p ON o.pubkey = p.pubkey
               WHERE LOWER(o.visibility) = 'public'
                 AND o.is_draft = false
                 AND o.stock_status != 'out_of_stock'
               ORDER BY p.reliability_score DESC NULLS LAST, o.monthly_price ASC
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.retain(|c| !seen_ids.contains(&c.offering_id));
        Ok(rows)
    }
}

/// Internal struct for candidate offerings fetched from DB for scoring
#[derive(Debug, Clone, sqlx::FromRow)]
struct CandidateOffering {
    offering_id: i64,
    offer_name: String,
    pubkey: String,
    product_type: String,
    monthly_price: f64,
    currency: String,
    datacenter_country: Option<String>,
    datacenter_city: Option<String>,
    trust_score: Option<f64>,
    gpu_name: Option<String>,
}

fn build_preference_profile(signals: &[SignalOffering]) -> UserPreferenceProfile {
    let mut type_counts: HashMap<String, f64> = HashMap::new();
    let mut country_counts: HashMap<String, f64> = HashMap::new();
    let mut gpu_counts: HashMap<String, f64> = HashMap::new();
    let mut prices: Vec<f64> = Vec::new();

    for s in signals {
        *type_counts.entry(s.product_type.clone()).or_default() += 1.0;
        *country_counts.entry(s.datacenter_country.clone()).or_default() += 1.0;
        if let Some(ref g) = s.gpu_name {
            *gpu_counts.entry(g.clone()).or_default() += 1.0;
        }
        prices.push(s.monthly_price);
    }

    let (avg_price, price_stddev) = if prices.len() >= 2 {
        let avg = prices.iter().sum::<f64>() / prices.len() as f64;
        let variance = prices.iter().map(|p| (p - avg).powi(2)).sum::<f64>() / prices.len() as f64;
        (Some(avg), Some(variance.sqrt()))
    } else if prices.len() == 1 {
        (Some(prices[0]), None)
    } else {
        (None, None)
    };

    UserPreferenceProfile {
        preferred_types: type_counts,
        preferred_countries: country_counts,
        preferred_gpus: gpu_counts,
        avg_price,
        price_stddev,
    }
}

fn score_candidate(candidate: &CandidateOffering, profile: &UserPreferenceProfile) -> RecommendedOffering {
    let mut score = 0.0;

    if let Some(w) = profile.preferred_types.get(&candidate.product_type) {
        score += w * 3.0;
    }
    if let Some(ref country) = candidate.datacenter_country {
        if let Some(w) = profile.preferred_countries.get(country) {
            score += w * 2.0;
        }
    }
    if let Some(ref gpu) = candidate.gpu_name {
        if let Some(w) = profile.preferred_gpus.get(gpu) {
            score += w * 4.0;
        }
    }
    if let Some(avg) = profile.avg_price {
        if let Some(stddev) = profile.price_stddev {
            if stddev > 0.0 {
                let z = (candidate.monthly_price - avg).abs() / stddev;
                score += (1.0 - z.min(2.0) / 2.0).max(0.0) * 1.0;
            }
        } else if (candidate.monthly_price - avg).abs() <= avg * 0.5 {
            score += 0.5;
        }
    }

    RecommendedOffering {
        offering_id: candidate.offering_id,
        offer_name: candidate.offer_name.clone(),
        pubkey: candidate.pubkey.clone(),
        product_type: candidate.product_type.clone(),
        monthly_price: candidate.monthly_price,
        currency: candidate.currency.clone(),
        datacenter_country: candidate.datacenter_country.clone(),
        datacenter_city: candidate.datacenter_city.clone(),
        trust_score: candidate.trust_score,
        gpu_name: candidate.gpu_name.clone(),
        score,
    }
}

#[cfg(test)]
mod tests;

/// PoC(338): Unit tests for the pure recommendation-engine functions.
/// These tests cover build_preference_profile and score_candidate without needing a DB.
#[cfg(test)]
mod recommendation_tests {
    use super::*;

    fn make_signal(product_type: &str, country: &str, gpu: Option<&str>, price: f64) -> SignalOffering {
        SignalOffering {
            product_type: product_type.to_string(),
            datacenter_country: country.to_string(),
            gpu_name: gpu.map(|s| s.to_string()),
            monthly_price: price,
        }
    }

    fn make_candidate(
        id: i64,
        product_type: &str,
        country: Option<&str>,
        gpu: Option<&str>,
        price: f64,
    ) -> CandidateOffering {
        CandidateOffering {
            offering_id: id,
            offer_name: format!("offering-{id}"),
            pubkey: "aabbcc".to_string(),
            product_type: product_type.to_string(),
            monthly_price: price,
            currency: "USD".to_string(),
            datacenter_country: country.map(|s| s.to_string()),
            datacenter_city: None,
            trust_score: None,
            gpu_name: gpu.map(|s| s.to_string()),
        }
    }

    /// Empty signal list produces zero-score candidates
    #[test]
    fn test_empty_signals_profile() {
        let profile = build_preference_profile(&[]);
        assert!(profile.preferred_types.is_empty());
        assert!(profile.preferred_countries.is_empty());
        assert!(profile.preferred_gpus.is_empty());
        assert!(profile.avg_price.is_none());
        assert!(profile.price_stddev.is_none());

        let candidate = make_candidate(1, "gpu", Some("US"), Some("NVIDIA A100"), 100.0);
        let scored = score_candidate(&candidate, &profile);
        assert_eq!(scored.score, 0.0, "no signals -> zero score");
    }

    /// Single GPU-matching signal scores highest on GPU weight (4x)
    #[test]
    fn test_gpu_match_scores_highest() {
        let signals = vec![make_signal("gpu", "US", Some("NVIDIA A100"), 500.0)];
        let profile = build_preference_profile(&signals);

        let gpu_match = make_candidate(1, "gpu", Some("US"), Some("NVIDIA A100"), 500.0);
        let no_gpu = make_candidate(2, "gpu", Some("US"), None, 500.0);

        let scored_gpu = score_candidate(&gpu_match, &profile);
        let scored_no_gpu = score_candidate(&no_gpu, &profile);

        // gpu match (+4) + type match (+3) + country match (+2) = 9 vs type (+3) + country (+2) = 5
        assert!(
            scored_gpu.score > scored_no_gpu.score,
            "GPU match should outscore no-GPU: {:.1} vs {:.1}",
            scored_gpu.score,
            scored_no_gpu.score
        );
        // GPU weight is 4, so GPU-match score should include 4.0 contribution
        assert!(scored_gpu.score >= 4.0);
    }

    /// Country mismatch should score lower than country match
    #[test]
    fn test_country_mismatch_scores_lower() {
        let signals = vec![make_signal("compute", "DE", None, 100.0)];
        let profile = build_preference_profile(&signals);

        let same_country = make_candidate(1, "compute", Some("DE"), None, 100.0);
        let diff_country = make_candidate(2, "compute", Some("US"), None, 100.0);

        let score_same = score_candidate(&same_country, &profile).score;
        let score_diff = score_candidate(&diff_country, &profile).score;

        assert!(
            score_same > score_diff,
            "Same country ({score_same:.1}) should beat different country ({score_diff:.1})"
        );
    }

    /// Price within user's avg range gets a bonus; far price gets none
    #[test]
    fn test_price_proximity_bonus() {
        let signals = vec![
            make_signal("compute", "US", None, 100.0),
            make_signal("compute", "US", None, 120.0),
        ];
        let profile = build_preference_profile(&signals);

        assert!(profile.avg_price.is_some());
        // avg ~110, stddev ~10
        let close_price = make_candidate(1, "compute", Some("US"), None, 110.0);
        let far_price = make_candidate(2, "compute", Some("US"), None, 500.0);

        let close_score = score_candidate(&close_price, &profile).score;
        let far_score = score_candidate(&far_price, &profile).score;

        assert!(
            close_score > far_score,
            "Close price ({close_score:.2}) should beat far price ({far_score:.2})"
        );
    }

    /// build_preference_profile aggregates type counts correctly
    #[test]
    fn test_profile_type_aggregation() {
        let signals = vec![
            make_signal("gpu", "US", None, 100.0),
            make_signal("gpu", "DE", None, 200.0),
            make_signal("compute", "US", None, 50.0),
        ];
        let profile = build_preference_profile(&signals);

        assert_eq!(profile.preferred_types.get("gpu"), Some(&2.0));
        assert_eq!(profile.preferred_types.get("compute"), Some(&1.0));
        assert_eq!(profile.preferred_countries.get("US"), Some(&2.0));
        assert_eq!(profile.preferred_countries.get("DE"), Some(&1.0));
    }
}
