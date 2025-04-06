use crate::{fn_info, AHashMap};
use borsh::{BorshDeserialize, BorshSerialize};
use candid::Principal;
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
use ic_cdk::println;
use ledger_map::LedgerMap;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

// Maximum number of linked identities allowed per primary
pub const MAX_LINKED_IDENTITIES: usize = 32;

thread_local! {
    // A link from a secondary identity to its primary identity
    pub static LINKED_PRINCIPALS_MAP: RefCell<AHashMap<Principal, Principal>> = RefCell::new(HashMap::default());
}

/// Get the primary principal for a given principal from the map
pub fn get_linked_primary_principal(secondary_principal: &Principal) -> Option<Principal> {
    LINKED_PRINCIPALS_MAP.with(|map| map.borrow().get(secondary_principal).copied())
}

/// Add secondary -> primary mappings to the map
pub fn link_secondary_to_primary_principal(primary: Principal, secondaries: Vec<Principal>) {
    LINKED_PRINCIPALS_MAP.with(|map| {
        for secondary in secondaries {
            map.borrow_mut().insert(secondary, primary);
        }
    });
}

/// Remove a secondary -> primary mapping from the map
pub fn unlink_secondary_from_primary_principal(
    primary: &Principal,
    secondaries: &Vec<Principal>,
) -> Vec<Principal> {
    let mut removed = Vec::new();
    LINKED_PRINCIPALS_MAP.with(|map| {
        for secondary in secondaries {
            // Check if the secondary principal is linked to the primary
            match map.borrow().get(secondary) {
                Some(linked_primary) if linked_primary == primary => {
                    // Remove the mapping
                    map.borrow_mut().remove(secondary);
                    removed.push(*linked_primary)
                }
                _ => {} // Secondary principal not found or not linked to the primary
            }
        }
    });
    removed
}

pub fn principal_serialize(
    principal: &Principal,
    writer: &mut impl borsh::io::Write,
) -> Result<(), borsh::io::Error> {
    BorshSerialize::serialize(&principal.as_slice(), writer)
}

pub fn vec_principal_serialize(
    principal: &[Principal],
    writer: &mut impl borsh::io::Write,
) -> Result<(), borsh::io::Error> {
    BorshSerialize::serialize(
        &principal.iter().map(|p| p.as_slice()).collect::<Vec<_>>(),
        writer,
    )
}

pub fn principal_deserialize(
    reader: &mut impl borsh::io::Read,
) -> Result<Principal, borsh::io::Error> {
    let principal: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
    Principal::try_from_slice(&principal).map_err(|_| {
        borsh::io::Error::new(borsh::io::ErrorKind::InvalidData, "Invalid Principal bytes")
    })
}

pub fn vec_principal_deserialize(
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
pub struct LinkedIcPrincipalsRecordV1 {
    #[borsh(
        serialize_with = "principal_serialize",
        deserialize_with = "principal_deserialize"
    )]
    primary_principal: Principal, // IC Principal of the primary identity
    #[borsh(
        serialize_with = "vec_principal_serialize",
        deserialize_with = "vec_principal_deserialize"
    )]
    secondary_principals: Vec<Principal>, // List of IC Principals of the secondary identities
    created_at: u64,      // Timestamp when the record was created
    last_updated_at: u64, // Timestamp when the record was last updated
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
pub enum LinkedIcIdsRecord {
    V1(LinkedIcPrincipalsRecordV1),
}

impl LinkedIcIdsRecord {
    /// Creates a new LinkedIdentityRecord with the given primary identity
    pub fn new(primary_principal: Principal, timestamp: u64) -> Self {
        LinkedIcIdsRecord::V1(LinkedIcPrincipalsRecordV1 {
            primary_principal,
            secondary_principals: Vec::new(),
            created_at: timestamp,
            last_updated_at: timestamp,
        })
    }

    /// Returns the primary identity principal
    pub fn primary_principal(&self) -> &Principal {
        match self {
            LinkedIcIdsRecord::V1(record) => &record.primary_principal,
        }
    }

