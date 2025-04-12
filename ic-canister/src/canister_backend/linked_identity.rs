use super::generic::LEDGER_MAP;
use candid::Principal;
use dcc_common::{
    do_get_main_principal, do_link_principals, do_list_alt_principals, do_unlink_principals,
};

pub fn _link_principals(
    main_principal: Principal,
    alt_principals: Vec<Principal>,
) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        do_link_principals(&mut ledger, main_principal, alt_principals)
    })
}

pub fn _unlink_principals(
    main_principal: Principal,
    alt_principals: Vec<Principal>,
) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        do_unlink_principals(&mut ledger, main_principal, alt_principals)
    })
}

pub fn _list_alt_principals(main_principal: Principal) -> Result<Vec<Principal>, String> {
    do_list_alt_principals(main_principal)
}

pub fn _get_main_principal(alt_principal: Principal) -> Result<Principal, String> {
    do_get_main_principal(alt_principal)
}
