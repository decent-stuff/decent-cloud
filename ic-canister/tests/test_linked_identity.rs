use decent_cloud_common::{
    add_secondary_identity, authorize_operation, create_linked_record, get_timestamp_ns,
    list_linked_identities, remove_secondary_identity, set_primary_identity, DccIdentity,
};
use ed25519_dalek::SigningKey;
use ledger_map::LedgerMap;
use rand::rngs::OsRng;

fn create_test_identity() -> DccIdentity {
    let signing_key = SigningKey::generate(&mut OsRng);
    DccIdentity::new_signing(&signing_key).unwrap()
}

#[test]
fn test_linked_identity_workflow() {
    // Create a ledger and test identities
    let mut ledger = LedgerMap::new();
    let primary = create_test_identity();
    let secondary1 = create_test_identity();
    let secondary2 = create_test_identity();
    let unauthorized = create_test_identity();
    let timestamp = get_timestamp_ns();

    // 1. Create a linked identity record
    let result = create_linked_record(&mut ledger, &primary, timestamp);
    assert!(result.is_ok());

    // 2. Add secondary identities
    let result = add_secondary_identity(&mut ledger, &primary, &secondary1, timestamp);
    assert!(result.is_ok());

    let result = add_secondary_identity(&mut ledger, &primary, &secondary2, timestamp);
    assert!(result.is_ok());

    // 3. List linked identities
    let result = list_linked_identities(&ledger, &primary);
    assert!(result.is_ok());
    let identities = result.unwrap();
    assert_eq!(identities.len(), 3); // Primary + 2 secondaries

    // 4. Check authorization
    // Primary should be authorized
    let result = authorize_operation(&ledger, &primary, &primary);
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Secondary identities should be authorized
    let result = authorize_operation(&ledger, &primary, &secondary1);
    assert!(result.is_ok());
    assert!(result.unwrap());

    let result = authorize_operation(&ledger, &primary, &secondary2);
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Unauthorized identity should not be authorized
    let result = authorize_operation(&ledger, &primary, &unauthorized);
    assert!(result.is_ok());
    assert!(!result.unwrap());

    // 5. Remove a secondary identity
    let result = remove_secondary_identity(&mut ledger, &primary, &secondary1, timestamp);
    assert!(result.is_ok());

    // Verify it was removed
    let result = authorize_operation(&ledger, &primary, &secondary1);
    assert!(result.is_ok());
    assert!(!result.unwrap());

    // 6. Set a new primary identity
    let result = set_primary_identity(&mut ledger, &primary, &secondary2, timestamp);
    assert!(result.is_ok());

    // Verify the new primary is authorized
    let result = authorize_operation(&ledger, &secondary2, &secondary2);
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify the old primary is now a secondary and is authorized
    let result = authorize_operation(&ledger, &secondary2, &primary);
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify the record doesn't exist under the old primary
    let result = list_linked_identities(&ledger, &primary);
    assert!(result.is_err());
}

#[test]
fn test_linked_identity_edge_cases() {
    let mut ledger = LedgerMap::new();
    let primary = create_test_identity();
    let secondary = create_test_identity();
    let timestamp = get_timestamp_ns();

    // Try to add a secondary identity without creating a record first (should create the record)
    let result = add_secondary_identity(&mut ledger, &primary, &secondary, timestamp);
    assert!(result.is_ok());

    // Try to add the primary as a secondary (should fail)
    let result = add_secondary_identity(&mut ledger, &primary, &primary, timestamp);
    assert!(result.is_err());

    // Try to add the same secondary again (should fail)
    let result = add_secondary_identity(&mut ledger, &primary, &secondary, timestamp);
    assert!(result.is_err());

    // Try to remove a non-existent secondary identity (should fail)
    let non_existent = create_test_identity();
    let result = remove_secondary_identity(&mut ledger, &primary, &non_existent, timestamp);
    assert!(result.is_err());

    // Try to set a non-existent secondary as primary (should fail)
    let result = set_primary_identity(&mut ledger, &primary, &non_existent, timestamp);
    assert!(result.is_err());
}
