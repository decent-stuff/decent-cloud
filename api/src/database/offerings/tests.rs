use super::*;
use crate::database::test_helpers::setup_test_db;

/// Helper to register a provider (required due to foreign key constraint on agent_pools)
async fn register_provider(db: &Database, pubkey: &[u8]) {
    sqlx::query(
        "INSERT INTO provider_registrations (pubkey, signature, created_at_ns) VALUES ($1, '\\x00', 0)",
    )
    .bind(pubkey)
    .execute(&db.pool)
    .await
    .expect("Failed to register provider for test");
}

/// Helper to insert test offering. Also ensures provider is registered and has a pool for the region.
async fn insert_test_offering(db: &Database, id: i64, pubkey: &[u8], country: &str, price: f64) {
    // Ensure provider is registered and has a pool for the region
    ensure_provider_with_pool(db, pubkey, country).await;

    // Use IDs starting from 100 to avoid conflicts with example data from migration 002
    let db_id = id + 100;
    let offering_id = format!("off-{}", id);
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, payment_methods, features, operating_systems, created_at_ns) VALUES ($1, $2, $3, 'Test Offer', 'USD', $4, 0, 'public', 'compute', 'monthly', 'in_stock', $5, 'City', FALSE, NULL, NULL, NULL, 0)",
        db_id,
        pubkey,
        offering_id,
        price,
        country
    )
    .execute(&db.pool)
    .await
    .expect("Failed to insert test offering");
}

/// Ensures provider is registered and has a pool for the country's region with an online agent.
/// Uses ON CONFLICT DO NOTHING to avoid conflicts if called multiple times for same provider.
async fn ensure_provider_with_pool(db: &Database, pubkey: &[u8], country: &str) {
    use crate::regions::country_to_region;

    // Register provider if not exists
    sqlx::query("INSERT INTO provider_registrations (pubkey, signature, created_at_ns) VALUES ($1, '\\x00', 0) ON CONFLICT (pubkey) DO NOTHING")
        .bind(pubkey)
        .execute(&db.pool)
        .await
        .expect("Failed to register provider in ensure_provider_with_pool");

    // Get region for country - all test offerings need a matching pool for marketplace visibility
    let region = country_to_region(country)
        .unwrap_or_else(|| panic!("Test country '{}' has no region mapping", country));

    // Create pool for region if not exists
    let pool_id = format!("pool-{}-{}", region, hex::encode(&pubkey[..4]));
    let pool_name = format!("Test Pool {}", region);
    sqlx::query("INSERT INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns) VALUES ($1, $2, $3, $4, 'manual', 0) ON CONFLICT (pool_id) DO NOTHING")
        .bind(&pool_id)
        .bind(pubkey)
        .bind(&pool_name)
        .bind(region)
        .execute(&db.pool)
        .await
        .expect("Failed to create agent pool in ensure_provider_with_pool");

    // Add an online agent to the pool for marketplace visibility
    // Use deterministic agent pubkey based on provider pubkey and region
    let mut agent_pubkey = [0u8; 32];
    let pubkey_len = pubkey.len().min(16);
    agent_pubkey[..pubkey_len].copy_from_slice(&pubkey[..pubkey_len]);
    let region_bytes = region.as_bytes();
    let region_len = region_bytes.len().min(16);
    agent_pubkey[16..16 + region_len].copy_from_slice(&region_bytes[..region_len]);

    // Register agent delegation
    sqlx::query("INSERT INTO provider_agent_delegations (provider_pubkey, agent_pubkey, permissions, expires_at_ns, label, signature, created_at_ns, pool_id) VALUES ($1, $2, '[]', NULL, 'Test Agent', '\\x00', 0, $3) ON CONFLICT (agent_pubkey) DO NOTHING")
        .bind(pubkey)
        .bind(&agent_pubkey[..])
        .bind(&pool_id)
        .execute(&db.pool)
        .await
        .expect("Failed to register agent delegation in ensure_provider_with_pool");

    // Mark provider as online (recent heartbeat)
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    sqlx::query("INSERT INTO provider_agent_status (provider_pubkey, online, last_heartbeat_ns, updated_at_ns) VALUES ($1, TRUE, $2, $3) ON CONFLICT (provider_pubkey) DO UPDATE SET online = TRUE, last_heartbeat_ns = excluded.last_heartbeat_ns, updated_at_ns = excluded.updated_at_ns")
        .bind(pubkey)
        .bind(now_ns)
        .bind(now_ns)
        .execute(&db.pool)
        .await
        .expect("Failed to update provider agent status in ensure_provider_with_pool");
}

// Helper to get the database ID from test ID (test IDs start from 1, DB IDs from 100)
fn test_id_to_db_id(test_id: i64) -> i64 {
    test_id + 100
}

/// Delete example provider data so tests get clean counts.
/// Migration 054 creates pools for example provider, making example offerings visible in search.
async fn delete_example_data(db: &Database) {
    let example_pubkey = Database::example_provider_pubkey();
    // Delete in correct order to respect foreign key constraints
    sqlx::query("DELETE FROM provider_agent_delegations WHERE provider_pubkey = $1")
        .bind(&example_pubkey[..])
        .execute(&db.pool)
        .await
        .expect("Failed to delete example provider agent delegations");
    sqlx::query("DELETE FROM agent_pools WHERE provider_pubkey = $1")
        .bind(&example_pubkey[..])
        .execute(&db.pool)
        .await
        .expect("Failed to delete example provider agent pools");
    sqlx::query("DELETE FROM provider_offerings WHERE pubkey = $1")
        .bind(&example_pubkey[..])
        .execute(&db.pool)
        .await
        .expect("Failed to delete example provider offerings");
}

#[test]
fn export_typescript_types() {
    // This test ensures TypeScript types are exported
    Offering::export().expect("Failed to export Offering type");
}

#[tokio::test]
async fn test_get_provider_offerings_empty() {
    let db = setup_test_db().await;
    let offerings = db
        .get_provider_offerings(&[1u8; 32])
        .await
        .expect("Failed to get provider offerings");
    assert_eq!(offerings.len(), 0);
}

#[tokio::test]
async fn test_get_provider_offerings() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;
    insert_test_offering(&db, 2, &pubkey, "DE", 200.0).await;

    let offerings = db
        .get_provider_offerings(&pubkey)
        .await
        .expect("Failed to get provider offerings");
    assert_eq!(offerings.len(), 2);
}

#[tokio::test]
async fn test_get_offering_by_id() {
    let db = setup_test_db().await;
    insert_test_offering(&db, 42, &[1u8; 32], "US", 100.0).await;

    let db_id = test_id_to_db_id(42);
    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering by ID");
    assert!(offering.is_some());
    assert_eq!(
        offering.expect("Expected offering to exist").id,
        Some(db_id)
    );
}

