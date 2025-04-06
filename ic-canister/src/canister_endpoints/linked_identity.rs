use crate::canister_backend::linked_identity::*;
use candid::Principal;

/// Links identities by adding secondary identities to a primary principal's record
#[ic_cdk::update]
fn link_identities(
    primary_principal: Principal,
    secondary_principals: Vec<Principal>,
) -> Result<String, String> {
    _link_identities(primary_principal, secondary_principals)
}

/// Unlinks identities by removing secondary identities from a primary principal's record
#[ic_cdk::update]
fn unlink_identities(
    primary_principal: Principal,
    secondary_principals: Vec<Principal>,
) -> Result<String, String> {
    _unlink_identities(primary_principal, secondary_principals)
}

/// Lists all identities linked to the given primary principal
#[ic_cdk::query]
fn list_linked_identities(primary_principal: Principal) -> Result<Vec<Principal>, String> {
    _list_linked_identities(primary_principal)
}

/// Gets the primary principal for a given IC principal
#[ic_cdk::query]
fn get_primary_identity(principal: Principal) -> Result<Principal, String> {
    _get_primary_identity(principal)
}
