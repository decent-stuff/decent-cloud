use crate::{AHashMap, DccIdentity, LABEL_REPUTATION_CHANGE, MAX_REPUTATION_INCREASE_PER_TX};
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::{warn, LedgerError, LedgerMap};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static REPUTATIONS: RefCell<AHashMap<Vec<u8>, u64>> = RefCell::new(HashMap::default());
}

pub fn reputation_get<S: AsRef<[u8]>>(verifying_pk: S) -> u64 {
    REPUTATIONS.with(|reputations| {
        let reputations = reputations.borrow();
        match reputations.get(verifying_pk.as_ref()) {
            Some(rep) => *rep,
            None => 0,
        }
    })
}

pub fn reputations_apply_changes(changes: &ReputationChange) {
    REPUTATIONS.with(|reputations| {
        let mut reputations = reputations.borrow_mut();
        for (verifying_pk, delta) in changes.changes() {
            let reputation = reputations.entry(verifying_pk.clone()).or_default();
            *reputation = (*reputation as i64 + delta).max(0) as u64;
        }
    });
}

pub fn reputations_apply_aging(reputation_age: &ReputationAge) {
    REPUTATIONS.with(|reputations| {
        let mut reputations = reputations.borrow_mut();
        for entry in reputations.iter_mut() {
            let delta = ((*entry.1) as u128 * reputation_age.reductions_ppm() as u128 / 1_000_000)
                .clamp(0, 100) as u64;
            *entry.1 = (*entry.1).saturating_sub(delta);
        }
    });
}

pub fn ledger_add_reputation_change(
    ledger: &mut LedgerMap,
    dcc_identity: &DccIdentity,
    delta: i64,
) -> Result<(), ReputationError> {
    if dcc_identity.is_minting_account().map_err(|e| ReputationError::Generic(e.to_string()))? {
        warn!("Attempted to add reputation change to minting account");
    } else {
        let delta = delta.min(MAX_REPUTATION_INCREASE_PER_TX);
        let entry = ReputationChange::new_single(dcc_identity.to_bytes_verifying(), delta);
        let entry_bytes = borsh::to_vec(&entry)?;

        let entry_id: [u8; 32] = Sha256::digest(&entry_bytes).into();
        ledger.upsert(LABEL_REPUTATION_CHANGE, entry_id, entry_bytes)?;

        reputations_apply_changes(&entry);
    }

    Ok(())
}

#[allow(dead_code)]
pub fn reputations_clear() {
    REPUTATIONS.with(|reputations| reputations.borrow_mut().clear());
}

type Identifier = Vec<u8>;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReputationChangeV1 {
    changes: Vec<(Identifier, i64)>,
}

/// Represents a list of reputation changes on specific principals
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ReputationChange {
    V1(ReputationChangeV1),
}

impl ReputationChange {
    /// Create a new list of reputation changes, for appending to the ledger
    /// verifying_pk: Verifying (public) key of the identity to change
    /// delta: The absolute change (positive or negative) to the identity's reputation
    pub fn new_single(verifying_pk: Identifier, delta: i64) -> Self {
        Self::V1(ReputationChangeV1 {
            changes: vec![(verifying_pk, delta)],
        })
    }

    /// Create a new list of reputation changes, for appending to the ledger
    pub fn new_many(changes: Vec<(Identifier, i64)>) -> Self {
        Self::V1(ReputationChangeV1 { changes })
    }

    pub fn changes(&self) -> &[(Identifier, i64)] {
        match self {
            ReputationChange::V1(r) => &r.changes,
        }
    }
}

