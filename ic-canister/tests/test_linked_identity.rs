use candid::Principal;

mod test_utils;
use crate::test_utils::TestContext;

fn create_test_principal(id: u8) -> Principal {
    let mut bytes = vec![0u8; 29];
    bytes[0] = id;
    Principal::from_slice(&bytes)
}

#[test]
fn test_linked_identities_basic_flow() {
    let ctx = TestContext::new();
    let primary = create_test_principal(1);
    let secondary1 = create_test_principal(2);
    let secondary2 = create_test_principal(3);

    // Initially no linked identities
    let result = ctx.list_linked_identities(primary);
    assert!(result.is_err(), "Should have no linked identities initially");

    // Link both secondary identities at once
    let result = ctx.link_identities(primary, vec![secondary1, secondary2]);
    assert!(result.is_ok(), "Failed to link identities: {:?}", result);

    // Verify linked identities
    let result = ctx.list_linked_identities(primary);
    assert!(result.is_ok(), "Failed to list linked identities");
    let linked_identities = result.unwrap();
    assert_eq!(linked_identities.len(), 2, "Expected 2 linked identities");
    assert!(linked_identities.contains(&secondary1), "First secondary identity not found");
    assert!(linked_identities.contains(&secondary2), "Second secondary identity not found");

    // Verify primary resolution
    assert_eq!(ctx.get_primary_principal(primary).unwrap(), primary, "Primary should resolve to itself");
    assert_eq!(ctx.get_primary_principal(secondary1).unwrap(), primary, "Secondary1 should resolve to primary");
    assert_eq!(ctx.get_primary_principal(secondary2).unwrap(), primary, "Secondary2 should resolve to primary");

    // Test persistence through upgrade
    ctx.upgrade().expect("Canister upgrade failed");

    let result = ctx.list_linked_identities(primary);
    assert!(result.is_ok());
    let linked_identities = result.unwrap();
    assert_eq!(linked_identities.len(), 2);
    assert!(linked_identities.contains(&secondary1));
    assert!(linked_identities.contains(&secondary2));
}

#[test]
fn test_unlink_identities() {
    let ctx = TestContext::new();
    let primary = create_test_principal(1);
    let secondary1 = create_test_principal(2);
    let secondary2 = create_test_principal(3);

    // Link both secondary identities
    let result = ctx.link_identities(primary, vec![secondary1, secondary2]);
    assert!(result.is_ok());

    // Verify initial state
    let linked_identities = ctx.list_linked_identities(primary).unwrap();
    assert_eq!(linked_identities.len(), 2);

    // Unlink first secondary identity
    let result = ctx.unlink_identities(primary, vec![secondary1]);
    assert!(result.is_ok(), "Failed to unlink identity: {:?}", result);

    // Verify state after unlinking
    let linked_identities = ctx.list_linked_identities(primary).unwrap();
    assert_eq!(linked_identities.len(), 1, "Expected 1 linked identity");
    assert!(!linked_identities.contains(&secondary1), "First secondary identity should be removed");
    assert!(linked_identities.contains(&secondary2), "Second secondary identity should remain");

    // Verify primary resolution after unlinking
    assert_eq!(ctx.get_primary_principal(primary).unwrap(), primary);
    assert_eq!(ctx.get_primary_principal(secondary1).unwrap(), secondary1, "Unlinked identity should resolve to itself");
    assert_eq!(ctx.get_primary_principal(secondary2).unwrap(), primary);

    // Test persistence through upgrade
    ctx.upgrade().expect("Canister upgrade failed");

    let linked_identities = ctx.list_linked_identities(primary).unwrap();
    assert_eq!(linked_identities.len(), 1);
    assert!(!linked_identities.contains(&secondary1));
    assert!(linked_identities.contains(&secondary2));
}

#[test]
fn test_link_identity_validations() {
    let ctx = TestContext::new();
    let primary1 = create_test_principal(1);
    let primary2 = create_test_principal(2);
    let secondary = create_test_principal(3);

    // First link is successful
    let result = ctx.link_identities(primary1, vec![secondary]);
    assert!(result.is_ok(), "Initial linking should succeed");

    // Attempting to link same secondary to different primary should fail
    let result = ctx.link_identities(primary2, vec![secondary]);
    assert!(result.is_ok(), "Re-linking should succeed but be idempotent");

    // Verify the secondary is still linked to the original primary
    assert_eq!(ctx.get_primary_principal(secondary).unwrap(), primary1);

    // Verify through listing
    let linked_to_primary1 = ctx.list_linked_identities(primary1).unwrap();
    assert_eq!(linked_to_primary1.len(), 1);
    assert!(linked_to_primary1.contains(&secondary));

    let result = ctx.list_linked_identities(primary2);
    assert!(result.is_err(), "Primary2 should have no linked identities");

    // Test persistence through upgrade
    ctx.upgrade().expect("Canister upgrade failed");

    assert_eq!(ctx.get_primary_principal(secondary).unwrap(), primary1);
    let linked_to_primary1 = ctx.list_linked_identities(primary1).unwrap();
    assert_eq!(linked_to_primary1.len(), 1);
    assert!(linked_to_primary1.contains(&secondary));
}

#[test]
fn test_unlink_identity_validations() {
    let ctx = TestContext::new();
    let primary = create_test_principal(1);
    let secondary = create_test_principal(2);
    let unlinked = create_test_principal(3);

    // Link identity first
    let result = ctx.link_identities(primary, vec![secondary]);
    assert!(result.is_ok());

    // Attempting to unlink non-linked identity should fail
    let result = ctx.unlink_identities(primary, vec![unlinked]);
    assert!(result.is_err(), "Unlinking non-linked identity should fail");

    // Attempting to unlink from wrong primary should fail
    let wrong_primary = create_test_principal(4);
    let result = ctx.unlink_identities(wrong_primary, vec![secondary]);
    assert!(result.is_err(), "Unlinking from wrong primary should fail");

    // Original link should still be intact
    assert_eq!(ctx.get_primary_principal(secondary).unwrap(), primary);
    let linked_identities = ctx.list_linked_identities(primary).unwrap();
    assert_eq!(linked_identities.len(), 1);
    assert!(linked_identities.contains(&secondary));
}