use candid::Principal;
use dcc_common::LedgerMap;

// Import the functions we need to test
use dcc_common::{
    link_identities, 
    unlink_identities,
    list_linked_identities,
    get_primary_principal,
    LABEL_LINKED_IC_IDS,
};

fn create_test_ledger() -> LedgerMap {
    LedgerMap::default()
}

fn create_test_principal(id: u8) -> Principal {
    let mut bytes = vec![0u8; 29];
    bytes[0] = id;
    Principal::from_slice(&bytes)
}

#[test]
fn test_link_identities() {
    let mut ledger = create_test_ledger();
    let primary = create_test_principal(1);
    let secondary1 = create_test_principal(2);
    let secondary2 = create_test_principal(3);

    // Link first secondary identity
    let result = link_identities(&mut ledger, primary, secondary1, 1000);
    assert!(result.is_ok(), "Failed to link first identity: {:?}", result);

    // Link second secondary identity
    let result = link_identities(&mut ledger, primary, secondary2, 2000);
    assert!(result.is_ok(), "Failed to link second identity: {:?}", result);

    // List linked identities
    let result = list_linked_identities(primary);
    assert!(result.is_ok(), "Failed to list linked identities: {:?}", result);

    let linked_identities = result.unwrap();
    assert_eq!(linked_identities.len(), 2, "Expected 2 linked identities");
    assert!(linked_identities.contains(&secondary1), "First secondary identity not found");
    assert!(linked_identities.contains(&secondary2), "Second secondary identity not found");
}

#[test]
fn test_unlink_identities() {
    let mut ledger = create_test_ledger();
    let primary = create_test_principal(1);
    let secondary1 = create_test_principal(2);
    let secondary2 = create_test_principal(3);

    // Link both secondary identities
    let result = link_identities(&mut ledger, primary, secondary1, 1000);
    assert!(result.is_ok());
    let result = link_identities(&mut ledger, primary, secondary2, 2000);
    assert!(result.is_ok());

    // Unlink first secondary identity
    let result = unlink_identities(&mut ledger, primary, secondary1, 3000);
    assert!(result.is_ok(), "Failed to unlink identity: {:?}", result);

    // Check that only second identity remains
    let result = list_linked_identities( primary);
    assert!(result.is_ok());
    let linked_identities = result.unwrap();
    assert_eq!(linked_identities.len(), 1, "Expected 1 linked identity");
    assert!(linked_identities.contains(&secondary2), "Second secondary identity not found");
    assert!(!linked_identities.contains(&secondary1), "First secondary identity should be removed");
}

#[test]
fn test_get_primary_identity() {
    let mut ledger = create_test_ledger();
    let primary = create_test_principal(1);
    let secondary = create_test_principal(2);

    // Link secondary identity
    let result = link_identities(&mut ledger, primary, secondary, 1000);
    assert!(result.is_ok());

    // Get primary for primary principal (should return itself)
    let result = get_primary_principal( primary);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), primary);

    // Get primary for secondary principal
    let result = get_primary_principal(secondary);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), primary);

    // Get primary for non-existent principal
    let non_existent = create_test_principal(99);
    let result = get_primary_principal( non_existent);
    assert!(result.is_err());
}

#[test]
fn test_link_validation() {
    let mut ledger = create_test_ledger();
    let primary = create_test_principal(1);
    let secondary = create_test_principal(2);

    // Cannot link identity to itself
    let result = link_identities(&mut ledger, primary, primary, 1000);
    assert!(result.is_err());

    // Cannot link same secondary identity twice
    let result = link_identities(&mut ledger, primary, secondary, 1000);
    assert!(result.is_ok());
    let result = link_identities(&mut ledger, primary, secondary, 2000);
    assert!(result.is_err());

    // Cannot link to non-existent primary
    let result = unlink_identities(&mut ledger, create_test_principal(99), secondary, 3000);
    assert!(result.is_err());
}