/// Reductions of account reputations, applied to all accounts
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReputationAgeV1 {
    age_reductions_ppm: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ReputationAge {
    V1(ReputationAgeV1),
}

impl ReputationAge {
    pub fn new(age_reductions_ppm: u64) -> Self {
        Self::V1(ReputationAgeV1 { age_reductions_ppm })
    }

    pub fn reductions_ppm(&self) -> u64 {
        match self {
            ReputationAge::V1(r) => r.age_reductions_ppm,
        }
    }
}

#[derive(Debug)]
pub enum ReputationError {
    InvalidInput,
    Serialization(String),
    Ledger(LedgerError),
    Generic(String),
}

impl From<ReputationError> for String {
    fn from(e: ReputationError) -> Self {
        match e {
            ReputationError::InvalidInput => "Invalid input".to_string(),
            ReputationError::Serialization(s) => format!("Serialization error: {}", s),
            ReputationError::Ledger(e) => format!("Ledger error: {}", e),
            ReputationError::Generic(s) => s,
        }
    }
}

impl From<ReputationError> for anyhow::Error {
    fn from(e: ReputationError) -> Self {
        anyhow::anyhow!(e)
    }
}

impl From<borsh::io::Error> for ReputationError {
    fn from(e: borsh::io::Error) -> Self {
        ReputationError::Serialization(e.to_string())
    }
}

impl From<LedgerError> for ReputationError {
    fn from(e: LedgerError) -> Self {
        ReputationError::Ledger(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_get_existing() {
        reputations_clear();
        let identity = b"user1".to_vec();
        reputations_apply_changes(&ReputationChange::new_single(identity.clone(), 10));
        assert_eq!(reputation_get(identity), 10);
    }

    #[test]
    fn test_reputation_get_non_existing() {
        reputations_clear();
        let identity = b"non_existent_user".to_vec();
        assert_eq!(reputation_get(&identity), 0);
    }

    #[test]
    fn test_reputations_apply_changes_single_positive() {
        reputations_clear();
        let identity = b"user1".to_vec();
        reputations_apply_changes(&ReputationChange::new_single(identity.clone(), 10));
        assert_eq!(reputation_get(&identity), 10);
    }

    #[test]
    fn test_reputations_apply_changes_single_negative() {
        reputations_clear();
        let identity = b"user1".to_vec();
        reputations_apply_changes(&ReputationChange::new_single(identity.clone(), -5));
        assert_eq!(reputation_get(&identity), 0);
    }

    #[test]
    fn test_reputations_apply_changes_many() {
        reputations_clear();
        let changes = vec![
            (b"user1".to_vec(), 10),
            (b"user2".to_vec(), -5),
            (b"user3".to_vec(), 15),
        ];
        reputations_apply_changes(&ReputationChange::new_many(changes.clone()));
        for (identity, delta) in changes {
            assert_eq!(reputation_get(&identity), delta.max(0) as u64);
        }
    }

    #[test]
    fn test_reputations_apply_aging() {
        reputations_clear();
        let identity = b"user1".to_vec();
        reputations_apply_changes(&ReputationChange::new_single(identity.clone(), 100));
        let reputation_age = ReputationAge::new(10_000); // 1% reduction
        reputations_apply_aging(&reputation_age);
        // 1% of 100 is 1, so 100 - 1 = 99
        assert_eq!(reputation_get(&identity), 99);
    }

    #[test]
    fn test_reputations_apply_aging_edge_case() {
        reputations_clear();
        let identity = b"user1".to_vec();
        reputations_apply_changes(&ReputationChange::new_single(identity.clone(), 1));
        let reputation_age = ReputationAge::new(1_000_000); // 100% reduction
        reputations_apply_aging(&reputation_age);
        // 100% reduction should set reputation to 0
        assert_eq!(reputation_get(&identity), 0);
    }

    #[test]
    fn test_reputations_clear() {
        reputations_clear();
        let identity = b"user1".to_vec();
        reputations_apply_changes(&ReputationChange::new_single(identity.clone(), 10));
        reputations_clear();
        assert_eq!(reputation_get(&identity), 0);
    }

    #[test]
    fn test_reputation_change_new_single() {
        let identity = b"user1".to_vec();
        let delta = 5;
        let change = ReputationChange::new_single(identity.clone(), delta);
        assert_eq!(change.changes(), &[(identity, delta)]);
    }

    #[test]
    fn test_reputation_change_new_many() {
        let changes = vec![(b"user1".to_vec(), 5), (b"user2".to_vec(), -3)];
        let change = ReputationChange::new_many(changes.clone());
        assert_eq!(change.changes(), &changes[..]);
    }

    #[test]
    fn test_reputation_age_new() {
        let reductions_ppm = 1000;
        let reputation_age = ReputationAge::new(reductions_ppm);
        assert_eq!(reputation_age.reductions_ppm(), reductions_ppm);
    }
}