#[tokio::test]
async fn test_get_offering_not_found() {
    let db = setup_test_db().await;
    let offering = db.get_offering(999).await.expect("Failed to get offering");
    assert!(offering.is_none());
}

#[tokio::test]
async fn test_count_offerings_no_filters() {
    let db = setup_test_db().await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "DE", 200.0).await;

    let count = db
        .count_offerings(None)
        .await
        .expect("Failed to count offerings");
    // Count includes example offerings from migration 008
    assert!(count >= 2);
}

#[tokio::test]
async fn test_search_offerings_no_filters() {
    let db = setup_test_db().await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "DE", 200.0).await;

    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings");
    // Results include example offerings from migration 008
    assert!(results.len() >= 2);
    // Verify our test offerings are present
    assert!(results.iter().any(|o| o.offering_id == "off-1"));
    assert!(results.iter().any(|o| o.offering_id == "off-2"));
}

#[tokio::test]
async fn test_search_offerings_excludes_private() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Insert public offering (should be shown)
    insert_test_offering(&db, 1, &pubkey, "US", 100.0).await;

    // Insert private offering (should NOT be shown)
    let db_id_private = test_id_to_db_id(2);
    let offering_id_private = "off-2";
    {
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, payment_methods, features, operating_systems, created_at_ns) VALUES ($1, $2, $3, 'Private Offer', 'USD', $4, 0, 'private', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, NULL, NULL, NULL, 0)",
            db_id_private,
            pubkey_ref,
            offering_id_private,
            200.0
        )
        .execute(&db.pool)
        .await
        .expect("Failed to insert private test offering");
    }

    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings");

    // Should return public offerings (including examples), not the private one
    assert!(!results.is_empty());
    assert!(results.iter().any(|o| o.offering_id == "off-1"));
    assert!(results
        .iter()
        .all(|o| o.visibility.to_lowercase() == "public"));
    // Verify private offering is NOT in results
    assert!(!results.iter().any(|o| o.offering_id == "off-2"));
}

#[tokio::test]
async fn test_search_offerings_by_country() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "DE", 200.0).await;

    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: Some("US"),
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 10,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings by country");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].datacenter_country, "US");
}

#[tokio::test]
async fn test_search_offerings_price_range() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 50.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "US", 150.0).await;
    insert_test_offering(&db, 3, &[3u8; 32], "US", 250.0).await;

    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings by price range");
    assert_eq!(results.len(), 3);
    // Results sorted by monthly_price ASC
    assert_eq!(results[0].monthly_price, 50.0);
    assert_eq!(results[1].monthly_price, 150.0);
    assert_eq!(results[2].monthly_price, 250.0);
}

#[tokio::test]
async fn test_search_offerings_min_price_filter() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 50.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "US", 150.0).await;
    insert_test_offering(&db, 3, &[3u8; 32], "US", 250.0).await;

    // Filter for offerings >= $100/month
    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: Some(100.0),
            max_price_monthly: None,
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings with min price filter");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].monthly_price, 150.0);
    assert_eq!(results[1].monthly_price, 250.0);
}

#[tokio::test]
async fn test_search_offerings_max_price_filter() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 50.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "US", 150.0).await;
    insert_test_offering(&db, 3, &[3u8; 32], "US", 250.0).await;

    // Filter for offerings <= $200/month
    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: Some(200.0),
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings with max price filter");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].monthly_price, 50.0);
    assert_eq!(results[1].monthly_price, 150.0);
}

#[tokio::test]
async fn test_search_offerings_price_range_both() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 50.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "US", 150.0).await;
    insert_test_offering(&db, 3, &[3u8; 32], "US", 250.0).await;

    // Filter for offerings between $100 and $200/month
    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: Some(100.0),
            max_price_monthly: Some(200.0),
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings with price range filter");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].monthly_price, 150.0);
}

#[tokio::test]
async fn test_search_offerings_pagination() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;

    // Use a single provider with pool for all offerings (simpler setup)
    let pubkey = vec![99u8; 32];
    ensure_provider_with_pool(&db, &pubkey, "US").await;

    // Create 10 offerings with very LOW prices so they appear first in sort order
    for i in 0..10 {
        let db_id = 200 + i;
        let offering_id = format!("pagination-{}", i);
        sqlx::query(
            "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, created_at_ns) VALUES ($1, $2, $3, 'Pagination Test', 'USD', $4, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0)"
        )
        .bind(db_id)
        .bind(&pubkey)
        .bind(&offering_id)
        .bind(0.01 + i as f64 * 0.01)  // Very low prices: 0.01, 0.02, 0.03, ...
        .execute(&db.pool)
        .await
        .expect("Failed to insert pagination test offering");
    }

    // Get all offerings with high limit - should get all 10 test offerings
    let all = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search all offerings for pagination test");
    // Should have exactly our 10 test offerings (example offerings have no pools)
    assert_eq!(all.len(), 10);

    // Verify all results have our offering IDs
    assert!(all.iter().all(|o| o.offering_id.starts_with("pagination-")));

    // Test pagination - get first 3
    let page1 = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 3,
            offset: 0,
        })
        .await
        .expect("Failed to search first page for pagination test");
    assert_eq!(page1.len(), 3);
    // Verify sorted by price (ascending)
    assert!(page1[0].monthly_price <= page1[1].monthly_price);
    assert!(page1[1].monthly_price <= page1[2].monthly_price);
}

