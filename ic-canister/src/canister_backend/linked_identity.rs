use super::generic::LEDGER_MAP;
use candid::Principal;
use dcc_common::{
    get_primary_principal as common_get_primary_principal, get_timestamp_ns,
    link_identities as common_link_identities,
    list_linked_identities as common_list_linked_identities,
    unlink_identities as common_unlink_identities,
};

/// Links identities by adding secondary identities to a primary principal's record
pub fn _link_identities(
    primary_principal: Principal,
    secondary_principals: Vec<Principal>,
) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        common_link_identities(
            &mut ledger,
            primary_principal,
            secondary_principals,
            get_timestamp_ns(),
        )
    })
}

/// Unlinks identities by removing secondary identities from a primary principal's record
pub fn _unlink_identities(
    primary_principal: Principal,
    secondary_principals: Vec<Principal>,
) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        common_unlink_identities(
            &mut ledger,
            primary_principal,
            secondary_principals,
            get_timestamp_ns(),
        )
    })
}

/// Lists all identities linked to the given primary principal
pub fn _list_linked_identities(primary_principal: Principal) -> Result<Vec<Principal>, String> {
    common_list_linked_identities(primary_principal)
}

/// Gets the primary principal for a given IC principal
pub fn _get_primary_identity(principal: Principal) -> Result<Principal, String> {
    common_get_primary_principal(principal)
}
