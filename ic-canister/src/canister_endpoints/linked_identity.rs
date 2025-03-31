use crate::canister_backend::linked_identity::*;

/// Creates a new linked identity record for the given primary identity
#[ic_cdk::update]
fn create_linked_record(
    pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _create_linked_record(pubkey_bytes, crypto_signature)
}

/// Adds a secondary identity to the linked identity record
#[ic_cdk::update]
fn add_secondary_identity(
    primary_pubkey_bytes: Vec<u8>,
    secondary_pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _add_secondary_identity(
        primary_pubkey_bytes,
        secondary_pubkey_bytes,
        crypto_signature,
    )
}

/// Removes a secondary identity from the linked identity record
#[ic_cdk::update]
fn remove_secondary_identity(
    primary_pubkey_bytes: Vec<u8>,
    secondary_pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _remove_secondary_identity(
        primary_pubkey_bytes,
        secondary_pubkey_bytes,
        crypto_signature,
    )
}

/// Sets a new primary identity for the linked identity record
#[ic_cdk::update]
fn set_primary_identity(
    old_primary_pubkey_bytes: Vec<u8>,
    new_primary_pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _set_primary_identity(
        old_primary_pubkey_bytes,
        new_primary_pubkey_bytes,
        crypto_signature,
    )
}

/// Lists all identities linked to the given primary identity
#[ic_cdk::query]
fn list_linked_identities(pubkey_bytes: Vec<u8>) -> Result<Vec<Vec<u8>>, String> {
    _list_linked_identities(pubkey_bytes)
}

/// Checks if the signing identity is authorized to act on behalf of the primary identity
#[ic_cdk::query]
fn authorize_operation(
    primary_pubkey_bytes: Vec<u8>,
    signing_pubkey_bytes: Vec<u8>,
) -> Result<bool, String> {
    _authorize_operation(primary_pubkey_bytes, signing_pubkey_bytes)
}
