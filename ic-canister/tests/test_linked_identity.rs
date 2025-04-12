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
    let pmain = create_test_principal(1);
    let palt1 = create_test_principal(2);
    let palt2 = create_test_principal(3);

    // Initially no linked identities
    let alt_principals = ctx.list_alt_principals(pmain).unwrap();
    assert_eq!(
        alt_principals.len(),
        0,
        "Should have no linked identities initially"
    );

    // Link both alt identities at once
    let result = ctx.link_principals(pmain, vec![palt1, palt2]);
    assert!(result.is_ok(), "Failed to link identities: {:?}", result);

    // Verify linked identities
    let result = ctx.list_alt_principals(pmain);
    assert!(result.is_ok(), "Failed to list linked identities");
    let linked_identities = result.unwrap();
    assert_eq!(linked_identities.len(), 2, "Expected 2 linked identities");
    assert!(
        linked_identities.contains(&palt1),
        "First alt identity not found"
    );
    assert!(
        linked_identities.contains(&palt2),
        "Second alt identity not found"
    );

    ctx.commit();

    // Verify linked identities
    let result = ctx.list_alt_principals(pmain);
    assert!(result.is_ok(), "Failed to list linked identities");
    let linked_identities = result.unwrap();
    assert_eq!(linked_identities.len(), 2, "Expected 2 linked identities");
    assert!(
        linked_identities.contains(&palt1),
        "First alt identity not found"
    );
    assert!(
        linked_identities.contains(&palt2),
        "Second alt identity not found"
    );

    // Verify main principal resolution
    assert_eq!(
        ctx.get_main_principal(pmain).unwrap(),
        pmain,
        "Main principal should resolve to itself"
    );
    assert_eq!(
        ctx.get_main_principal(palt1).unwrap(),
        pmain,
        "Alt1 principal should resolve to main"
    );
    assert_eq!(
        ctx.get_main_principal(palt2).unwrap(),
        pmain,
        "Alt2 principal should resolve to main"
    );

    // Test persistence through upgrade
    ctx.upgrade().expect("Canister upgrade failed");

    let result = ctx.list_alt_principals(pmain);
    assert!(result.is_ok());
    let linked_identities = result.unwrap();
    assert_eq!(linked_identities.len(), 2);
    assert!(linked_identities.contains(&palt1));
    assert!(linked_identities.contains(&palt2));
}

#[test]
fn test_unlink_principals() {
    let ctx = TestContext::new();
    let pmain = create_test_principal(1);
    let palt1 = create_test_principal(2);
    let palt2 = create_test_principal(3);

    // Link both alternate identities
    let result = ctx.link_principals(pmain, vec![palt1, palt2]);
    assert!(result.is_ok());

    // Verify initial state
    let linked_identities = ctx.list_alt_principals(pmain).unwrap();
    assert_eq!(linked_identities.len(), 2);

    // Unlink first alternate identity
    let result = ctx.unlink_principals(pmain, vec![palt1]);
    assert!(result.is_ok(), "Failed to unlink identity: {:?}", result);

    // Verify state after unlinking
    let linked_identities = ctx.list_alt_principals(pmain).unwrap();
    assert_eq!(linked_identities.len(), 1, "Expected 1 linked identity");
    assert!(
        !linked_identities.contains(&palt1),
        "First alternate identity should be removed"
    );
    assert!(
        linked_identities.contains(&palt2),
        "Second alternate identity should remain"
    );

    // Verify main resolution after unlinking
    assert_eq!(ctx.get_main_principal(pmain).unwrap(), pmain);
    assert_eq!(
        ctx.get_main_principal(palt1).unwrap(),
        palt1,
        "Unlinked identity should resolve to itself"
    );
    assert_eq!(ctx.get_main_principal(palt2).unwrap(), pmain);

    // Test persistence through upgrade
    ctx.upgrade().expect("Canister upgrade failed");

    let linked_identities = ctx.list_alt_principals(pmain).unwrap();
    assert_eq!(linked_identities.len(), 1);
    assert!(!linked_identities.contains(&palt1));
    assert!(linked_identities.contains(&palt2));
    assert_eq!(ctx.get_main_principal(pmain).unwrap(), pmain);
    assert_eq!(
        ctx.get_main_principal(palt1).unwrap(),
        palt1,
        "Unlinked identity should resolve to itself"
    );
    assert_eq!(ctx.get_main_principal(palt2).unwrap(), pmain);
}

#[test]
fn test_link_identity_validations() {
    let ctx = TestContext::new();
    let pmain1 = create_test_principal(1);
    let pmain2 = create_test_principal(2);
    let palt = create_test_principal(3);

    // First link is successful
    let result = ctx.link_principals(pmain1, vec![palt]);
    assert!(result.is_ok(), "Initial linking should succeed");

    // Linking the same alt principal to different main principal should fail since it's already linked
    let result = ctx.link_principals(pmain2, vec![palt]);
    assert!(result.is_err(), "Re-linking should fail since palt is already linked");

    // Verify the alt principal is still linked to the original main principal
    assert_eq!(ctx.get_main_principal(palt).unwrap(), pmain1);

    // Verify through listing
    let alts_main1 = ctx.list_alt_principals(pmain1).unwrap();
    assert_eq!(alts_main1.len(), 1);

    let alts_main2 = ctx.list_alt_principals(pmain2).unwrap();
    assert_eq!(alts_main2.len(), 0);

    // Unlink pmain1 -> palt
    let result = ctx.unlink_principals(pmain1, vec![palt]);
    assert!(result.is_ok(), "Failed to unlink the alt principal from pmain1");

    // After unlinking, palt should resolve to itself
    assert_eq!(ctx.get_main_principal(palt).unwrap(), palt);
    let alts_main1 = ctx.list_alt_principals(pmain1).unwrap();
    assert_eq!(alts_main1.len(), 0);

    // Now linking palt to pmain2 should succeed
    let result = ctx.link_principals(pmain2, vec![palt]);
    assert!(result.is_ok(), "Linking after unlinking should succeed");

    // Verify the alt principal is now linked to pmain2
    assert_eq!(ctx.get_main_principal(palt).unwrap(), pmain2);
    let alts_main2 = ctx.list_alt_principals(pmain2).unwrap();
    assert_eq!(alts_main2.len(), 1);
    assert!(alts_main2.contains(&palt));

    // Test persistence through upgrade
    ctx.upgrade().expect("Canister upgrade failed");

    assert_eq!(ctx.get_main_principal(palt).unwrap(), pmain2);
    let alts_main2 = ctx.list_alt_principals(pmain2).unwrap();
    assert_eq!(alts_main2.len(), 1);
    assert!(alts_main2.contains(&palt));
}

#[test]
fn test_unlink_identity_validations() {
    let ctx = TestContext::new();
    let pmain = create_test_principal(1);
    let palt = create_test_principal(2);
    let unlinked = create_test_principal(3);

    // Link identity first
    let result = ctx.link_principals(pmain, vec![palt]);
    assert!(result.is_ok());

    // Attempting to unlink non-linked identity should succeed but not change the state
    let result = ctx.unlink_principals(pmain, vec![unlinked]);
    assert!(result.is_ok(), "Unlinking non-linked identity should succeed, without changes");

    // Original link should still be intact
    assert_eq!(ctx.get_main_principal(palt).unwrap(), pmain);
    let linked_identities = ctx.list_alt_principals(pmain).unwrap();
    assert_eq!(linked_identities.len(), 1);
    assert!(linked_identities.contains(&palt));
}