// CRUD Tests
#[tokio::test]
async fn test_create_offering_success() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
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
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: Some(1),
        max_contract_hours: None,
        payment_methods: Some("BTC,ETH".to_string()),
        features: Some("RAID,Backup".to_string()),
        operating_systems: Some("Ubuntu 22.04".to_string()),
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    let offering_id = db
        .create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");
    assert!(offering_id > 0);

    // Verify the offering was created
    let offering = db
        .get_offering(offering_id)
        .await
        .expect("Failed to get created offering");
    assert!(offering.is_some());
    let offering = offering.expect("Expected offering to exist after creation");
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

    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
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
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: Some(1),
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
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

    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
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
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
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
    let db_id = test_id_to_db_id(1);
    let update_params = Offering {
        id: Some(db_id),
        pubkey: hex::encode(&pubkey),
        offering_id: "off-1".to_string(),
        offer_name: "Updated Server".to_string(),
        description: Some("Updated description".to_string()),
        product_page_url: None,
        currency: "DER".to_string(),
        monthly_price: 199.99,
        setup_fee: 50.0,
        visibility: "private".to_string(),
        product_type: "vps".to_string(),
        virtualization_type: Some("kvm".to_string()),
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
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
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: Some("ETH".to_string()),
        features: Some("Backup".to_string()),
        operating_systems: Some("Debian 12".to_string()),
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
        is_subscription: false,
        subscription_interval_days: None,
    };

    let db_id = test_id_to_db_id(1);
    let result = db.update_offering(&pubkey, db_id, update_params).await;
    assert!(result.is_ok());

    // Verify update
    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get updated offering")
        .expect("Expected offering to exist after update");
    assert_eq!(offering.offer_name, "Updated Server");
    assert_eq!(offering.monthly_price, 199.99);
    assert_eq!(offering.currency, "DER");
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

    let db_id = test_id_to_db_id(1);
    let params = Offering {
        id: Some(db_id),
        pubkey: hex::encode(&pubkey2),
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
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

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
    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering after deletion");
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

    // Create offering with payment_methods - let PostgreSQL generate the ID
    let offering_id = "off-dup-test".to_string();
    let db_id: i64 = {
        let pubkey_ref = &pubkey;
        let offering_id_ref = &offering_id;
        sqlx::query_scalar!(
            "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, payment_methods, features, operating_systems, created_at_ns) VALUES ($1, $2, 'Test Offer', 'USD', $3, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 'BTC', NULL, NULL, 0) RETURNING id",
            pubkey_ref,
            offering_id_ref,
            100.0
        )
        .fetch_one(&db.pool)
        .await
        .expect("Failed to insert test offering")
    };

    let new_id = db
        .duplicate_offering(&pubkey, db_id, "off-dup-test-copy".to_string())
        .await
        .expect("Failed to duplicate offering");

    // New ID should be different and valid (auto-generated IDs are always positive)
    assert_ne!(new_id, db_id);
    assert!(new_id > 0);

    // Verify duplication
    let duplicated = db
        .get_offering(new_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
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
    assert_eq!(result.expect("Expected operation to succeed"), 3);

    // Verify all updated
    for id in offering_ids {
        let offering = db
            .get_offering(id)
            .await
            .expect("Failed to get offering")
            .expect("Expected offering to exist");
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
    assert_eq!(result.expect("Expected operation to succeed"), 0);
}

#[tokio::test]
async fn test_csv_import_success() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    let csv_data = "offering_id,offer_name,description,product_page_url,currency,monthly_price,setup_fee,visibility,product_type,virtualization_type,billing_interval,stock_status,processor_brand,processor_amount,processor_cores,processor_speed,processor_name,memory_error_correction,memory_type,memory_amount,hdd_amount,total_hdd_capacity,ssd_amount,total_ssd_capacity,unmetered_bandwidth,uplink_speed,traffic,datacenter_country,datacenter_city,datacenter_latitude,datacenter_longitude,control_panel,gpu_name,min_contract_hours,max_contract_hours,payment_methods,features,operating_systems
off-1,Test Server,Great server,https://example.com,USD,100.0,0.0,public,dedicated,,monthly,in_stock,Intel,2,8,3.5GHz,Xeon,ECC,DDR4,32GB,2,2TB,1,500GB,true,1Gbps,10000,US,New York,40.7128,-74.0060,cPanel,RTX 3090,1,720,BTC,SSD,Ubuntu
off-2,Test Server 2,Another server,,DER,200.0,50.0,public,vps,kvm,monthly,in_stock,,,,,,,,,,,,,false,,,DE,Berlin,,,,,,,\"BTC,ETH\",\"SSD,NVMe\",\"Ubuntu,Debian\"";

    let (success_count, errors) = db
        .import_offerings_csv(&pubkey, csv_data, false)
        .await
        .expect("Failed to import offerings CSV");

    assert_eq!(success_count, 2);
    assert_eq!(errors.len(), 0);

    // Verify first offering
    let off1 = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1"#,
        "off-1"
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to fetch from database");
    let offering = db
        .get_offering(off1)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
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
        .expect("Failed to import offerings CSV");

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
off-2,New Offer,New desc,,DER,150.0,0.0,public,vps,,monthly,in_stock,,,,,,,,,,,,,false,,,DE,Berlin,,,,,,,,,";

    let (success_count, errors) = db
        .import_offerings_csv(&pubkey, csv_data, true)
        .await
        .expect("Failed to import offerings CSV");

    assert_eq!(success_count, 2);
    assert_eq!(errors.len(), 0);

    // Verify update
    let db_id = test_id_to_db_id(1);
    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert_eq!(offering.offer_name, "Updated Offer");
    assert_eq!(offering.monthly_price, 200.0);
    assert_eq!(offering.stock_status, "out_of_stock");

    // Verify new offering was created
    let off2 = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1"#,
        "off-2"
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to fetch from database");
    // Just verify it exists and has a valid ID
    assert!(off2 > 0);
    assert_ne!(off2, db_id);
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
        .expect("Failed to import offerings CSV");

    // Should create new offering for pubkey2, not update pubkey1's offering
    assert_eq!(success_count, 1);
    assert_eq!(errors.len(), 0);

    // Verify original offering unchanged
    let db_id = test_id_to_db_id(1);
    let original = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert_eq!(original.offer_name, "Test Offer");
    assert_eq!(original.monthly_price, 100.0);
}

#[tokio::test]
async fn test_csv_import_column_order_independence() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // CSV with columns in different order (offer_name before offering_id, price columns swapped)
    let csv_data = "offer_name,offering_id,currency,setup_fee,monthly_price,visibility,product_type,billing_interval,stock_status,datacenter_country,datacenter_city,unmetered_bandwidth
Reordered Server,reorder-1,USD,10.0,99.0,public,compute,monthly,in_stock,US,NYC,false";

    let (success_count, errors) = db
        .import_offerings_csv(&pubkey, csv_data, false)
        .await
        .expect("Failed to import offerings CSV");

    assert_eq!(success_count, 1, "errors: {:?}", errors);
    assert_eq!(errors.len(), 0);

    // Verify fields parsed correctly despite different order
    let off = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1"#,
        "reorder-1"
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to fetch from database");
    let offering = db
        .get_offering(off)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert_eq!(offering.offer_name, "Reordered Server");
    assert_eq!(offering.monthly_price, 99.0);
    assert_eq!(offering.setup_fee, 10.0);
    assert_eq!(offering.datacenter_country, "US");
}

#[tokio::test]
async fn test_csv_import_gpu_fields() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // CSV with GPU fields
    let csv_data = "offering_id,offer_name,currency,monthly_price,setup_fee,visibility,product_type,billing_interval,stock_status,datacenter_country,datacenter_city,unmetered_bandwidth,gpu_name,gpu_count,gpu_memory_gb
