use provider_offering::{
    Currency, OfferingFilter, OfferingRegistry, ProductType, SearchQuery, StockStatus,
    VirtualizationType,
};
mod common;
use common::*;

#[test]
fn test_input_sanitization() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Test with SQL injection inputs
    let injection_input = "Intel; DROP TABLE offerings; --";
    let results = registry.search_text(injection_input);
    // Should still find results but with risky characters removed
    assert_eq!(
        results.len(),
        2,
        "Expected 2 results for SQL injection input, got {}",
        results.len()
    ); // Should find "Intel" offerings

    // Test with special characters that should be filtered
    let special_chars = "Intel<script>alert('xss')</script>";
    let results = registry.search_text(special_chars);
    // The script tags are removed, but "Intel" and other keywords should still be found
    assert_eq!(
        results.len(),
        2,
        "Expected 2 results for special chars, got {}",
        results.len()
    ); // Should find "Intel" offerings

    // Test with allowed punctuation
    let allowed_punctuation = "rotterdam, netherlands";
    let results = registry.search_text(allowed_punctuation);
    assert_eq!(
        results.len(),
        1,
        "Expected 1 result for netherlands, got {}",
        results.len()
    ); // Should find the Netherlands offering
}

#[test]
fn test_query_optimization_order() {
    // Test that filters are applied in the optimal order
    // This is a bit tricky to test directly since it's an internal optimization,
    // but we can test the behavior with multiple filters

    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Create a query with multiple filters that should be optimized
    let query = SearchQuery::new()
        .with_filter(OfferingFilter::HasGPU(true)) // High selectivity (should be first)
        .with_filter(OfferingFilter::PriceRange(0.0, 200.0)) // Low selectivity (should be last)
        .with_filter(OfferingFilter::Country("US".to_string())); // Medium selectivity

    let results = registry.search(&query);
    assert_eq!(results.len(), 1);
    assert!(results[0].server_offering.gpu_name.is_some());
    assert_eq!(results[0].server_offering.datacenter_country, "US");
}

#[test]
fn test_error_handling_invalid_csv() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);

    // Test with malformed CSV
    let invalid_csv = "invalid,csv,data";
    let result = registry.add_provider_offerings(provider.clone(), invalid_csv);
    // Should handle gracefully without panicking
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0); // No valid offerings parsed

    // Test with empty CSV
    let empty_csv = "";
    let result = registry.add_provider_offerings(provider.clone(), empty_csv);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_edge_cases_empty_registry() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);

    // Test search on empty registry
    let query = SearchQuery::new().with_text("Intel");
    let results = registry.search(&query);
    assert_eq!(results.len(), 0);

    // Test text search on empty registry
    let results = registry.search_text("Intel");
    assert_eq!(results.len(), 0);

    // Test get_offering on empty registry
    let offering = registry.get_offering(&provider, "nonexistent");
    assert!(offering.is_none());

    // Test get_provider_offerings on empty registry
    let offerings = registry.get_provider_offerings(&provider);
    assert_eq!(offerings.len(), 0);

    // Test remove_provider on empty registry
    let removed = registry.remove_provider(&provider);
    assert_eq!(removed, 0);
}

