use np_offering::{
    BillingInterval, Currency, ErrorCorrection, ProductType, ServerOffering, StockStatus,
    VirtualizationType, Visibility,
};

#[test]
fn test_server_offering_creation() {
    let offering = ServerOffering {
        offer_name: "Test Server".to_string(),
        description: "A test server offering".to_string(),
        unique_internal_identifier: "TEST001".to_string(),
        product_page_url: "https://example.com/test".to_string(),
        currency: Currency::USD,
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: Visibility::Visible,
        product_type: ProductType::VPS,
        virtualization_type: Some(VirtualizationType::KVM),
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(2),
        processor_speed: Some("2.4 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: Some(ErrorCorrection::ECC),
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("8 GB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("100 GB".to_string()),
        unmetered: vec!["inbound".to_string()],
        uplink_speed: Some("1 Gbit".to_string()),
        traffic: Some(1024),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["KVM over IP".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    assert_eq!(offering.offer_name, "Test Server");
    assert!(std::mem::discriminant(&offering.currency) == std::mem::discriminant(&Currency::USD));
    assert_eq!(offering.monthly_price, 99.99);
    assert!(
        std::mem::discriminant(&offering.product_type) == std::mem::discriminant(&ProductType::VPS)
    );
    assert!(
        std::mem::discriminant(&offering.virtualization_type)
            == std::mem::discriminant(&Some(VirtualizationType::KVM))
    );
    assert!(
        std::mem::discriminant(&offering.stock) == std::mem::discriminant(&StockStatus::InStock)
    );
}

#[test]
fn test_get_all_instance_ids() {
    let offering = ServerOffering {
        offer_name: "Test Server".to_string(),
        description: "A test server offering".to_string(),
        unique_internal_identifier: "TEST001".to_string(),
        product_page_url: "https://example.com/test".to_string(),
        currency: Currency::USD,
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: Visibility::Visible,
        product_type: ProductType::VPS,
        virtualization_type: Some(VirtualizationType::KVM),
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(2),
        processor_speed: Some("2.4 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: Some(ErrorCorrection::ECC),
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("8 GB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("100 GB".to_string()),
        unmetered: vec!["inbound".to_string()],
        uplink_speed: Some("1 Gbit".to_string()),
        traffic: Some(1024),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["KVM over IP".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    let ids = offering.get_all_instance_ids();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], "TEST001");
}

#[test]
fn test_matches_search() {
    let offering = ServerOffering {
        offer_name: "Intel Dual Core Server".to_string(),
        description: "A powerful Intel-based server".to_string(),
        unique_internal_identifier: "INTEL001".to_string(),
        product_page_url: "https://example.com/intel".to_string(),
        currency: Currency::USD,
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: Visibility::Visible,
        product_type: ProductType::VPS,
        virtualization_type: Some(VirtualizationType::KVM),
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(2),
        processor_speed: Some("2.4 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: Some(ErrorCorrection::ECC),
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("8 GB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("100 GB".to_string()),
        unmetered: vec!["inbound".to_string()],
        uplink_speed: Some("1 Gbit".to_string()),
        traffic: Some(1024),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["KVM over IP".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    // Test search in offer_name
    let matches = offering.matches_search("Intel");
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.contains("offer_name")));

    // Test search in description
    let matches = offering.matches_search("powerful");
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.contains("description")));

    // Test search in unique_internal_identifier
    let matches = offering.matches_search("INTEL001");
    assert!(!matches.is_empty());
    assert!(matches
        .iter()
        .any(|m| m.contains("unique_internal_identifier")));

    // Test search in processor_brand
    let matches = offering.matches_search("Intel");
    assert!(matches.iter().any(|m| m.contains("processor_brand")));

    // Test search in features
    let matches = offering.matches_search("KVM");
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.contains("features")));

    // Test search in operating_systems
    let matches = offering.matches_search("Ubuntu");
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.contains("operating_systems")));

    // Test search in payment_methods
    let matches = offering.matches_search("PayPal");
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.contains("payment_methods")));

    // Test case insensitive search
    let matches = offering.matches_search("intel");
    assert!(!matches.is_empty());

    // Test search with no matches
    let matches = offering.matches_search("NonExistent");
    assert!(matches.is_empty());
}