gpu-1,GPU Server,USD,500.0,0.0,public,gpu,monthly,in_stock,US,NYC,false,NVIDIA A100,4,80";

    let (success_count, errors) = db
        .import_offerings_csv(&pubkey, csv_data, false)
        .await
        .expect("Failed to import offerings CSV");

    assert_eq!(success_count, 1, "errors: {:?}", errors);
    assert_eq!(errors.len(), 0);

    let off = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1"#,
        "gpu-1"
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to fetch from database");
    let offering = db
        .get_offering(off)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert_eq!(offering.product_type, "gpu");
    assert_eq!(offering.gpu_name, Some("NVIDIA A100".to_string()));
    assert_eq!(offering.gpu_count, Some(4));
    assert_eq!(offering.gpu_memory_gb, Some(80));
}

// DSL Search Tests
#[tokio::test]
async fn test_search_offerings_dsl_empty_query() {
    let db = setup_test_db().await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "DE", 200.0).await;

    // Empty query returns all public offerings (including examples)
    let results = db
        .search_offerings_dsl("", 100, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert!(results.len() >= 2);
    assert!(results.iter().any(|o| o.offering_id == "off-1"));
    assert!(results.iter().any(|o| o.offering_id == "off-2"));
}

#[tokio::test]
async fn test_search_offerings_dsl_basic_type_filter() {
    let db = setup_test_db().await;

    // Insert offerings with different product types
    let pubkey1 = vec![1u8; 32];
    ensure_provider_with_pool(&db, &pubkey1, "US").await;
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, $2, $3, 'VPS Server', 'USD', 50.0, 0, 'public', 'vps', 'monthly', 'in_stock', 'US', 'City', FALSE, 0)",
        101,
        pubkey1,
        "vps-1"
    )
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    let pubkey2 = vec![2u8; 32];
    ensure_provider_with_pool(&db, &pubkey2, "US").await;
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, $2, $3, 'Compute Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 0)",
        102,
        pubkey2,
        "compute-1"
    )
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    // Search for compute type only (1 test + 2 example compute offerings)
    let results = db
        .search_offerings_dsl("type:compute", 10, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(results.len(), 3);
    // Verify all are compute type
    assert!(results.iter().all(|r| r.product_type == "compute"));
}

#[tokio::test]
async fn test_search_offerings_dsl_price_range() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 50.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "US", 150.0).await;
    insert_test_offering(&db, 3, &[3u8; 32], "US", 250.0).await;

    // Search for price range [0 TO 100]
    let results = db
        .search_offerings_dsl("price:[0 TO 100]", 10, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(results.len(), 1);
    assert!(results.iter().all(|r| r.monthly_price <= 100.0));

    // Search for price range [100 TO 200]
    let results = db
        .search_offerings_dsl("price:[100 TO 200]", 10, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(results.len(), 1);
    assert!(results
        .iter()
        .all(|r| r.monthly_price >= 100.0 && r.monthly_price <= 200.0));
}

#[tokio::test]
async fn test_search_offerings_dsl_combined_filters() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;

    // Insert offerings with different attributes
    let pubkey1 = vec![1u8; 32];
    ensure_provider_with_pool(&db, &pubkey1, "US").await;
    sqlx::query(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, $2, $3, 'US Compute', 'USD', 80.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0)",
    )
    .bind(101i64)
    .bind(&pubkey1)
    .bind("us-compute")
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    let pubkey2 = vec![2u8; 32];
    ensure_provider_with_pool(&db, &pubkey2, "DE").await;
    sqlx::query(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, $2, $3, 'DE Compute', 'USD', 120.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'DE', 'Berlin', FALSE, 0)",
    )
    .bind(102i64)
    .bind(&pubkey2)
    .bind("eu-compute")
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    let pubkey3 = vec![3u8; 32];
    ensure_provider_with_pool(&db, &pubkey3, "US").await;
    sqlx::query(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, $2, $3, 'US VPS', 'USD', 50.0, 0, 'public', 'vps', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0)",
    )
    .bind(103i64)
    .bind(&pubkey3)
    .bind("us-vps")
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    // Combined query: type:compute AND country:US
    let results = db
        .search_offerings_dsl("type:compute AND country:US", 10, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].product_type, "compute");
    assert_eq!(results[0].datacenter_country, "US");
    assert_eq!(results[0].offer_name, "US Compute");
}

#[tokio::test]
async fn test_search_offerings_dsl_comparison_operators() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;

    // Insert offerings with different core counts (ensure providers have pools)
    let pubkey1 = vec![1u8; 32];
    ensure_provider_with_pool(&db, &pubkey1, "US").await;
    sqlx::query(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, processor_cores, created_at_ns) VALUES ($1, $2, $3, '4 Core Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 4, 0)",
    )
    .bind(101i64)
    .bind(&pubkey1)
    .bind("server-4core")
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    let pubkey2 = vec![2u8; 32];
    ensure_provider_with_pool(&db, &pubkey2, "US").await;
    sqlx::query(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, processor_cores, created_at_ns) VALUES ($1, $2, $3, '8 Core Server', 'USD', 150.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 8, 0)",
    )
    .bind(102i64)
    .bind(&pubkey2)
    .bind("server-8core")
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    let pubkey3 = vec![3u8; 32];
    ensure_provider_with_pool(&db, &pubkey3, "US").await;
    sqlx::query(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, processor_cores, created_at_ns) VALUES ($1, $2, $3, '16 Core Server', 'USD', 200.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 16, 0)",
    )
    .bind(103i64)
    .bind(&pubkey3)
    .bind("server-16core")
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    // Test >= operator (2 test offerings with cores >= 8)
    let results = db
        .search_offerings_dsl("cores:>=8", 10, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.processor_cores.unwrap_or(0) >= 8));

    // Test < operator (1 test offering with 4 cores)
    let results = db
        .search_offerings_dsl("cores:<8", 10, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(results.len(), 1);
    assert!(results.iter().all(|r| r.processor_cores.unwrap_or(0) < 8));
}

#[tokio::test]
async fn test_search_offerings_dsl_excludes_private() {
    let db = setup_test_db().await;

    // Insert public offering
    insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;

    // Insert private offering (should be excluded)
    let pubkey = vec![2u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, $2, $3, 'Private', 'USD', 50.0, 0, 'private', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 0)",
        200,
        pubkey,
        "private-1"
    )
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    // DSL search should only return public offerings (including examples)
    let results = db
        .search_offerings_dsl("type:compute", 100, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert!(!results.is_empty());
    assert!(results
        .iter()
        .all(|o| o.visibility.to_lowercase() == "public"));
    assert!(results.iter().any(|o| o.offering_id == "off-1"));
    // Verify private offering is NOT in results
    assert!(!results.iter().any(|o| o.offering_id == "private-1"));
}

