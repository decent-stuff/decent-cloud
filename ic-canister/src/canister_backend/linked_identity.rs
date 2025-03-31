use super::generic::LEDGER_MAP;
use dcc_common::{
    add_secondary_identity as common_add_secondary_identity,
    authorize_operation as common_authorize_operation,
    create_linked_record as common_create_linked_record, fn_info, get_timestamp_ns,
    list_linked_identities as common_list_linked_identities,
    remove_secondary_identity as common_remove_secondary_identity,
    set_primary_identity as common_set_primary_identity, DccIdentity,
};
use std::cell::RefCell;

/// Creates a new linked identity record for the given primary identity
pub fn _create_linked_record(
    pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    // Verify the signature
    let primary_dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes)?;
    primary_dcc_id.verify_bytes(&pubkey_bytes, &crypto_signature)?;

    fn_info!("{}", primary_dcc_id);

    // Create the linked identity record
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        common_create_linked_record(&mut ledger, &primary_dcc_id, get_timestamp_ns())
    })?;

    Ok(format!(
        "Linked identity record created for {}",
        primary_dcc_id
    ))
}

/// Adds a secondary identity to the linked identity record
pub fn _add_secondary_identity(
    primary_pubkey_bytes: Vec<u8>,
    secondary_pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    // Verify the signature
    let primary_dcc_id = DccIdentity::new_verifying_from_bytes(&primary_pubkey_bytes)?;
    primary_dcc_id.verify_bytes(&secondary_pubkey_bytes, &crypto_signature)?;

    fn_info!("{}", primary_dcc_id);

    // Create the secondary identity
    let secondary_dcc_id = DccIdentity::new_verifying_from_bytes(&secondary_pubkey_bytes)?;

    // Add the secondary identity
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        common_add_secondary_identity(
            &mut ledger,
            &primary_dcc_id,
            &secondary_dcc_id,
            get_timestamp_ns(),
        )
    })?;

    Ok(format!(
        "Secondary identity {} added to primary identity {}",
        secondary_dcc_id, primary_dcc_id
    ))
}

/// Removes a secondary identity from the linked identity record
pub fn _remove_secondary_identity(
    primary_pubkey_bytes: Vec<u8>,
    secondary_pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    // Verify the signature
    let primary_dcc_id = DccIdentity::new_verifying_from_bytes(&primary_pubkey_bytes)?;
    primary_dcc_id.verify_bytes(&secondary_pubkey_bytes, &crypto_signature)?;

    fn_info!("{}", primary_dcc_id);

    // Create the secondary identity
    let secondary_dcc_id = DccIdentity::new_verifying_from_bytes(&secondary_pubkey_bytes)?;

    // Remove the secondary identity
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        common_remove_secondary_identity(
            &mut ledger,
            &primary_dcc_id,
            &secondary_dcc_id,
            get_timestamp_ns(),
        )
    })?;

    Ok(format!(
        "Secondary identity {} removed from primary identity {}",
        secondary_dcc_id, primary_dcc_id
    ))
}

/// Sets a new primary identity for the linked identity record
pub fn _set_primary_identity(
    old_primary_pubkey_bytes: Vec<u8>,
    new_primary_pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    // Verify the signature
    let old_primary_dcc_id = DccIdentity::new_verifying_from_bytes(&old_primary_pubkey_bytes)?;
    old_primary_dcc_id.verify_bytes(&new_primary_pubkey_bytes, &crypto_signature)?;

    fn_info!("{}", old_primary_dcc_id);

    // Create the new primary identity
    let new_primary_dcc_id = DccIdentity::new_verifying_from_bytes(&new_primary_pubkey_bytes)?;

    // Set the new primary identity
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        common_set_primary_identity(
            &mut ledger,
            &old_primary_dcc_id,
            &new_primary_dcc_id,
            get_timestamp_ns(),
        )
    })?;

    Ok(format!(
        "Primary identity changed from {} to {}",
        old_primary_dcc_id, new_primary_dcc_id
    ))
}

/// Lists all identities linked to the given primary identity
pub fn _list_linked_identities(pubkey_bytes: Vec<u8>) -> Result<Vec<Vec<u8>>, String> {
    // Create the primary identity
    let primary_dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes)?;

    fn_info!("{}", primary_dcc_id);

    // List the linked identities
    LEDGER_MAP.with(|ledger| {
        let ledger = ledger.borrow();
        common_list_linked_identities(&ledger, &primary_dcc_id)
    })
}

/// Checks if the signing identity is authorized to act on behalf of the primary identity
pub fn _authorize_operation(
    primary_pubkey_bytes: Vec<u8>,
    signing_pubkey_bytes: Vec<u8>,
) -> Result<bool, String> {
    // Create the primary identity
    let primary_dcc_id = DccIdentity::new_verifying_from_bytes(&primary_pubkey_bytes)?;

    // Create the signing identity
    let signing_dcc_id = DccIdentity::new_verifying_from_bytes(&signing_pubkey_bytes)?;

    fn_info!(
        "Checking if {} is authorized to act on behalf of {}",
        signing_dcc_id,
        primary_dcc_id
    );

    // Check if the signing identity is authorized
    LEDGER_MAP.with(|ledger| {
        let ledger = ledger.borrow();
        common_authorize_operation(&ledger, &primary_dcc_id, &signing_dcc_id)
    })
}
