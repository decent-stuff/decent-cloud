use np_offering::serialization::{OfferingResponseBuilder, PaginatedOfferingsResponse};
use np_offering::ProviderOfferings;
mod common;
use common::*;

/// Create a test offering for serialization tests
fn create_test_offering() -> ProviderOfferings {
    let provider_pubkey = default_test_provider_pubkey();
    ProviderOfferings::new_from_str(&provider_pubkey.to_vec(), SINGLE_OFFERING_CSV)
        .expect("Failed to create test offering")
}

#[test]
fn test_optimized_json_serialization() {
    let offering = create_test_offering();

    let json = offering
        .serialize_as_json()
        .expect("Failed to serialize optimized JSON");
    assert!(!json.is_empty());

    // The optimized JSON should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse optimized JSON as valid JSON");

    // Should have the expected compact structure (PEM + CSV format)
    assert!(parsed.is_object());
    assert!(parsed.get("provider_pubkey_pem").is_some());
    assert!(parsed.get("server_offerings_csv").is_some());

    // Should contain the expected data
    assert!(parsed["provider_pubkey_pem"]
        .as_str()
        .unwrap()
        .starts_with("-----BEGIN PUBLIC KEY-----"));
    assert!(parsed["server_offerings_csv"]
        .as_str()
        .unwrap()
        .contains("Offer Name"));
    assert!(parsed["server_offerings_csv"]
        .as_str()
        .unwrap()
        .contains(&offering.server_offerings[0].offer_name));
}

#[test]
fn test_compact_json_serialization() {
    let offering = create_test_offering();

    let compact = offering
        .serialize_as_json()
        .expect("Failed to serialize compact JSON");

    // Test that compact JSON is valid and contains expected data
    let parsed: serde_json::Value =
        serde_json::from_str(&compact).expect("Failed to parse compact JSON as valid JSON");

    assert!(parsed.is_object());
    assert!(parsed.get("provider_pubkey_pem").is_some());
    assert!(parsed.get("server_offerings_csv").is_some());
    assert!(parsed["provider_pubkey_pem"]
        .as_str()
        .unwrap()
        .starts_with("-----BEGIN PUBLIC KEY-----"));
    assert!(parsed["server_offerings_csv"]
        .as_str()
        .unwrap()
        .contains(&offering.server_offerings[0].offer_name));

    // Test with a larger offering to see size difference
    let mut larger_offering = create_test_offering();
    let additional_offering = create_test_offering();
    larger_offering
        .server_offerings
        .extend(additional_offering.server_offerings);

    let larger_compact = larger_offering
        .serialize_as_json()
        .expect("Failed to serialize larger compact JSON");

    // Larger offering should produce larger JSON
    assert!(
        larger_compact.len() > compact.len(),
        "Larger offering should produce larger JSON: {} vs {}",
        larger_compact.len(),
        compact.len()
    );
}

#[test]
fn test_response_builder() {
    let offering = create_test_offering();
    let mut builder = OfferingResponseBuilder::new(10000); // 10KB limit

    let added = builder
        .try_add_offering(&offering)
        .expect("Failed to add offering to response builder");
    assert!(added, "Expected offering to be added to response builder");

    let response = builder.build();
    assert_eq!(response.len(), 1, "Expected response to contain 1 offering");
    assert_eq!(
        response[0].0, offering.provider_pubkey,
        "Provider pubkey in response should match original"
    );

    // Test with a very small limit that should reject the offering
    let mut tiny_builder = OfferingResponseBuilder::new(1); // Extremely small limit
    let tiny_added = tiny_builder
        .try_add_offering(&offering)
        .expect("Failed to check offering against tiny limit");

    // The offering should not fit in such a small limit, but the builder should still accept it
    // since it's the first offering (builder always accepts at least one offering)
    assert!(
        tiny_added,
        "Expected first offering to be accepted even by tiny limit"
    );

    let tiny_response = tiny_builder.build();
    assert_eq!(
        tiny_response.len(),
        1,
        "Expected tiny response to have 1 offering"
    );

    // Test adding multiple offerings
    let mut multi_builder = OfferingResponseBuilder::new(5000); // 5KB limit
    let offering1 = create_test_offering();
    let mut offering2 = create_test_offering();
    offering2.provider_pubkey = test_provider_pubkey(2).to_vec();

    let added1 = multi_builder
        .try_add_offering(&offering1)
        .expect("Failed to add first offering to multi builder");
    assert!(
        added1,
        "Expected first offering to be added to multi builder"
    );

    let added2 = multi_builder
        .try_add_offering(&offering2)
        .expect("Failed to add second offering to multi builder");
    // Second offering may or may not fit depending on size
    println!("Second offering added: {}", added2);

    let multi_response = multi_builder.build();
    assert!(
        !multi_response.is_empty(),
        "Expected multi response to have at least 1 offering"
    );
}

#[test]
fn test_paginated_response() {
    let offering1 = create_test_offering();
    let mut offering2 = create_test_offering();

    // Make the second offering different
    offering2.provider_pubkey = test_provider_pubkey(2).to_vec();
    offering2.server_offerings[0].offer_name = "Modified Test Offering".to_string();
    offering2.server_offerings[0].unique_internal_identifier = "MOD001".to_string();

    let offerings = vec![offering1, offering2];
    let response = PaginatedOfferingsResponse::new(offerings.clone(), 10, 0, 2, 100000)
        .expect("Failed to create paginated response");

    assert_eq!(
        response.offerings.len(),
        2,
        "Expected 2 offerings in response"
    );
    assert_eq!(response.total_count, 10, "Expected total_count to be 10");
    assert!(response.has_more, "Expected has_more to be true");
    assert_eq!(response.page, 0, "Expected page to be 0");
    assert_eq!(response.page_size, 2, "Expected page_size to be 2");

    // Test that the offerings are serialized as JSON strings
    assert!(response.offerings[0].contains("provider_pubkey_pem"));
    assert!(response.offerings[0].contains("server_offerings_csv"));
    assert!(response.offerings[1].contains("Modified Test Offering"));

    // Test that we can deserialize the offerings back
    let deserialized_offerings = response
        .to_provider_offerings()
        .expect("Failed to deserialize offerings from paginated response");
    assert_eq!(
        deserialized_offerings.len(),
        2,
        "Expected 2 deserialized offerings"
    );
    assert_eq!(
        deserialized_offerings[0].server_offerings[0].offer_name,
        offerings[0].server_offerings[0].offer_name
    );
    assert_eq!(
        deserialized_offerings[1].server_offerings[0].offer_name,
        offerings[1].server_offerings[0].offer_name
    );

    // Test with a page that should have no more results
    let last_page_response =
        PaginatedOfferingsResponse::new(vec![create_test_offering()], 1, 1, 2, 100000)
            .expect("Failed to create last page response");

    assert!(
        !last_page_response.has_more,
        "Expected has_more to be false for last page"
    );

    // Test with a byte limit that restricts the response
    let limited_response = PaginatedOfferingsResponse::new(
        offerings.clone(),
        10,
        0,
        2,
        1000, // Small byte limit
    )
    .expect("Failed to create limited response");

    // Should still have at least one offering due to minimum size logic
    assert!(
        !limited_response.offerings.is_empty(),
        "Expected at least 1 offering even with small limit"
    );
}