#[tokio::test]
async fn test_search_offerings_dsl_invalid_query() {
    let db = setup_test_db().await;

    // Invalid field name should return error
    let result = db.search_offerings_dsl("invalid_field:value", 10, 0).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown field"));
}

#[tokio::test]
async fn test_search_offerings_dsl_pagination() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;
    for i in 0..5 {
        insert_test_offering(&db, i, &[i as u8; 32], "US", 100.0 + i as f64).await;
    }

    // First page (sorted by price ASC)
    let page1 = db
        .search_offerings_dsl("", 2, 0)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].monthly_price, 100.0);
    assert_eq!(page1[1].monthly_price, 101.0);

    // Second page
    let page2 = db
        .search_offerings_dsl("", 2, 2)
        .await
        .expect("Failed to search offerings DSL");
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].monthly_price, 102.0);
    assert_eq!(page2[1].monthly_price, 103.0);
}

#[tokio::test]
async fn test_import_seeded_offerings_csv() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    let csv_data = "offering_id,offer_name,description,product_page_url,currency,monthly_price,setup_fee,visibility,product_type,virtualization_type,billing_interval,stock_status,datacenter_country,datacenter_city,unmetered_bandwidth
seeded-1,Seeded Server,From scraper,https://example.com/product,USD,100.0,0.0,public,dedicated,,monthly,in_stock,US,New York,false";

    let (success_count, errors) = db
        .import_seeded_offerings_csv(&pubkey, csv_data, false)
        .await
        .expect("Failed to import seeded offerings CSV");

    assert_eq!(success_count, 1, "errors: {:?}", errors);
    assert_eq!(errors.len(), 0);

    // Verify offering has offering_source='seeded'
    let off = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1"#,
        "seeded-1"
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to fetch from database");
    let offering = db
        .get_offering(off)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert_eq!(offering.offering_source, Some("seeded".to_string()));

    // Verify external_checkout_url was copied from product_page_url
    assert_eq!(
        offering.external_checkout_url,
        Some("https://example.com/product".to_string())
    );
    assert_eq!(
        offering.product_page_url,
        Some("https://example.com/product".to_string())
    );
}

#[tokio::test]
async fn test_import_seeded_offerings_csv_with_external_checkout_url() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // CSV with explicit external_checkout_url should not be overridden
    let csv_data = "offering_id,offer_name,description,product_page_url,external_checkout_url,currency,monthly_price,setup_fee,visibility,product_type,virtualization_type,billing_interval,stock_status,datacenter_country,datacenter_city,unmetered_bandwidth
seeded-2,Seeded Server 2,From scraper,https://example.com/info,https://example.com/checkout,USD,100.0,0.0,public,dedicated,,monthly,in_stock,US,New York,false";

    let (success_count, errors) = db
        .import_seeded_offerings_csv(&pubkey, csv_data, false)
        .await
        .expect("Failed to import seeded offerings CSV");

    assert_eq!(success_count, 1, "errors: {:?}", errors);
    assert_eq!(errors.len(), 0);

    let off = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1"#,
        "seeded-2"
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to fetch from database");
    let offering = db
        .get_offering(off)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert_eq!(offering.offering_source, Some("seeded".to_string()));

    // Verify external_checkout_url kept explicit value
    assert_eq!(
        offering.external_checkout_url,
        Some("https://example.com/checkout".to_string())
    );
    assert_eq!(
        offering.product_page_url,
        Some("https://example.com/info".to_string())
    );
}

#[tokio::test]
async fn test_import_seeded_offerings_csv_upsert() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Initial import
    let csv_data = "offering_id,offer_name,currency,monthly_price,setup_fee,visibility,product_type,billing_interval,stock_status,datacenter_country,datacenter_city,unmetered_bandwidth,product_page_url
seeded-3,Original,USD,100.0,0.0,public,compute,monthly,in_stock,US,NYC,false,https://example.com/old";

    let (success_count, _) = db
        .import_seeded_offerings_csv(&pubkey, csv_data, false)
        .await
        .expect("Failed to import seeded offerings CSV");
    assert_eq!(success_count, 1);

    // Upsert with updated data
    let csv_data_updated = "offering_id,offer_name,currency,monthly_price,setup_fee,visibility,product_type,billing_interval,stock_status,datacenter_country,datacenter_city,unmetered_bandwidth,product_page_url
seeded-3,Updated,USD,200.0,10.0,public,compute,monthly,in_stock,US,LA,false,https://example.com/new";

    let (success_count, errors) = db
        .import_seeded_offerings_csv(&pubkey, csv_data_updated, true)
        .await
        .expect("Failed to import seeded offerings CSV");

    assert_eq!(success_count, 1);
    assert_eq!(errors.len(), 0);

    // Verify update
    let off = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = $1"#,
        "seeded-3"
    )
    .fetch_one(&db.pool)
    .await
    .expect("Failed to fetch from database");
    let offering = db
        .get_offering(off)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert_eq!(offering.offer_name, "Updated");
    assert_eq!(offering.monthly_price, 200.0);
    assert_eq!(offering.offering_source, Some("seeded".to_string()));
    assert_eq!(
        offering.external_checkout_url,
        Some("https://example.com/new".to_string())
    );
}

#[tokio::test]
async fn test_get_provider_offerings_with_resolved_pool() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];
    register_provider(&db, &pubkey).await;

    // Create a pool for europe region
    db.create_agent_pool("pool-eu-1", &pubkey, "DE Proxmox", "europe", "proxmox")
        .await
        .expect("Failed to create agent pool");

    // Create offering with datacenter in DE (maps to europe region)
    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "test-eu-1".to_string(),
        offer_name: "German Server".to_string(),
        description: None,
        product_page_url: None,
        currency: "DER".to_string(),
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "dedicated_server".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
        stock_status: "in_stock".to_string(),
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
        ssd_amount: None,
        total_ssd_capacity: None,
        unmetered_bandwidth: false,
        uplink_speed: None,
        traffic: None,
        datacenter_country: "DE".to_string(),
        datacenter_city: "Frankfurt".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    db.create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");

    // Get provider offerings - should have resolved pool
    let offerings = db
        .get_provider_offerings(&pubkey)
        .await
        .expect("Failed to get provider offerings");
    assert_eq!(offerings.len(), 1);
    assert_eq!(offerings[0].resolved_pool_id, Some("pool-eu-1".to_string()));
    assert_eq!(
        offerings[0].resolved_pool_name,
        Some("DE Proxmox".to_string())
    );
}

