use np_offering::{OfferingRegistry, SearchQuery, OfferingFilter, ProductType};
mod test_utils;
use test_utils::*;

#[test]
fn test_registry_basic_operations() {
    let mut registry = OfferingRegistry::new();
    let provider = default_test_provider_pubkey();
    
    // Test adding offerings
    let count = registry.add_provider_offerings(provider.clone(), SAMPLE_CSV).unwrap();
    assert_eq!(count, 3);
    assert_eq!(registry.count(), 3);
    assert_eq!(registry.provider_count(), 1);
    
    // Test direct lookup
    let offering = registry.get_offering(&provider, "DC2993");
    assert!(offering.is_some());
    assert_eq!(offering.unwrap().server_offering.offer_name, "Intel Dual Core Dedicated Server");
    
    // Test provider offerings
    let provider_offerings = registry.get_provider_offerings(&provider);
    assert_eq!(provider_offerings.len(), 3);
}

#[test]
fn test_registry_search() {
    let mut registry = OfferingRegistry::new();
    let provider = default_test_provider_pubkey();
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
    assert_eq!(results.len(), 3);
    
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
    let provider = default_test_provider_pubkey();
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
    let provider = default_test_provider_pubkey();
    
    // Add initial offerings
    registry.add_provider_offerings(provider.clone(), SAMPLE_CSV).unwrap();
    assert_eq!(registry.count(), 3);
    
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
    let provider1 = test_provider_pubkey(1);
    let provider2 = test_provider_pubkey(2);
    
    registry.add_provider_offerings(provider1.clone(), SAMPLE_CSV).unwrap();
    registry.add_provider_offerings(provider2.clone(), SAMPLE_CSV).unwrap();
    
    assert_eq!(registry.count(), 6);
    assert_eq!(registry.provider_count(), 2);
    
    // Test provider-specific queries
    let query = SearchQuery::new().with_provider(provider1.clone());
    let results = registry.search(&query);
    assert_eq!(results.len(), 3);
    
    // Remove one provider
    registry.remove_provider(&provider1);
    assert_eq!(registry.count(), 3);
    assert_eq!(registry.provider_count(), 1);
    
    // Verify provider2 offerings are still there
    let provider2_offerings = registry.get_provider_offerings(&provider2);
    assert_eq!(provider2_offerings.len(), 3);
}