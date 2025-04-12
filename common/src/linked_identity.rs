use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, fn_info, reward_e9s_per_block,
    AHashMap, IcrcCompatibleAccount, TokenAmountE9s, LABEL_LINKED_IC_IDS,
};
use borsh::{BorshDeserialize, BorshSerialize};
use candid::Principal;
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
use ic_cdk::println;
use ledger_map::{AHashSet, LedgerMap};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

// Maximum allowed number of linked identities allowed per main principal
pub const MAX_LINKED_PRINCIPALS: usize = 32;

fn principal_linking_fee_e9s() -> TokenAmountE9s {
    reward_e9s_per_block() / 10000
}

thread_local! {
    pub static LINKED_ALT_TO_MAIN: RefCell<AHashMap<Principal, Principal>> = RefCell::new(HashMap::default());
    pub static LINKED_MAIN_TO_ALT: RefCell<AHashMap<Principal, AHashSet<Principal>>> = RefCell::new(HashMap::default());
}

/// Get the main principal for a given principal from the map
pub fn cache_get_main_from_alt_principal(alt_principal: &Principal) -> Option<Principal> {
    LINKED_ALT_TO_MAIN.with(|map| map.borrow().get(alt_principal).copied())
}

pub fn cache_get_alt_principals_from_main(main_principal: &Principal) -> Option<Vec<Principal>> {
    LINKED_MAIN_TO_ALT.with(|map| {
        map.borrow()
            .get(main_principal)
            .map(|set| set.iter().copied().collect())
    })
}

/// Add alternate -> main mappings to the map
pub fn cache_link_alt_to_main_principal(main_principal: Principal, alt_principals: &[Principal]) -> Result<(), String> {
    if alt_principals.is_empty() {
        return Ok(());
    }
    for alt_principal in alt_principals {
        if let Some(prev_main) = cache_get_main_from_alt_principal(alt_principal) {
            return Err(format!(
                "Principal {} is already linked to main principal {}",
                alt_principal, prev_main
            ));
        }
    }
    LINKED_ALT_TO_MAIN.with(|map| {
        for alt_principal in alt_principals {
            map.borrow_mut().insert(*alt_principal, main_principal);
        }
    });
    LINKED_MAIN_TO_ALT.with(|map| {
        map.borrow_mut()
            .entry(main_principal)
            .or_default()
            .extend(alt_principals);
    });
    Ok(())
}

/// Remove an alt -> main mapping from the map
pub fn cache_unlink_alt_from_main_principal(
    main_principal: &Principal,
    alt_principals: &[Principal],
) {
    if alt_principals.is_empty() {
        return;
    }
    LINKED_ALT_TO_MAIN.with(|map| {
        let mut map = map.borrow_mut();
        for alt_principal in alt_principals {
            if map.get(alt_principal) == Some(main_principal) {
                println!("Removing {} from {}", alt_principal, main_principal);
                map.remove(alt_principal);
            }
        }
    });
    LINKED_MAIN_TO_ALT.with(|map| {
        map.borrow_mut().entry(*main_principal).and_modify(|set| {
            for alt_principal in alt_principals {
                set.remove(alt_principal);
            }
        });
    });
}

pub fn cache_update_from_ledger_record(record: &LinkedIcIdsRecord) {
    if let Err(err) = cache_link_alt_to_main_principal(*record.main_principal(), record.alt_principals_add() ) {
        println!("Error for cache_link_alt_to_main_principal: {}", err);
    }
    cache_unlink_alt_from_main_principal(record.main_principal(), record.alt_principals_rm());
}

pub fn serialize_ppal(
    principal: &Principal,
    writer: &mut impl borsh::io::Write,
) -> Result<(), borsh::io::Error> {
    BorshSerialize::serialize(&principal.as_slice(), writer)
}

pub fn serialize_vec_ppal(
    principal: &[Principal],
    writer: &mut impl borsh::io::Write,
) -> Result<(), borsh::io::Error> {
    BorshSerialize::serialize(
        &principal.iter().map(|p| p.as_slice()).collect::<Vec<_>>(),
        writer,
    )
}

pub fn deserialize_ppal(reader: &mut impl borsh::io::Read) -> Result<Principal, borsh::io::Error> {
    let principal: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
    Principal::try_from_slice(&principal).map_err(|_| {
        borsh::io::Error::new(borsh::io::ErrorKind::InvalidData, "Invalid Principal bytes")
    })
}

pub fn deserialize_vec_ppal(
    reader: &mut impl borsh::io::Read,
) -> Result<Vec<Principal>, borsh::io::Error> {
    let principals: Vec<Vec<u8>> = BorshDeserialize::deserialize_reader(reader)?;
    principals
        .into_iter()
        .map(|p| Principal::try_from_slice(&p))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            borsh::io::Error::new(borsh::io::ErrorKind::InvalidData, "Invalid Principal bytes")
        })
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
pub struct LinkedIcIdsRecordV1 {
    #[borsh(
        serialize_with = "serialize_ppal",
        deserialize_with = "deserialize_ppal"
    )]
    main_principal: Principal, // IC Principal of the main identity
    #[borsh(
        serialize_with = "serialize_vec_ppal",
        deserialize_with = "deserialize_vec_ppal"
    )]
    alt_principals_add: Vec<Principal>, // List of alt IC Principals to add
    #[borsh(
        serialize_with = "serialize_vec_ppal",
        deserialize_with = "deserialize_vec_ppal"
    )]
    alt_principals_rm: Vec<Principal>, // List of alt IC Principals to remove
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
pub enum LinkedIcIdsRecord {
    V1(LinkedIcIdsRecordV1),
}