#[tokio::test]
async fn test_get_provider_offerings_with_explicit_pool_id() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];
    register_provider(&db, &pubkey).await;

    // Create two pools
    db.create_agent_pool("pool-eu-1", &pubkey, "DE Pool 1", "europe", "proxmox")
        .await
        .expect("Failed to create agent pool");
    db.create_agent_pool("pool-eu-2", &pubkey, "DE Pool 2", "europe", "script")
        .await
        .expect("Failed to create agent pool");

    // Create offering with explicit pool assignment (should use pool-eu-2 even though location matches pool-eu-1)
    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "test-explicit".to_string(),
        offer_name: "Server with Explicit Pool".to_string(),
        description: None,
        product_page_url: None,
        currency: "DER".to_string(),
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "dedicated_server".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        datacenter_country: "DE".to_string(),
        datacenter_city: "Berlin".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: Some("pool-eu-2".to_string()),
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    db.create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");

    // Get provider offerings - should resolve to explicit pool
    let offerings = db
        .get_provider_offerings(&pubkey)
        .await
        .expect("Failed to get provider offerings");
    assert_eq!(offerings.len(), 1);
    assert_eq!(offerings[0].resolved_pool_id, Some("pool-eu-2".to_string()));
    assert_eq!(
        offerings[0].resolved_pool_name,
        Some("DE Pool 2".to_string())
    );
}

#[tokio::test]
async fn test_get_provider_offerings_no_matching_pool() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];
    register_provider(&db, &pubkey).await;

    // Create pool for north america
    db.create_agent_pool("pool-na-1", &pubkey, "NA Pool", "na", "proxmox")
        .await
        .expect("Failed to create agent pool");

    // Create offering in Europe (no matching pool)
    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "test-no-match".to_string(),
        offer_name: "Server without Pool".to_string(),
        description: None,
        product_page_url: None,
        currency: "DER".to_string(),
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "dedicated_server".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        datacenter_country: "DE".to_string(),
        datacenter_city: "Frankfurt".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    db.create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");

    // Get provider offerings - should have no resolved pool
    let offerings = db
        .get_provider_offerings(&pubkey)
        .await
        .expect("Failed to get provider offerings");
    assert_eq!(offerings.len(), 1);
    assert_eq!(offerings[0].resolved_pool_id, None);
    assert_eq!(offerings[0].resolved_pool_name, None);
}

#[tokio::test]
async fn test_search_offerings_filters_by_pool_existence() {
    let db = setup_test_db().await;

    // Provider with pool
    let provider_with_pool = vec![10u8; 32];
    // Provider without pool
    let provider_without_pool = vec![20u8; 32];

    // Register providers
    register_provider(&db, &provider_with_pool).await;
    register_provider(&db, &provider_without_pool).await;

    // Create a pool for the first provider in US (na region)
    let pool = db
        .create_agent_pool("us-pool", &provider_with_pool, "US Pool", "na", "manual")
        .await
        .expect("Failed to create agent pool");

    // Create offerings for both providers
    // Offering 1: Provider with pool, in US (should be included - matches pool location)
    let params1 = Offering {
        id: None,
        pubkey: hex::encode(&provider_with_pool),
        offering_id: "test-with-pool-1".to_string(),
        offer_name: "Offering with Pool".to_string(),
        description: None,
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 100.0,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "compute".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        datacenter_country: "US".to_string(), // US -> na region
        datacenter_city: "NYC".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None, // No explicit pool - should auto-match by location
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };
    db.create_offering(&provider_with_pool, params1)
        .await
        .expect("Failed to create offering");

    // Offering 2: Provider with pool, explicit pool_id (should be included)
    let params2 = Offering {
        id: None,
        pubkey: hex::encode(&provider_with_pool),
        offering_id: "test-with-pool-2".to_string(),
        offer_name: "Offering with Explicit Pool".to_string(),
        description: None,
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 150.0,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "compute".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        datacenter_country: "CA".to_string(), // Canada - doesn't matter, explicit pool
        datacenter_city: "Toronto".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: Some(pool.pool_id.clone()), // Explicit pool
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };
    db.create_offering(&provider_with_pool, params2)
        .await
        .expect("Failed to create offering");

    // Offering 3: Provider without pool, in US (should be EXCLUDED)
    let params3 = Offering {
        id: None,
        pubkey: hex::encode(&provider_without_pool),
        offering_id: "test-no-pool".to_string(),
        offer_name: "Offering without Pool".to_string(),
        description: None,
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 50.0, // Cheapest price
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "compute".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        datacenter_city: "LA".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };
    db.create_offering(&provider_without_pool, params3)
        .await
        .expect("Failed to create offering");

    // Search all offerings
    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings");

    // Should include offerings 1 and 2 (with pools), exclude offering 3 (no pool)
    let offering_ids: Vec<&str> = results.iter().map(|o| o.offering_id.as_str()).collect();
    assert!(
        offering_ids.contains(&"test-with-pool-1"),
        "Should include offering with auto-matched pool"
    );
    assert!(
        offering_ids.contains(&"test-with-pool-2"),
        "Should include offering with explicit pool"
    );
    assert!(
        !offering_ids.contains(&"test-no-pool"),
        "Should exclude offering without matching pool"
    );
}

#[tokio::test]
async fn test_create_subscription_offering() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];
    ensure_provider_with_pool(&db, &pubkey, "US").await;

    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "subscription-monthly".to_string(),
        offer_name: "Monthly Subscription".to_string(),
        description: Some("A monthly subscription offering".to_string()),
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 49.99,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "compute".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: true,
        subscription_interval_days: Some(30),
        stock_status: "in_stock".to_string(),
        processor_brand: None,
        processor_amount: None,
        processor_cores: Some(4),
        processor_speed: None,
        processor_name: None,
        memory_error_correction: None,
        memory_type: None,
        memory_amount: Some("8GB".to_string()),
        hdd_amount: None,
        total_hdd_capacity: None,
        ssd_amount: None,
        total_ssd_capacity: Some("100GB".to_string()),
        unmetered_bandwidth: false,
        uplink_speed: None,
        traffic: None,
        datacenter_country: "US".to_string(),
        datacenter_city: "NYC".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    let db_id = db
        .create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");
    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");

    assert!(offering.is_subscription);
    assert_eq!(offering.subscription_interval_days, Some(30));
    assert_eq!(offering.offer_name, "Monthly Subscription");
}

#[tokio::test]
async fn test_create_yearly_subscription_offering() {
    let db = setup_test_db().await;
    let pubkey = vec![2u8; 32];
    ensure_provider_with_pool(&db, &pubkey, "DE").await;

    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "subscription-yearly".to_string(),
        offer_name: "Yearly Subscription".to_string(),
        description: None,
        product_page_url: None,
        currency: "EUR".to_string(),
        monthly_price: 399.99,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "compute".to_string(),
        virtualization_type: None,
        billing_interval: "yearly".to_string(),
        billing_unit: "year".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: true,
        subscription_interval_days: Some(365),
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
        datacenter_country: "DE".to_string(),
        datacenter_city: "Berlin".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    let db_id = db
        .create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");
    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");

    assert!(offering.is_subscription);
    assert_eq!(offering.subscription_interval_days, Some(365));
}