#[test]
fn test_instance_pricing() {
    let offering = ServerOffering {
        offer_name: "Test Server".to_string(),
        description: "A test server offering".to_string(),
        unique_internal_identifier: "TEST001".to_string(),
        product_page_url: "https://example.com/test".to_string(),
        currency: Currency::USD,
        monthly_price: 30.0,
        setup_fee: 0.0,
        visibility: Visibility::Visible,
        product_type: ProductType::VPS,
        virtualization_type: Some(VirtualizationType::KVM),
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(2),
        processor_speed: Some("2.4 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: Some(ErrorCorrection::ECC),
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("8 GB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("100 GB".to_string()),
        unmetered: vec!["inbound".to_string()],
        uplink_speed: Some("1 Gbit".to_string()),
        traffic: Some(1024),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["KVM over IP".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    let pricing = offering.instance_pricing("TEST001");

    // Check that on_demand pricing exists
    assert!(pricing.contains_key("on_demand"));
    let on_demand = &pricing["on_demand"];

    // Check that all time units are present
    assert!(on_demand.contains_key("month"));
    assert!(on_demand.contains_key("year"));
    assert!(on_demand.contains_key("day"));
    assert!(on_demand.contains_key("hour"));

    // Check pricing calculations
    assert_eq!(on_demand["month"], "30"); // Monthly price
    assert_eq!(on_demand["year"], "360"); // Monthly * 12
    assert_eq!(on_demand["day"], "1"); // Monthly / 30
    assert!(on_demand["hour"].parse::<f64>().unwrap() > 0.0); // Monthly / (30 * 24)
}

#[test]
fn test_serialize() {
    let offering = ServerOffering {
        offer_name: "Test Server".to_string(),
        description: "A test server offering".to_string(),
        unique_internal_identifier: "TEST001".to_string(),
        product_page_url: "https://example.com/test".to_string(),
        currency: Currency::USD,
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: Visibility::Visible,
        product_type: ProductType::VPS,
        virtualization_type: Some(VirtualizationType::KVM),
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(2),
        processor_speed: Some("2.4 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: Some(ErrorCorrection::ECC),
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("8 GB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("100 GB".to_string()),
        unmetered: vec!["inbound".to_string()],
        uplink_speed: Some("1 Gbit".to_string()),
        traffic: Some(1024),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["KVM over IP".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    let csv_bytes = offering.serialize().unwrap();
    let csv_string = String::from_utf8(csv_bytes).unwrap();

    // Check that CSV contains expected headers
    assert!(csv_string.contains("Offer Name"));
    assert!(csv_string.contains("Description"));
    assert!(csv_string.contains("Currency"));
    assert!(csv_string.contains("Monthly price"));

    // Check that CSV contains expected data
    assert!(csv_string.contains("Test Server"));
    assert!(csv_string.contains("A test server offering"));
    assert!(csv_string.contains("USD"));
    assert!(csv_string.contains("99.99"));
    assert!(csv_string.contains("KVM over IP, IPv6")); // Features joined by comma
    assert!(csv_string.contains("Ubuntu, CentOS")); // OS joined by comma
}

#[test]
fn test_serialize_with_optional_fields() {
    let offering = ServerOffering {
        offer_name: "Minimal Server".to_string(),
        description: "A minimal server offering".to_string(),
        unique_internal_identifier: "MIN001".to_string(),
        product_page_url: "https://example.com/min".to_string(),
        currency: Currency::EUR,
        monthly_price: 29.99,
        setup_fee: 10.0,
        visibility: Visibility::Visible,
        product_type: ProductType::Dedicated,
        virtualization_type: None, // None virtualization
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: None, // None processor brand
        processor_amount: None,
        processor_cores: None,
        processor_speed: None,
        processor_name: None,
        memory_error_correction: None,
        memory_type: None,
        memory_amount: None,
        hdd_amount: 1,
        total_hdd_capacity: Some("500 GB".to_string()),
        ssd_amount: 0,
        total_ssd_capacity: None,
        unmetered: vec![], // Empty unmetered
        uplink_speed: None,
        traffic: None,
        datacenter_country: "DE".to_string(),
        datacenter_city: "Berlin".to_string(),
        datacenter_coordinates: None, // None coordinates
        features: vec![],             // Empty features
        operating_systems: vec![],    // Empty OS
        control_panel: None,
        gpu_name: None,
        payment_methods: vec![], // Empty payment methods
    };

    let csv_bytes = offering.serialize().unwrap();
    let csv_string = String::from_utf8(csv_bytes).unwrap();

    // Check that CSV contains expected data
    assert!(csv_string.contains("Minimal Server"));
    assert!(csv_string.contains("EUR"));
    assert!(csv_string.contains("29.99"));
    assert!(csv_string.contains("10"));

    // Check that empty/None fields are handled properly
    assert!(csv_string.contains(",")); // Empty fields should result in consecutive commas
}

#[test]
fn test_display() {
    let offering = ServerOffering {
        offer_name: "Test Server".to_string(),
        description: "A test server offering".to_string(),
        unique_internal_identifier: "TEST001".to_string(),
        product_page_url: "https://example.com/test".to_string(),
        currency: Currency::USD,
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: Visibility::Visible,
        product_type: ProductType::VPS,
        virtualization_type: Some(VirtualizationType::KVM),
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(2),
        processor_speed: Some("2.4 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: Some(ErrorCorrection::ECC),
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("8 GB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("100 GB".to_string()),
        unmetered: vec!["inbound".to_string()],
        uplink_speed: Some("1 Gbit".to_string()),
        traffic: Some(1024),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["KVM over IP".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    let display_str = format!("{}", offering);

    // Should be formatted as JSON
    assert!(display_str.contains("\"offer_name\": \"Test Server\""));
    assert!(display_str.contains("\"currency\": \"USD\""));
    assert!(display_str.contains("\"monthly_price\": 99.99"));
    assert!(display_str.contains("\"product_type\": \"VPS\""));
}

#[test]
fn test_clone() {
    let offering = ServerOffering {
        offer_name: "Test Server".to_string(),
        description: "A test server offering".to_string(),
        unique_internal_identifier: "TEST001".to_string(),
        product_page_url: "https://example.com/test".to_string(),
        currency: Currency::USD,
        monthly_price: 99.99,
        setup_fee: 0.0,
        visibility: Visibility::Visible,
        product_type: ProductType::VPS,
        virtualization_type: Some(VirtualizationType::KVM),
        billing_interval: BillingInterval::Monthly,
        stock: StockStatus::InStock,
        processor_brand: Some("Intel".to_string()),
        processor_amount: Some(1),
        processor_cores: Some(2),
        processor_speed: Some("2.4 GHz".to_string()),
        processor_name: Some("Intel Xeon".to_string()),
        memory_error_correction: Some(ErrorCorrection::ECC),
        memory_type: Some("DDR4".to_string()),
        memory_amount: Some("8 GB".to_string()),
        hdd_amount: 0,
        total_hdd_capacity: None,
        ssd_amount: 1,
        total_ssd_capacity: Some("100 GB".to_string()),
        unmetered: vec!["inbound".to_string()],
        uplink_speed: Some("1 Gbit".to_string()),
        traffic: Some(1024),
        datacenter_country: "US".to_string(),
        datacenter_city: "New York".to_string(),
        datacenter_coordinates: Some((40.7128, -74.0060)),
        features: vec!["KVM over IP".to_string(), "IPv6".to_string()],
        operating_systems: vec!["Ubuntu".to_string(), "CentOS".to_string()],
        control_panel: Some("cPanel".to_string()),
        gpu_name: None,
        payment_methods: vec!["Credit Card".to_string(), "PayPal".to_string()],
    };

    let cloned_offering = offering.clone();

    assert_eq!(cloned_offering.offer_name, offering.offer_name);
    assert!(
        std::mem::discriminant(&cloned_offering.currency)
            == std::mem::discriminant(&offering.currency)
    );
    assert_eq!(cloned_offering.monthly_price, offering.monthly_price);
    assert!(
        std::mem::discriminant(&cloned_offering.product_type)
            == std::mem::discriminant(&offering.product_type)
    );
    assert!(
        std::mem::discriminant(&cloned_offering.virtualization_type)
            == std::mem::discriminant(&offering.virtualization_type)
    );
    assert!(
        std::mem::discriminant(&cloned_offering.stock) == std::mem::discriminant(&offering.stock)
    );
}
