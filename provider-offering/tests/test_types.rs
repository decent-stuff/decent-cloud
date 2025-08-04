use provider_offering::{OfferingError, OfferingFilter, ProductType, ProviderPubkey, SearchQuery};
mod common;
use common::*;

#[test]
fn test_provider_pubkey_creation() {
    let bytes = [1u8; 32];
    let pubkey = ProviderPubkey::new(bytes);

    assert_eq!(pubkey.as_bytes(), &bytes);
    assert_eq!(pubkey.to_vec(), bytes.to_vec());
    assert_eq!(pubkey.to_hex().len(), 64); // 32 bytes * 2 hex chars
}

#[test]
fn test_provider_pubkey_from_slice() {
    let bytes = [1u8; 32];
    let pubkey = ProviderPubkey::from_slice(&bytes).unwrap();
    assert_eq!(pubkey.as_bytes(), &bytes);

    // Test invalid length
    let short_bytes = [1u8; 16];
    let result = ProviderPubkey::from_slice(&short_bytes);
    assert!(result.is_err());

    if let Err(OfferingError::InvalidPubkeyLength(len)) = result {
        assert_eq!(len, 16);
    } else {
        panic!("Expected InvalidPubkeyLength error");
    }
}

#[test]
fn test_search_query_builder() {
    let provider = default_test_provider_pubkey();

    let query = SearchQuery::new()
        .with_provider(provider.clone())
        .with_key("test-key")
        .with_text("search text")
        .with_filter(OfferingFilter::ProductType(ProductType::VPS))
        .with_filter(OfferingFilter::PriceRange(10.0, 100.0))
        .with_limit(10)
        .with_offset(5);

    assert_eq!(query.provider_pubkey, Some(provider));
    assert_eq!(query.offering_key, Some("test-key".to_string()));
    assert_eq!(query.text_filter, Some("search text".to_string()));
    assert_eq!(query.filters.len(), 2);
    assert_eq!(query.limit, Some(10));
    assert_eq!(query.offset, Some(5));
}

#[test]
fn test_search_query_default() {
    let query = SearchQuery::new();

    assert!(query.provider_pubkey.is_none());
    assert!(query.offering_key.is_none());
    assert!(query.text_filter.is_none());
    assert!(query.filters.is_empty());
    assert!(query.limit.is_none());
    assert!(query.offset.is_none());
}

#[test]
fn test_offering_filters() {
    let filters = vec![
        OfferingFilter::PriceRange(10.0, 100.0),
        OfferingFilter::ProductType(ProductType::VPS),
        OfferingFilter::Country("US".to_string()),
        OfferingFilter::City("New York".to_string()),
        OfferingFilter::HasGPU(true),
        OfferingFilter::MinMemoryGB(8),
        OfferingFilter::MinCores(4),
    ];

    // Just verify they can be created and stored
    assert_eq!(filters.len(), 7);
}

#[test]
fn test_provider_pubkey_equality() {
    let pubkey1 = test_provider_pubkey(1);
    let pubkey2 = test_provider_pubkey(1);
    let pubkey3 = test_provider_pubkey(2);

    assert_eq!(pubkey1, pubkey2);
    assert_ne!(pubkey1, pubkey3);
}

#[test]
fn test_provider_pubkey_hex_format() {
    let bytes = [
        0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67,
        0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45,
        0x67, 0x89,
    ];
    let pubkey = ProviderPubkey::new(bytes);
    let hex = pubkey.to_hex();

    assert_eq!(hex.len(), 64);
    assert!(hex.starts_with("abcdef"));
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}