#[tokio::test]
async fn test_get_subscription_offering_fields() {
    let db = setup_test_db().await;
    let pubkey = vec![3u8; 32];
    ensure_provider_with_pool(&db, &pubkey, "US").await;

    // Create a subscription offering
    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "sub-get-test".to_string(),
        offer_name: "Get Test Sub".to_string(),
        description: None,
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 29.99,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "compute".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: true,
        subscription_interval_days: Some(30),
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
        datacenter_city: "NYC".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    let db_id = db
        .create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");

    // Get the offering by ID and verify subscription fields
    let retrieved = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");
    assert!(retrieved.is_subscription);
    assert_eq!(retrieved.subscription_interval_days, Some(30));

    // Also verify via search
    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            min_price_monthly: None,
            max_price_monthly: None,
            limit: 100,
            offset: 0,
        })
        .await
        .expect("Failed to search offerings");

    let found = results.iter().find(|o| o.offering_id == "sub-get-test");
    assert!(
        found.is_some(),
        "Subscription offering should be in search results"
    );
    let found = found.expect("Expected to find subscription offering in search results");
    assert!(found.is_subscription);
    assert_eq!(found.subscription_interval_days, Some(30));
}

#[tokio::test]
async fn test_one_time_offering_default_subscription_fields() {
    let db = setup_test_db().await;
    let pubkey = vec![4u8; 32];
    ensure_provider_with_pool(&db, &pubkey, "US").await;

    // Create a one-time (non-subscription) offering
    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "one-time-test".to_string(),
        offer_name: "One-Time Offering".to_string(),
        description: None,
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "compute".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false, // One-time, not subscription
        subscription_interval_days: None,
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
        datacenter_city: "NYC".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: None,
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    let db_id = db
        .create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");
    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");

    // One-time offerings should have is_subscription = false and no interval
    assert!(!offering.is_subscription);
    assert_eq!(offering.subscription_interval_days, None);
}

#[tokio::test]
async fn test_template_name_generates_provisioner_config() {
    let db = setup_test_db().await;
    let pubkey = vec![42u8; 32];

    // Create offering with numeric template_name (should auto-generate provisioner_config)
    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "template-test".to_string(),
        offer_name: "Template Test".to_string(),
        description: None,
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 50.0,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "vps".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        datacenter_city: "NYC".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None, // Not set - should be auto-generated
        template_name: Some("9001".to_string()), // Numeric template name
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    let db_id = db
        .create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");

    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");

    // Verify provisioner_config was auto-generated from numeric template_name
    assert_eq!(offering.template_name, Some("9001".to_string()));
    let config = offering
        .provisioner_config
        .expect("provisioner_config should be auto-generated");
    let parsed: serde_json::Value =
        serde_json::from_str(&config).expect("provisioner_config should be valid JSON");
    assert_eq!(parsed["template_vmid"], 9001);
}

#[tokio::test]
async fn test_template_name_non_numeric_no_config() {
    let db = setup_test_db().await;
    let pubkey = vec![43u8; 32];

    // Create offering with non-numeric template_name (should NOT generate provisioner_config)
    let params = Offering {
        id: None,
        pubkey: hex::encode(&pubkey),
        offering_id: "template-non-numeric".to_string(),
        offer_name: "Non-Numeric Template".to_string(),
        description: None,
        product_page_url: None,
        currency: "USD".to_string(),
        monthly_price: 50.0,
        setup_fee: 0.0,
        visibility: "public".to_string(),
        product_type: "vps".to_string(),
        virtualization_type: None,
        billing_interval: "monthly".to_string(),
        billing_unit: "month".to_string(),
        pricing_model: None,
        price_per_unit: None,
        included_units: None,
        overage_price_per_unit: None,
        stripe_metered_price_id: None,
        is_subscription: false,
        subscription_interval_days: None,
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
        datacenter_city: "NYC".to_string(),
        datacenter_latitude: None,
        datacenter_longitude: None,
        control_panel: None,
        gpu_name: None,
        gpu_count: None,
        gpu_memory_gb: None,
        min_contract_hours: None,
        max_contract_hours: None,
        payment_methods: None,
        features: None,
        operating_systems: None,
        trust_score: None,
        has_critical_flags: None,
        is_example: false,
        offering_source: None,
        external_checkout_url: None,
        reseller_name: None,
        reseller_commission_percent: None,
        owner_username: None,
        provisioner_type: None,
        provisioner_config: None,
        template_name: Some("ubuntu-22.04".to_string()), // Non-numeric
        agent_pool_id: None,
        provider_online: None,
        resolved_pool_id: None,
        resolved_pool_name: None,
    };

    let db_id = db
        .create_offering(&pubkey, params)
        .await
        .expect("Failed to create offering");

    let offering = db
        .get_offering(db_id)
        .await
        .expect("Failed to get offering")
        .expect("Expected offering to exist");

    // Non-numeric template_name should be stored but NOT generate provisioner_config
    assert_eq!(offering.template_name, Some("ubuntu-22.04".to_string()));
    assert!(
        offering.provisioner_config.is_none(),
        "Non-numeric template_name should not generate provisioner_config"
    );
}

// ==================== Tier Selection Tests ====================

#[test]
fn test_tier_selection_with_sufficient_resources() {
    use crate::database::agent_pools::PoolCapabilities;
    use crate::database::offerings::select_applicable_tiers;

    // Pool with resources sufficient for small and medium tiers
    let capabilities = PoolCapabilities {
        pool_id: "test-pool".to_string(),
        online_agents: 2,
        total_cpu_cores: 16,
        min_agent_cpu_cores: 8,
        total_memory_mb: 32 * 1024, // 32 GB
        min_agent_memory_mb: 16 * 1024,
        total_storage_gb: 500,
        min_agent_storage_gb: 250,
        cpu_models: vec!["AMD EPYC 7763".to_string()],
        has_gpu: false,
        gpu_models: vec![],
        available_templates: vec!["ubuntu-22.04".to_string()],
    };

    let (applicable, unavailable) = select_applicable_tiers(&capabilities);

    // Should have small, medium, and large (but not xlarge)
    let tier_names: Vec<_> = applicable.iter().map(|t| t.name.as_str()).collect();
    assert!(
        tier_names.contains(&"small"),
        "Small tier should be available"
    );
    assert!(
        tier_names.contains(&"medium"),
        "Medium tier should be available"
    );
    assert!(
        tier_names.contains(&"large"),
        "Large tier should be available"
    );
    assert!(
        !tier_names.contains(&"xlarge"),
        "XLarge tier should NOT be available (need 32 cores)"
    );

    // GPU tier should be unavailable
    let unavailable_names: Vec<_> = unavailable.iter().map(|t| t.tier.as_str()).collect();
    assert!(
        unavailable_names.contains(&"gpu-small"),
        "GPU tier should be unavailable (no GPU)"
    );
}

