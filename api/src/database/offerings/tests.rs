use super::*;
use crate::database::test_helpers::setup_test_db;

async fn insert_test_offering(db: &Database, id: i64, pubkey: &[u8], country: &str, price: f64) {
    // Use IDs starting from 100 to avoid conflicts with example data from migration 002
    let db_id = id + 100;
    let offering_id = format!("off-{}", id);
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, payment_methods, features, operating_systems, created_at_ns) VALUES (?, ?, ?, 'Test Offer', 'USD', ?, 0, 'public', 'compute', 'monthly', 'in_stock', ?, 'City', 0, NULL, NULL, NULL, 0)",
        db_id,
        pubkey,
        offering_id,
        price,
        country
    )
    .execute(&db.pool)
    .await
    .unwrap();
}

// Helper to get the database ID from test ID (test IDs start from 1, DB IDs from 100)
fn test_id_to_db_id(test_id: i64) -> i64 {
    test_id + 100
}

#[test]
fn export_typescript_types() {
    // This test ensures TypeScript types are exported
    Offering::export().expect("Failed to export Offering type");
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
    assert_eq!(offering.unwrap().id, Some(db_id));
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
    // Count includes example offerings from migration 008
    assert!(count >= 2);
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
            in_stock_only: false,
            limit: 100,
            offset: 0,
        })
        .await
        .unwrap();
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
            "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, payment_methods, features, operating_systems, created_at_ns) VALUES (?, ?, ?, 'Private Offer', 'USD', ?, 0, 'private', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, NULL, NULL, NULL, 0)",
            db_id_private,
            pubkey_ref,
            offering_id_private,
            200.0
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            limit: 100,
            offset: 0,
        })
        .await
        .unwrap();

    // Should return public offerings (including examples), not the private one
    assert!(results.len() >= 1);
    assert!(results.iter().any(|o| o.offering_id == "off-1"));
    assert!(results.iter().all(|o| o.visibility.to_lowercase() == "public"));
    // Verify private offering is NOT in results
    assert!(!results.iter().any(|o| o.offering_id == "off-2"));
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

    // All offerings are fetched
    let results = db
        .search_offerings(SearchOfferingsParams {
            product_type: None,
            country: None,
            in_stock_only: false,
            limit: 10,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 3);
    // Results sorted by monthly_price ASC
    assert_eq!(results[0].monthly_price, 50.0);
    assert_eq!(results[1].monthly_price, 150.0);
    assert_eq!(results[2].monthly_price, 250.0);
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
    {
        let pubkey_ref = &pubkey;
        let offering_id_ref = &offering_id;
        sqlx::query!(
            "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, payment_methods, features, operating_systems, created_at_ns) VALUES (?, ?, ?, 'Test Offer', 'USD', ?, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 'BTC', NULL, NULL, 0)",
            db_id,
            pubkey_ref,
            offering_id_ref,
            100.0
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

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
    let off1 = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = ?"#,
        "off-1"
    )
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
    let off2 = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = ?"#,
        "off-2"
    )
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
        .unwrap();

    assert_eq!(success_count, 1, "errors: {:?}", errors);
    assert_eq!(errors.len(), 0);

    // Verify fields parsed correctly despite different order
    let off = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = ?"#,
        "reorder-1"
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let offering = db.get_offering(off).await.unwrap().unwrap();
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
        .unwrap();

    assert_eq!(success_count, 1, "errors: {:?}", errors);
    assert_eq!(errors.len(), 0);

    let off = sqlx::query_scalar!(
        r#"SELECT id as "id!: i64" FROM provider_offerings WHERE offering_id = ?"#,
        "gpu-1"
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let offering = db.get_offering(off).await.unwrap().unwrap();
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
    insert_test_offering(&db, 2, &[2u8; 32], "EU", 200.0).await;

    // Empty query returns all public offerings (including examples)
    let results = db.search_offerings_dsl("", 100, 0).await.unwrap();
    assert!(results.len() >= 2);
    assert!(results.iter().any(|o| o.offering_id == "off-1"));
    assert!(results.iter().any(|o| o.offering_id == "off-2"));
}

#[tokio::test]
async fn test_search_offerings_dsl_basic_type_filter() {
    let db = setup_test_db().await;

    // Insert offerings with different product types
    let pubkey1 = vec![1u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, ?, ?, 'VPS Server', 'USD', 50.0, 0, 'public', 'vps', 'monthly', 'in_stock', 'US', 'City', 0, 0)",
        101,
        pubkey1,
        "vps-1"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let pubkey2 = vec![2u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, ?, ?, 'Compute Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 0)",
        102,
        pubkey2,
        "compute-1"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Search for compute type only
    let results = db
        .search_offerings_dsl("type:compute", 10, 0)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].product_type, "compute");
    assert_eq!(results[0].offer_name, "Compute Server");
}