#[test]
fn test_edge_cases_pagination() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Test with offset larger than result set
    let query = SearchQuery::new().with_offset(100);
    let results = registry.search(&query);
    assert_eq!(results.len(), 0);

    // Test with limit larger than result set
    let query = SearchQuery::new().with_limit(100);
    let results = registry.search(&query);
    assert_eq!(results.len(), 3); // All offerings

    // Test with zero limit
    let query = SearchQuery::new().with_limit(0);
    let results = registry.search(&query);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_edge_cases_special_filters() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Test filter with no GPU
    let query = SearchQuery::new().with_filter(OfferingFilter::HasGPU(false));
    let results = registry.search(&query);
    assert_eq!(results.len(), 2); // Two offerings without GPU

    // Test filter with empty string
    let query = SearchQuery::new().with_filter(OfferingFilter::City("".to_string()));
    let results = registry.search(&query);
    // Should return all offerings since empty string is contained in any city name
    assert_eq!(results.len(), 3);

    // Test filter with non-existent country
    let query = SearchQuery::new().with_filter(OfferingFilter::Country("NonExistent".to_string()));
    let results = registry.search(&query);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_comprehensive_filter_combinations() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Test complex filter combination
    let query = SearchQuery::new()
        .with_filter(OfferingFilter::ProductType(ProductType::VPS))
        .with_filter(OfferingFilter::Currency(Currency::USD))
        .with_filter(OfferingFilter::StockStatus(StockStatus::InStock))
        .with_filter(OfferingFilter::MinMemoryGB(4))
        .with_filter(OfferingFilter::MinCores(2));

    let results = registry.search(&query);
    assert_eq!(results.len(), 1);
    assert!(
        std::mem::discriminant(&results[0].server_offering.currency)
            == std::mem::discriminant(&Currency::USD)
    );
    assert!(
        std::mem::discriminant(&results[0].server_offering.product_type)
            == std::mem::discriminant(&ProductType::VPS)
    );

    // Test with virtualization type filter
    let query =
        SearchQuery::new().with_filter(OfferingFilter::VirtualizationType(VirtualizationType::KVM));

    let results = registry.search(&query);
    assert_eq!(results.len(), 2); // Two offerings with KVM
}

#[test]
fn test_case_insensitive_search() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Test case insensitive text search
    let test_cases = vec![
        ("intel", 2),
        ("INTEL", 2),
        ("Intel", 2),
        ("netherlands", 1), // Found in datacenter_city: "Rotterdam, Netherlands"
        ("NETHERLANDS", 1), // Found in datacenter_city: "Rotterdam, Netherlands"
        ("rotterdam", 0), // Not found as a separate keyword, only as part of "Rotterdam, Netherlands"
    ];

    for (search_term, expected_count) in test_cases {
        let results = registry.search_text(search_term);
        println!(
            "Search for '{}': {} results (expected {})",
            search_term,
            results.len(),
            expected_count
        );
        assert_eq!(
            results.len(),
            expected_count,
            "Failed for search term: {}",
            search_term
        );
    }
}

#[test]
fn test_stop_word_filtering() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Test that stop words are filtered out
    let stop_words = vec![
        "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
    ];

    for stop_word in stop_words {
        let results = registry.search_text(stop_word);
        assert_eq!(
            results.len(),
            0,
            "Stop word '{}' should return no results",
            stop_word
        );
    }

    // Test that short words (< 3 chars) are filtered out
    let short_words = vec!["a", "an", "is", "it", "of"];

    for short_word in short_words {
        let results = registry.search_text(short_word);
        assert_eq!(
            results.len(),
            0,
            "Short word '{}' should return no results",
            short_word
        );
    }
}

#[test]
fn test_memory_amount_parsing() {
    let mut registry = OfferingRegistry::new();
    let provider = test_provider_pubkey(1);
    registry
        .add_provider_offerings(provider.clone(), SAMPLE_CSV)
        .unwrap();

    // Test MinMemoryGB filter with different memory formats
    let query = SearchQuery::new().with_filter(OfferingFilter::MinMemoryGB(8));
    let results = registry.search(&query);
    assert_eq!(results.len(), 2); // Two offerings with >= 8GB

    let query = SearchQuery::new().with_filter(OfferingFilter::MinMemoryGB(16));
    let results = registry.search(&query);
    assert_eq!(results.len(), 1); // One offering with >= 16GB

    let query = SearchQuery::new().with_filter(OfferingFilter::MinMemoryGB(32));
    let results = registry.search(&query);
    assert_eq!(results.len(), 0); // No offerings with >= 32GB
}