#[test]
fn test_tier_selection_with_gpu() {
    use crate::database::agent_pools::PoolCapabilities;
    use crate::database::offerings::select_applicable_tiers;

    // Pool with GPU
    let capabilities = PoolCapabilities {
        pool_id: "gpu-pool".to_string(),
        online_agents: 1,
        total_cpu_cores: 32,
        min_agent_cpu_cores: 32,
        total_memory_mb: 128 * 1024, // 128 GB
        min_agent_memory_mb: 128 * 1024,
        total_storage_gb: 2000,
        min_agent_storage_gb: 2000,
        cpu_models: vec!["AMD EPYC 7763".to_string()],
        has_gpu: true,
        gpu_models: vec!["NVIDIA RTX 4090".to_string()],
        available_templates: vec!["ubuntu-22.04".to_string()],
    };

    let (applicable, _unavailable) = select_applicable_tiers(&capabilities);

    // Should have all tiers including GPU
    let tier_names: Vec<_> = applicable.iter().map(|t| t.name.as_str()).collect();
    assert!(
        tier_names.contains(&"small"),
        "Small tier should be available"
    );
    assert!(
        tier_names.contains(&"medium"),
        "Medium tier should be available"
    );
    assert!(
        tier_names.contains(&"large"),
        "Large tier should be available"
    );
    assert!(
        tier_names.contains(&"xlarge"),
        "XLarge tier should be available"
    );
    assert!(
        tier_names.contains(&"gpu-small"),
        "GPU tier should be available"
    );
}

#[test]
fn test_tier_selection_with_insufficient_resources() {
    use crate::database::agent_pools::PoolCapabilities;
    use crate::database::offerings::select_applicable_tiers;

    // Pool with minimal resources
    let capabilities = PoolCapabilities {
        pool_id: "tiny-pool".to_string(),
        online_agents: 1,
        total_cpu_cores: 2,
        min_agent_cpu_cores: 2,
        total_memory_mb: 4 * 1024, // 4 GB
        min_agent_memory_mb: 4 * 1024,
        total_storage_gb: 50,
        min_agent_storage_gb: 50,
        cpu_models: vec!["Intel Xeon".to_string()],
        has_gpu: false,
        gpu_models: vec![],
        available_templates: vec![],
    };

    let (applicable, unavailable) = select_applicable_tiers(&capabilities);

    // No tiers should be available (need at least 4 cores for small)
    assert!(
        applicable.is_empty(),
        "No tiers should be available with insufficient resources"
    );

    // All tiers should be unavailable with reasons
    assert!(
        unavailable.len() >= 4,
        "All compute tiers + GPU tier should be marked unavailable"
    );

    // Check that reasons are provided
    for tier in &unavailable {
        assert!(
            !tier.reason.is_empty(),
            "Unavailable tier {} should have a reason",
            tier.tier
        );
    }
}

#[test]
fn test_generate_suggestions() {
    use crate::database::agent_pools::PoolCapabilities;
    use crate::database::offerings::{default_compute_tiers, generate_suggestions};

    let capabilities = PoolCapabilities {
        pool_id: "eu-pool".to_string(),
        online_agents: 2,
        total_cpu_cores: 16,
        min_agent_cpu_cores: 8,
        total_memory_mb: 32 * 1024,
        min_agent_memory_mb: 16 * 1024,
        total_storage_gb: 500,
        min_agent_storage_gb: 250,
        cpu_models: vec!["AMD EPYC 7763 64-Core Processor".to_string()],
        has_gpu: false,
        gpu_models: vec![],
        available_templates: vec!["ubuntu-22.04".to_string(), "debian-12".to_string()],
    };

    // Generate suggestions for first 2 tiers
    let tiers: Vec<_> = default_compute_tiers().into_iter().take(2).collect();
    let suggestions = generate_suggestions("eu-pool", "EU Pool", "europe", &capabilities, &tiers);

    assert_eq!(suggestions.len(), 2, "Should generate 2 suggestions");

    // Check first suggestion
    let small = &suggestions[0];
    assert_eq!(small.tier_name, "small");
    assert_eq!(small.offering_id, "eu-pool-small");
    assert_eq!(small.offer_name, "Basic VPS (EU Pool)");
    assert_eq!(small.processor_cores, 1);
    assert_eq!(small.memory_amount, "2 GB");
    assert_eq!(small.total_ssd_capacity, "25 GB");
    assert_eq!(small.datacenter_country, "DE"); // Default for Europe
    assert!(small.processor_brand.is_some());
    assert_eq!(small.processor_brand.as_deref(), Some("AMD"));
    assert!(small.needs_pricing);

    // Check medium suggestion
    let medium = &suggestions[1];
    assert_eq!(medium.tier_name, "medium");
    assert_eq!(medium.processor_cores, 2);
    assert_eq!(medium.memory_amount, "4 GB");
}

#[test]
fn test_tier_eligibility_min_agent_check() {
    use crate::database::agent_pools::PoolCapabilities;
    use crate::database::offerings::select_applicable_tiers;

    // Pool has lots of total resources, but smallest agent is tiny
    let capabilities = PoolCapabilities {
        pool_id: "heterogeneous-pool".to_string(),
        online_agents: 10,
        total_cpu_cores: 100,        // Lots total
        min_agent_cpu_cores: 1,      // But smallest agent only has 1 core
        total_memory_mb: 256 * 1024, // Lots total
        min_agent_memory_mb: 1024,   // But smallest has only 1 GB
        total_storage_gb: 5000,      // Lots total
        min_agent_storage_gb: 10,    // But smallest has only 10 GB
        cpu_models: vec!["Intel Xeon".to_string()],
        has_gpu: false,
        gpu_models: vec![],
        available_templates: vec![],
    };

    let (applicable, unavailable) = select_applicable_tiers(&capabilities);

    // No tiers should be available because min agent is too small
    assert!(
        applicable.is_empty(),
        "No tiers should be available when smallest agent cannot host them"
    );

    // Check that unavailability reasons mention agent constraints
    let small_tier = unavailable.iter().find(|t| t.tier == "small");
    assert!(
        small_tier.is_some(),
        "Small tier should be in unavailable list"
    );
    assert!(
        small_tier.unwrap().reason.contains("agent"),
        "Reason should mention agent constraint"
    );
}