impl LinkedIcIdsRecord {
    /// Creates a new LinkedIdentityRecord with the given main identity
    pub fn new(
        main_principal: Principal,
        alt_principals_add: Vec<Principal>,
        alt_principals_rm: Vec<Principal>,
    ) -> Self {
        LinkedIcIdsRecord::V1(LinkedIcIdsRecordV1 {
            main_principal,
            alt_principals_add,
            alt_principals_rm,
        })
    }

    /// Returns the main identity principal
    pub fn main_principal(&self) -> &Principal {
        match self {
            LinkedIcIdsRecord::V1(record) => &record.main_principal,
        }
    }

    /// Returns the list of alternate principals added
    pub fn alt_principals_add(&self) -> &[Principal] {
        match self {
            LinkedIcIdsRecord::V1(record) => &record.alt_principals_add,
        }
    }

    /// Returns the list of alternate principals removed
    pub fn alt_principals_rm(&self) -> &[Principal] {
        match self {
            LinkedIcIdsRecord::V1(record) => &record.alt_principals_rm,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, borsh::io::Error> {
        borsh::to_vec(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, borsh::io::Error> {
        LinkedIcIdsRecord::try_from_slice(bytes)
    }
}

#[named]
/// Links principals by adding alternate principals to a main principal's record
pub fn do_link_principals(
    ledger: &mut LedgerMap,
    main_principal: Principal,
    alt_principals_add: Vec<Principal>,
) -> Result<String, String> {
    fn_info!(
        "ADD main {} <-> alt {:?}",
        main_principal,
        alt_principals_add.iter().map(|p| p.to_string().split_once('-').unwrap().0.to_owned()).collect::<Vec<_>>()
    );

    let payload = LinkedIcIdsRecord::new(main_principal, alt_principals_add.clone(), vec![])
        .serialize()
        .unwrap();

    // Store the pubkey in the ledger
    let key = ledger.count_entries_for_label(LABEL_LINKED_IC_IDS).to_le_bytes();
    ledger
        .upsert(LABEL_LINKED_IC_IDS, &key, payload)
        .map_err(|e| e.to_string())
        .and_then(|_| {
            cache_link_alt_to_main_principal(main_principal, &alt_principals_add)
        })
        .and_then(|_| {
            let fee = principal_linking_fee_e9s();
            let icrc1_account = IcrcCompatibleAccount::new(main_principal, None);
            match charge_fees_to_account_no_bump_reputation(ledger, &icrc1_account, fee, "") {
                Ok(_) => Ok(format!(
                    "Successfully linked {} to {}. You have been charged {} tokens.",
                    alt_principals_add
                        .iter()
                        .map(|p| p.to_string().split_once('-').unwrap().0.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    main_principal,
                    amount_as_string(fee)
                )),
                Err(e) => Err(format!("Failed to charge the fees: {}", e)),
            }
        })
}

#[named]
/// Unlinks principals by removing alt principals from the main principal's record
pub fn do_unlink_principals(
    ledger: &mut LedgerMap,
    main_principal: Principal,
    alt_principals_rm: Vec<Principal>,
) -> Result<String, String> {
    fn_info!("RM {} rm alt {:?}", main_principal, alt_principals_rm.iter().map(|p| p.to_string().split_once('-').unwrap().0.to_owned()).collect::<Vec<_>>()
);

    // Create an updated record with the remaining alternate principals
    let payload = LinkedIcIdsRecord::new(main_principal, vec![], alt_principals_rm.clone())
        .serialize()
        .unwrap();

    // Update the ledger and the cache
    let key = ledger.count_entries_for_label(LABEL_LINKED_IC_IDS).to_le_bytes();
    ledger
        .upsert(LABEL_LINKED_IC_IDS, &key, payload)
        .map_err(|e| e.to_string())
        .map(|_| cache_unlink_alt_from_main_principal(&main_principal, &alt_principals_rm))
        .and_then(|_| {
            let fee = principal_linking_fee_e9s();
            let icrc1_account = IcrcCompatibleAccount::new(main_principal, None);
            match charge_fees_to_account_no_bump_reputation(ledger, &icrc1_account, fee, "") {
                Ok(_) => Ok(format!(
                    "Successfully unlinked {} from {}. You have been charged {} tokens.",
                    alt_principals_rm
                        .iter()
                        .map(|p| p.to_string().split_once('-').unwrap().0.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    main_principal,
                    amount_as_string(fee)
                )),
                Err(e) => Err(format!("Failed to charge the fees: {}", e)),
            }
        })
}

/// Lists all principals linked to the given main principal
pub fn do_list_alt_principals(main_principal: Principal) -> Result<Vec<Principal>, String> {
    Ok(cache_get_alt_principals_from_main(&main_principal).unwrap_or_default())
}

pub fn do_get_main_principal(alt_principal: Principal) -> Result<Principal, String> {
    match cache_get_main_from_alt_principal(&alt_principal) {
        Some(main_principal) => Ok(main_principal),
        None => Ok(alt_principal), // There is no main identity, return the alternate principal
    }
}
