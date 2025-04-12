use crate::canister_backend::linked_identity::*;
use candid::Principal;

/// Links principals by adding alternate principals to a main principal's record
#[ic_cdk::update]
fn link_principals(
    main_principal: Principal,
    alt_principals: Vec<Principal>,
) -> Result<String, String> {
    _link_principals(main_principal, alt_principals)
}

/// Unlinks principals by removing alternate principals from a main principal's record
#[ic_cdk::update]
fn unlink_principals(
    main_principal: Principal,
    alt_principals: Vec<Principal>,
) -> Result<String, String> {
    _unlink_principals(main_principal, alt_principals)
}

/// Lists alternate principals linked to the given main principal
#[ic_cdk::query]
fn list_alt_principals(main_principal: Principal) -> Result<Vec<Principal>, String> {
    _list_alt_principals(main_principal)
}

/// Gets the main principal for a given alternate principal
#[ic_cdk::query]
fn get_main_principal(alt_principal: Principal) -> Result<Principal, String> {
    _get_main_principal(alt_principal)
}
