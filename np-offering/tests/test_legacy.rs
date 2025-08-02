use np_offering::{ProviderOfferings, ProductType};

const SAMPLE_CSV: &str = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Intel Dual Core Dedicated Server,Here goes a product description.,DC2993,https://test.com/DC2993/,EUR,99.99,99.99,Visible,VPS,KVM,Monthly,In stock,Intel,1,2,2.6 GHz,Intel® Xeon® Processor E5-1620 v4,non-ECC,DDR4,8192 MB,0,0,2,160 GB,Unmetered inbound,1000 mbit,10240,NL,"Rotterdam, Netherlands","51.9229,4.46317","KVM over IP, Managed support, Native IPv6, Instant setup","Debian, CentOs, VMWare",cPanel,,"Bitcoin, Credit card, PayPal, Wire Transfer""#;

#[test]
fn test_legacy_parse_csv() {
    let collection = ProviderOfferings::new_from_str(&[0u8; 32], SAMPLE_CSV).expect("Failed to parse CSV");
    assert_eq!(collection.server_offerings.len(), 1);

    let offering = &collection.server_offerings[0];
    assert_eq!(offering.offer_name, "Intel Dual Core Dedicated Server");
    assert_eq!(offering.unique_internal_identifier, "DC2993");
    assert_eq!(offering.monthly_price, 99.99);
    assert_eq!(offering.datacenter_country, "NL");
    assert_eq!(offering.datacenter_city, "Rotterdam, Netherlands");
    assert!(offering.datacenter_coordinates.is_some());
    assert!(!offering.features.is_empty());
    assert!(!offering.operating_systems.is_empty());
    assert!(!offering.payment_methods.is_empty());
}

#[test]
fn test_legacy_filtering() {
    let collection = ProviderOfferings::new_from_str(&[0u8; 32], SAMPLE_CSV).expect("Failed to parse CSV");

    let vps_offerings = collection.find_by_product_type(&ProductType::VPS);
    assert_eq!(vps_offerings.len(), 1);

    let nl_offerings = collection.find_by_country("NL");
    assert_eq!(nl_offerings.len(), 1);

    let price_range = collection.find_by_price_range(50.0, 150.0);
    assert_eq!(price_range.len(), 1);
}

#[test]
fn test_legacy_get_instance_ids() {
    let collection = ProviderOfferings::new_from_str(&[0u8; 32], SAMPLE_CSV).expect("Failed to parse CSV");
    let ids = collection.get_all_instance_ids();
    
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], "DC2993");
}

#[test]
fn test_legacy_matches_search() {
    let collection = ProviderOfferings::new_from_str(&[0u8; 32], SAMPLE_CSV).expect("Failed to parse CSV");
    
    let matches = collection.matches_search("Intel");
    assert!(!matches.is_empty());
    
    let matches = collection.matches_search("DC2993");
    assert!(!matches.is_empty());
    
    let matches = collection.matches_search("NonExistent");
    assert!(matches.is_empty());
}

#[test]
fn test_legacy_serialization() {
    let offerings = ProviderOfferings::new_from_str(&[0u8; 32], SAMPLE_CSV).expect("Failed to parse CSV");

    let json = offerings.serialize_as_json().expect("Failed to serialize");
    assert!(!json.is_empty());

    let deserialized = ProviderOfferings::deserialize_from_json(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.provider_pubkey, offerings.provider_pubkey);
    assert_eq!(deserialized.server_offerings.len(), offerings.server_offerings.len());
    assert_eq!(
        deserialized.server_offerings[0].offer_name,
        offerings.server_offerings[0].offer_name
    );
}

#[test]
fn test_legacy_to_csv_string() {
    let collection = ProviderOfferings::new_from_str(&[0u8; 32], SAMPLE_CSV).expect("Failed to parse CSV");
    
    // Test individual offering serialization instead of the collection
    let offering = &collection.server_offerings[0];
    let csv_bytes = offering.serialize().expect("Failed to serialize offering to CSV");
    let csv_string = String::from_utf8(csv_bytes).expect("Invalid UTF-8 in CSV");
    
    assert!(!csv_string.is_empty());
    assert!(csv_string.contains("Offer Name")); // Header should be present
    assert!(csv_string.contains("Intel Dual Core Dedicated Server"));
}

#[test]
fn test_legacy_new_constructor() {
    let provider_pubkey = vec![1u8; 32];
    let server_offerings = vec![];
    
    let collection = ProviderOfferings::new(provider_pubkey.clone(), server_offerings);
    
    assert_eq!(collection.provider_pubkey, provider_pubkey);
    assert_eq!(collection.server_offerings.len(), 0);
}