    /// Returns the list of secondary principals
    pub fn secondary_principals(&self) -> &[Principal] {
        match self {
            LinkedIcIdsRecord::V1(record) => &record.secondary_principals,
        }
    }

    /// Adds a secondary principal to the record
    pub fn add_secondary_principal(
        &mut self,
        secondary_principal: Principal,
        timestamp: u64,
    ) -> Result<(), String> {
        match self {
            LinkedIcIdsRecord::V1(record) => {
                // Check if the secondary principal is the same as the primary
                if record.primary_principal == secondary_principal {
                    return Err("Cannot add primary principal as a secondary principal".to_string());
                }

                // Check if the secondary principal is already in the list
                if record.secondary_principals.contains(&secondary_principal) {
                    return Err("Secondary principal already linked".to_string());
                }

                // Check if we've reached the maximum number of linked identities
                if record.secondary_principals.len() >= MAX_LINKED_IDENTITIES {
                    return Err(format!(
                        "Cannot add more than {} linked identities",
                        MAX_LINKED_IDENTITIES
                    ));
                }

                // Add the secondary principal
                record.secondary_principals.push(secondary_principal);
                record.last_updated_at = timestamp;
                Ok(())
            }
        }
    }

    /// Removes a secondary principal from the record
    pub fn remove_secondary_principal(
        &mut self,
        secondary_principal: Principal,
        timestamp: u64,
    ) -> Result<(), String> {
        match self {
            LinkedIcIdsRecord::V1(record) => {
                if let Some(pos) = record
                    .secondary_principals
                    .iter()
                    .position(|p| p == &secondary_principal)
                {
                    record.secondary_principals.remove(pos);
                    record.last_updated_at = timestamp;
                    Ok(())
                } else {
                    Err("Secondary principal not found".to_string())
                }
            }
        }
    }
}

#[named]
/// Links identities by adding a secondary principal to a primary principal's record
pub fn link_identities(
    _ledger: &mut LedgerMap,
    primary_principal: Principal,
    secondary_principals: Vec<Principal>,
    _timestamp: u64,
) -> Result<String, String> {
    fn_info!("for principal {}", primary_principal);
    link_secondary_to_primary_principal(primary_principal, secondary_principals);
    Ok("Successfully linked identities".to_string())
}

#[named]
/// Unlinks identities by removing secondary principals from a primary principal's record
pub fn unlink_identities(
    _ledger: &mut LedgerMap,
    primary_principal: Principal,
    secondary_principals: Vec<Principal>,
    _timestamp: u64,
) -> Result<String, String> {
    fn_info!("for principal {}", primary_principal);
    let unlinked =
        unlink_secondary_from_primary_principal(&primary_principal, &secondary_principals);
    if unlinked.is_empty() {
        return Err("No valid linked identities found to unlink".to_string());
    }
    Ok(format!(
        "Successfully unlinked identities: {:?} from {}",
        unlinked, primary_principal
    ))
}

#[named]
/// Lists all principals linked to the given primary principal
pub fn list_linked_identities(primary_principal: Principal) -> Result<Vec<Principal>, String> {
    fn_info!("for principal {}", primary_principal);
    // Collect all secondary principals that map to this primary
    let mut secondaries = Vec::new();
    LINKED_PRINCIPALS_MAP.with(|map| {
        for (secondary, primary) in map.borrow().iter() {
            if *primary == primary_principal {
                secondaries.push(*secondary);
            }
        }
    });

    if secondaries.is_empty() {
        Err("No linked identities found".to_string())
    } else {
        Ok(secondaries)
    }
}

#[named]
/// Gets the primary principal for a given principal (if it exists)
pub fn get_primary_principal(principal: Principal) -> Result<Principal, String> {
    fn_info!("for principal {}", principal);
    match get_linked_primary_principal(&principal) {
        Some(primary) => Ok(primary),
        None => Ok(principal), // If not found as secondary, it might be a primary
    }
}