#[tokio::test]
async fn test_search_offerings_dsl_price_range() {
    let db = setup_test_db().await;
    insert_test_offering(&db, 1, &[1u8; 32], "US", 50.0).await;
    insert_test_offering(&db, 2, &[2u8; 32], "US", 150.0).await;
    insert_test_offering(&db, 3, &[3u8; 32], "US", 250.0).await;

    // Search for price range [0 TO 100]
    let results = db
        .search_offerings_dsl("price:[0 TO 100]", 10, 0)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].monthly_price, 50.0);

    // Search for price range [100 TO 200]
    let results = db
        .search_offerings_dsl("price:[100 TO 200]", 10, 0)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].monthly_price, 150.0);
}

#[tokio::test]
async fn test_search_offerings_dsl_combined_filters() {
    let db = setup_test_db().await;

    // Insert offerings with different attributes
    let pubkey1 = vec![1u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, ?, ?, 'US Compute', 'USD', 80.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0)",
        101,
        pubkey1,
        "us-compute"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let pubkey2 = vec![2u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, ?, ?, 'EU Compute', 'USD', 120.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'EU', 'Berlin', 0, 0)",
        102,
        pubkey2,
        "eu-compute"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let pubkey3 = vec![3u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, ?, ?, 'US VPS', 'USD', 50.0, 0, 'public', 'vps', 'monthly', 'in_stock', 'US', 'NYC', 0, 0)",
        103,
        pubkey3,
        "us-vps"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Combined query: type:compute AND country:US
    let results = db
        .search_offerings_dsl("type:compute AND country:US", 10, 0)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].product_type, "compute");
    assert_eq!(results[0].datacenter_country, "US");
    assert_eq!(results[0].offer_name, "US Compute");
}

#[tokio::test]
async fn test_search_offerings_dsl_comparison_operators() {
    let db = setup_test_db().await;

    // Insert offerings with different core counts
    let pubkey1 = vec![1u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, processor_cores, created_at_ns) VALUES (?, ?, ?, '4 Core Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 4, 0)",
        101,
        pubkey1,
        "server-4core"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let pubkey2 = vec![2u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, processor_cores, created_at_ns) VALUES (?, ?, ?, '8 Core Server', 'USD', 150.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 8, 0)",
        102,
        pubkey2,
        "server-8core"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let pubkey3 = vec![3u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, processor_cores, created_at_ns) VALUES (?, ?, ?, '16 Core Server', 'USD', 200.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 16, 0)",
        103,
        pubkey3,
        "server-16core"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Test >= operator
    let results = db.search_offerings_dsl("cores:>=8", 10, 0).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.processor_cores.unwrap_or(0) >= 8));

    // Test < operator
    let results = db.search_offerings_dsl("cores:<8", 10, 0).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].processor_cores, Some(4));
}

#[tokio::test]
async fn test_search_offerings_dsl_excludes_private() {
    let db = setup_test_db().await;

    // Insert public offering
    insert_test_offering(&db, 1, &[1u8; 32], "US", 100.0).await;

    // Insert private offering (should be excluded)
    let pubkey = vec![2u8; 32];
    sqlx::query!(
        "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, ?, ?, 'Private', 'USD', 50.0, 0, 'private', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 0)",
        200,
        pubkey,
        "private-1"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // DSL search should only return public offerings (including examples)
    let results = db
        .search_offerings_dsl("type:compute", 100, 0)
        .await
        .unwrap();
    assert!(results.len() >= 1);
    assert!(results.iter().all(|o| o.visibility.to_lowercase() == "public"));
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
    for i in 0..5 {
        insert_test_offering(&db, i, &[i as u8; 32], "US", 100.0 + i as f64).await;
    }

    // First page
    let page1 = db.search_offerings_dsl("", 2, 0).await.unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].monthly_price, 100.0);
    assert_eq!(page1[1].monthly_price, 101.0);

    // Second page
    let page2 = db.search_offerings_dsl("", 2, 2).await.unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].monthly_price, 102.0);
    assert_eq!(page2[1].monthly_price, 103.0);
}
