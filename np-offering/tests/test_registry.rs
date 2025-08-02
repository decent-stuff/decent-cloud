use np_offering::{OfferingRegistry, ProviderPubkey, SearchQuery, OfferingFilter, ProductType};

const SAMPLE_CSV: &str = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
Intel Dual Core Dedicated Server,Here goes a product description.,DC2993,https://test.com/DC2993/,EUR,99.99,99.99,Visible,VPS,KVM,Monthly,In stock,Intel,1,2,2.6 GHz,Intel® Xeon® Processor E5-1620 v4,non-ECC,DDR4,8192 MB,0,0,2,160 GB,Unmetered inbound,1000 mbit,10240,NL,"Rotterdam, Netherlands","51.9229,4.46317","KVM over IP, Managed support, Native IPv6, Instant setup","Debian, CentOs, VMWare",cPanel,,"Bitcoin, Credit card, PayPal, Wire Transfer"
Intel Quad Core VPS,Another product description.,QC1494,https://test.com/QC1494/,USD,149.99,0.0,Visible,VPS,KVM,Monthly,In stock,Intel,1,4,2200 MHz,Intel® Xeon® Processor E3-1505L v6,ECC,DDR4,16 GB,0,0,1,240 GB,Unmetered inbound,1000 mbit,5120,US,"New York, NY","40.7128,-74.0060","KVM over IP, SSD Storage, IPv6","Ubuntu, CentOS, Debian",cPanel,NVIDIA GTX 1080,"Credit card, PayPal""#;

#[test]
fn test_registry_basic_operations() {
    let mut registry = OfferingRegistry::new();
    let provider = ProviderPubkey::new([1u8; 32]);
    
    // Test adding offerings
    let count = registry.add_provider_offerings(provider.clone(), SAMPLE_CSV).unwrap();
    assert_eq!(count, 2);
    assert_eq!(registry.count(), 2);
    assert_eq!(registry.provider_count(), 1);
    
    // Test direct lookup
    let offering = registry.get_offering(&provider, "DC2993");
    assert!(offering.is_some());
    assert_eq!(offering.unwrap().server_offering.offer_name, "Intel Dual Core Dedicated Server");
    
    // Test provider offerings
    let provider_offerings = registry.get_provider_offerings(&provider);
    assert_eq!(provider_offerings.len(), 2);
}

#[test]
fn test_registry_search() {
    let mut registry = OfferingRegistry::new();
    let provider = ProviderPubkey::new([1u8; 32]);
    registry.add_provider_offerings(provider.clone(), SAMPLE_CSV).unwrap();
    
    // Test text search
    let results = registry.search_text("Intel");
    assert_eq!(results.len(), 2);
    
    let results = registry.search_text("NVIDIA");
    assert_eq!(results.len(), 1);
    
    let results = registry.search_text("nvidia");
    assert_eq!(results.len(), 1);
    
    // Test structured search
    let query = SearchQuery::new()
        .with_filter(OfferingFilter::ProductType(ProductType::VPS));
    let results = registry.search(&query);
    assert_eq!(results.len(), 2);
    
    let query = SearchQuery::new()
        .with_filter(OfferingFilter::HasGPU(true));
    let results = registry.search(&query);
    assert_eq!(results.len(), 1);
    
    let query = SearchQuery::new()
        .with_filter(OfferingFilter::PriceRange(100.0, 150.0));
    let results = registry.search(&query);
    assert_eq!(results.len(), 1);
}

#[test]
fn test_registry_compound_search() {
    let mut registry = OfferingRegistry::new();
    let provider = ProviderPubkey::new([1u8; 32]);
    registry.add_provider_offerings(provider.clone(), SAMPLE_CSV).unwrap();
    
    // Test compound query
    let query = SearchQuery::new()
        .with_provider(provider.clone())
        .with_text("Intel")
        .with_filter(OfferingFilter::HasGPU(true))
        .with_limit(1);
    
    let results = registry.search(&query);
    assert_eq!(results.len(), 1);
    assert!(results[0].server_offering.gpu_name.is_some());
}

#[test]
fn test_registry_update_and_remove() {
    let mut registry = OfferingRegistry::new();
    let provider = ProviderPubkey::new([1u8; 32]);
    
    // Add initial offerings
    registry.add_provider_offerings(provider.clone(), SAMPLE_CSV).unwrap();
    assert_eq!(registry.count(), 2);
    
    // Update offerings (should replace)
    let new_csv = r#"Offer Name,Description,Unique Internal identifier,Product page URL,Currency,Monthly price,Setup fee,Visibility,Product Type,Virtualization type,Billing interval,Stock,Processor Brand,Processor Amount,Processor Cores,Processor Speed,Processor Name,Memory Error Correction,Memory Type,Memory Amount,Hard Disk Drive Amount,Total Hard Disk Drive Capacity,Solid State Disk Amount,Total Solid State Disk Capacity,Unmetered,Uplink speed,Traffic,Datacenter Country,Datacenter City,Datacenter Coordinates,Features,Operating Systems,Control Panel,GPU Name,Payment Methods
New Offering,New description.,NEW001,https://test.com/NEW001/,EUR,75.00,0.0,Visible,Cloud,None,Monthly,In stock,AMD,1,1,3.0 GHz,AMD Ryzen,non-ECC,DDR4,4 GB,0,0,1,100 GB,Standard,100 mbit,1024,DE,"Berlin, Germany","52.5200,13.4050","Basic support","Ubuntu, Debian",,,"PayPal""#;
    
    let count = registry.update_provider_offerings(provider.clone(), new_csv).unwrap();
    assert_eq!(count, 1);
    assert_eq!(registry.count(), 1);
    
    // Verify old offerings are gone
    assert!(registry.get_offering(&provider, "DC2993").is_none());
    assert!(registry.get_offering(&provider, "NEW001").is_some());
    
    // Remove provider
    let removed = registry.remove_provider(&provider);
    assert_eq!(removed, 1);
    assert_eq!(registry.count(), 0);
    assert_eq!(registry.provider_count(), 0);
}

#[test]
fn test_multiple_providers() {
    let mut registry = OfferingRegistry::new();
    let provider1 = ProviderPubkey::new([1u8; 32]);
    let provider2 = ProviderPubkey::new([2u8; 32]);
    
    registry.add_provider_offerings(provider1.clone(), SAMPLE_CSV).unwrap();
    registry.add_provider_offerings(provider2.clone(), SAMPLE_CSV).unwrap();
    
    assert_eq!(registry.count(), 4);
    assert_eq!(registry.provider_count(), 2);
    
    // Test provider-specific queries
    let query = SearchQuery::new().with_provider(provider1.clone());
    let results = registry.search(&query);
    assert_eq!(results.len(), 2);
    
    // Remove one provider
    registry.remove_provider(&provider1);
    assert_eq!(registry.count(), 2);
    assert_eq!(registry.provider_count(), 1);
    
    // Verify provider2 offerings are still there
    let provider2_offerings = registry.get_provider_offerings(&provider2);
    assert_eq!(provider2_offerings.len(), 2);
}