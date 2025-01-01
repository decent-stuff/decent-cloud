use crate::LABEL_DC_TOKEN_TRANSFER;
use borsh::BorshDeserialize;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use icrc_ledger_types::icrc3::transactions::Transaction;
use ledger_map::LedgerBlock;
use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::FundsTransfer;

const CACHE_MAX_LENGTH: usize = 100_000; // Keep at most 100k entries with the highest ids in the cache

thread_local! {
    /// Total count of transactions that were committed and are in stable memory
    pub static CACHE_TXS_NUM_COMMITTED: RefCell<u64> = const { RefCell::new(0) };
    /// Recently committed transactions, that can be served from the get_transactions endpoint
    static RECENT_CACHE: RefCell<BTreeMap<u64, Transaction>> = const { RefCell::new(BTreeMap::new()) };
}

pub fn get_ledger_txs_num_committed() -> u64 {
    CACHE_TXS_NUM_COMMITTED.with(|n| *n.borrow())
}

/// Caches up to <CACHE_MAX_LENGTH> entries with the highest entry number.
/// The entry key can be the transaction number or the block number, and we keep in the cache
/// the entries with the highest entry number
pub struct RecentCache {}

impl RecentCache {
    /// Add a transaction to the cache, if the transaction is sufficiently enough
    pub fn add_entry(tx_num: u64, tx: Transaction) {
        RECENT_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            Self::_add_entry(&mut cache, tx_num, tx)
        })
    }

    fn _add_entry(cache: &mut BTreeMap<u64, Transaction>, tx_num: u64, tx: Transaction) {
        if cache.len() < CACHE_MAX_LENGTH || tx_num > *cache.keys().next().unwrap_or(&0) {
            // Only insert the entry if the cache is not full or if the new entry has a higher id than the minimal
            cache.insert(tx_num, tx);
        }

        // If the number of entries exceeds the maximum length, remove the oldest entries
        while cache.len() > CACHE_MAX_LENGTH {
            if let Some((&first_key, _)) = cache.iter().next() {
                cache.remove(&first_key);
            }
        }
    }

    pub fn get_min_tx_num() -> Option<u64> {
        RECENT_CACHE.with(|cache| cache.borrow().keys().next().copied())
    }

    pub fn get_num_entries() -> usize {
        RECENT_CACHE.with(|cache| cache.borrow().len())
    }

    // Get a transaction from the cache
    pub fn get_transaction(tx_num: u64) -> Option<Transaction> {
        RECENT_CACHE.with(|cache| cache.borrow().get(&tx_num).cloned())
    }

    // Get a range of transactions from the cache
    pub fn get_transactions(tx_num_start: u64, tx_num_end: u64) -> Vec<Transaction> {
        RECENT_CACHE.with(|cache| {
            cache
                .borrow()
                .range(tx_num_start..tx_num_end)
                .map(|(_k, v)| v.clone())
                .collect()
        })
    }

    // Remove a transaction from the cache
    pub fn remove_transaction(tx_num: u64) -> Option<Transaction> {
        RECENT_CACHE.with(|cache| cache.borrow_mut().remove(&tx_num))
    }

    // Clear the entire cache
    pub fn clear_cache() {
        RECENT_CACHE.with(|cache| {
            cache.borrow_mut().clear();
        });
    }

    // Get the current size of the cache
    pub fn cache_size() -> usize {
        RECENT_CACHE.with(|cache| cache.borrow().len())
    }

    /// Parse transactions from a LedgerBlock and append transactions to the cache.
    /// tx_num_start is the lowest transaction number in the block.
    /// ledger_block is the LedgerBlock that contains the transactions.
    /// Returns the parsed transactions.
    pub fn parse_ledger_block(tx_num_start: u64, ledger_block: &LedgerBlock) {
        let mut tx_num = tx_num_start;
        RECENT_CACHE.with(|cache| {
            for entry in ledger_block.entries() {
                if entry.label() == LABEL_DC_TOKEN_TRANSFER {
                    let transfer: FundsTransfer = BorshDeserialize::try_from_slice(entry.value())
                        .expect("Failed to deserialize funds transfer");
                    let tx: Transaction = transfer.into();
                    Self::_add_entry(&mut cache.borrow_mut(), tx_num, tx.clone());
                    tx_num += 1;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::{account_balance_get, get_timestamp_ns, TokenAmountE9s, MINTING_ACCOUNT};

    use super::*;
    use candid::Principal;
    use icrc_ledger_types::{
        icrc1::account::Account,
        icrc3::transactions::{Mint, Transaction},
    };

    fn create_dummy_transaction(id: u64) -> Transaction {
        Transaction {
            kind: "mint".into(),
            timestamp: id * 1000,
            mint: Some(Mint {
                amount: ((id + 1) * 1_000_000).into(),
                to: Account {
                    owner: Principal::from_slice(&id.to_be_bytes()),
                    subaccount: None,
                },
                memo: None,
                created_at_time: None,
            }),
            burn: None,
            transfer: None,
            approve: None,
        }
    }

    #[test]
    fn test_add_and_get_transaction() {
        RecentCache::clear_cache();
        let tx = create_dummy_transaction(1);
        RecentCache::add_entry(1, tx.clone());

        assert_eq!(RecentCache::get_transaction(1), Some(tx));
        assert_eq!(RecentCache::get_transaction(2), None);
    }

    #[test]
    fn test_cache_size_limit() {
        RecentCache::clear_cache();
        for i in 0..CACHE_MAX_LENGTH + 10 {
            RecentCache::add_entry(i as u64, create_dummy_transaction(i as u64));
        }

        assert_eq!(RecentCache::cache_size(), CACHE_MAX_LENGTH);
        assert_eq!(RecentCache::get_min_tx_num(), Some(10));
    }

    #[test]
    fn test_get_transactions_range() {
        RecentCache::clear_cache();
        for i in 0..10 {
            RecentCache::add_entry(i, create_dummy_transaction(i));
        }

        let transactions = RecentCache::get_transactions(3, 7);
        assert_eq!(transactions.len(), 4);
        assert_eq!(transactions[0].timestamp, 3000);
        assert_eq!(transactions[3].timestamp, 6000);
    }

    #[test]
    fn test_remove_transaction() {
        RecentCache::clear_cache();
        RecentCache::add_entry(1, create_dummy_transaction(1));

        assert_eq!(RecentCache::cache_size(), 1);
        let removed_tx = RecentCache::remove_transaction(1);
        assert!(removed_tx.is_some());
        assert_eq!(RecentCache::cache_size(), 0);
    }

    #[test]
    fn test_clear_cache() {
        RecentCache::clear_cache();
        for i in 0..5 {
            RecentCache::add_entry(i, create_dummy_transaction(i));
        }

        assert_eq!(RecentCache::cache_size(), 5);
        RecentCache::clear_cache();
        assert_eq!(RecentCache::cache_size(), 0);
    }

    #[test]
    fn test_parse_ledger_block() {
        RecentCache::clear_cache();

        fn create_dummy_funds_transfer(to: u64, amount: TokenAmountE9s) -> FundsTransfer {
            let account = crate::IcrcCompatibleAccount {
                owner: Principal::from_slice(&to.to_be_bytes()),
                subaccount: None,
            };
            let balance_to_after = account_balance_get(&account) + amount;
            FundsTransfer::new(
                MINTING_ACCOUNT,
                account,
                None,
                None,
                Some(get_timestamp_ns()),
                vec![],
                amount,
                0,
                balance_to_after,
            )
        }

        // Create a dummy LedgerBlock
        let mut entries = Vec::new();
        for i in 0..103 {
            let transfer = create_dummy_funds_transfer(i, (i + 1) as TokenAmountE9s);
            let entry = ledger_map::LedgerEntry::new(
                LABEL_DC_TOKEN_TRANSFER,
                transfer.to_tx_id(),
                borsh::to_vec(&transfer).unwrap(),
                ledger_map::Operation::Upsert,
            );
            entries.push(entry);
        }
        let ledger_block = LedgerBlock::new(entries, 0, vec![]);

        // Pretend that the first free transaction number is 899
        RecentCache::parse_ledger_block(899, &ledger_block);

        assert_eq!(RecentCache::cache_size(), 103);
        assert_eq!(RecentCache::get_min_tx_num(), Some(899));
        assert!(RecentCache::get_transaction(898).is_none());
        assert!(RecentCache::get_transaction(899).is_some());
        assert!(RecentCache::get_transaction(1001).is_some()); // in range [899..1001] (inclusive) there are 103 elements
        assert!(RecentCache::get_transaction(1002).is_none());
        assert!(RecentCache::get_transaction(1003).is_none());
    }
}
