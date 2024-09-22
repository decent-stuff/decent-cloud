use super::generic::LEDGER_MAP;
use crate::canister_backend::generic::encode_to_cbor_bytes;
use borsh::BorshDeserialize;
use dcc_common::{
    cache_transactions::RecentCache, get_ledger_txs_num_committed, FundsTransfer,
    LABEL_DC_TOKEN_TRANSFER,
};
use ic_certification::{HashTreeNode, Label};
use icrc_ledger_types::icrc3::blocks::DataCertificate as DataCertificatePreIcrc3;
use icrc_ledger_types::icrc3::transactions::{
    GetTransactionsRequest, GetTransactionsResponse, Transaction,
};
use ledger_map::LedgerMap;
use serde_bytes::ByteBuf;

/// Get both committed and uncommitted transactions.
pub fn _get_transactions(req: GetTransactionsRequest) -> GetTransactionsResponse {
    // Reference ledger implementation unfortunately only stores a single Transaction per block
    // so for clarity we rename all their references to block numbers into transaction numbers

    // Give me txs_length transactions starting at txs_from
    let (txs_from, txs_length) = req
        .as_start_and_length()
        .unwrap_or_else(|msg| ic_cdk::api::trap(&msg));

    let count_total_txs_committed = get_ledger_txs_num_committed();
    let count_total_txs_uncommitted = LEDGER_MAP.with(|ledger| {
        ledger
            .borrow()
            .get_next_block_entries_count(Some(LABEL_DC_TOKEN_TRANSFER))
    }) as u64;
    let mut txs = _get_committed_transactions(txs_from, txs_length);
    let txs_missing = txs_length.saturating_sub(txs.len() as u64);
    if txs_missing > 0 {
        let txs_uncommitted = _get_uncommitted_transactions(txs_missing);
        txs.extend(txs_uncommitted);
    }
    GetTransactionsResponse {
        // We don't have archived transactions in this implementation, so the first_index is always the requested tx number
        first_index: txs_from.into(),
        log_length: (count_total_txs_committed + count_total_txs_uncommitted).into(),
        transactions: txs,
        archived_transactions: vec![],
    }
}

pub fn ledger_construct_hash_tree(ledger: &LedgerMap) -> HashTreeNode {
    let hash = ledger.get_latest_block_hash();
    let last_block_index = ledger.get_blocks_count();
    HashTreeNode::Fork(Box::new((
        HashTreeNode::Labeled(
            Label::from("last_block_index"),
            Box::new(HashTreeNode::Leaf(last_block_index.to_be_bytes().to_vec())),
        ),
        HashTreeNode::Labeled(
            Label::from("last_block_hash"),
            Box::new(HashTreeNode::Leaf(hash.as_slice().to_vec())),
        ),
    )))
}

// Borrowed from https://github.com/ldclabs/ic-sft/blob/4825d760811731476ffbbb1705295a6ad4aae58f/src/ic_sft_canister/src/api_icrc3.rs#L57
pub fn get_tip_certificate(ledger: &LedgerMap) -> DataCertificatePreIcrc3 {
    let certificate = ByteBuf::from(
        ic_cdk::api::data_certificate()
            .unwrap_or_else(|| ic_cdk::api::trap("failed to get data certificate from the IC")),
    );
    let hash_tree = ledger_construct_hash_tree(ledger);
    let buf = encode_to_cbor_bytes(&hash_tree);
    DataCertificatePreIcrc3 {
        certificate: Some(certificate),
        hash_tree: ByteBuf::from(buf),
    }
}

pub fn _get_data_certificate() -> DataCertificatePreIcrc3 {
    LEDGER_MAP.with(|ledger| get_tip_certificate(&ledger.borrow()))
}

/// Get transactions committed to the ledger.
/// This implementation is inefficient and should be avoided.
/// The IC ledger implementation only stores a single Transaction per block so we
/// iterate over all blocks from the beginning, until we reach the "start".
/// This can be a problem if ledger is large.
/// We should be able to optimize this in the future with some caching.
/// Committed ledger is immutable, so caching should be easy.
fn _get_committed_transactions(start: u64, max_length: u64) -> Vec<Transaction> {
    // First check in the RecentCache
    if let Some(tx_num) = RecentCache::get_min_tx_num() {
        if start >= tx_num {
            // We have the entries in the cache
            return RecentCache::get_transactions(start, start + max_length);
        }
    }

    // We did not have the entries in the cache, get them from the ledger
    let mut txs_result = vec![];
    let mut tx_num: u64 = 0;

    LEDGER_MAP.with(|ledger| {
        'outer: for block in ledger.borrow().iter_raw() {
            let (_blk_header, ledger_block) = block.unwrap_or_else(|e| {
                ic_cdk::api::trap(&format!("Failed to deserialize block: {}", e));
            });

            // Extract the transactions from the block
            for entry in ledger_block.entries() {
                if entry.label() == LABEL_DC_TOKEN_TRANSFER {
                    if tx_num < start {
                        tx_num += 1;
                        continue;
                    }
                    let transfer: FundsTransfer = BorshDeserialize::try_from_slice(entry.value())
                        .expect("Failed to deserialize funds transfer");
                    txs_result.push(transfer.into());
                    tx_num += 1;

                    if txs_result.len() as u64 >= max_length {
                        break 'outer;
                    }
                }
            }
        }
    });
    txs_result
}

fn _get_uncommitted_transactions(max_length: u64) -> Vec<Transaction> {
    let mut txs = vec![];
    LEDGER_MAP.with(|ledger| {
        for entry in ledger
            .borrow()
            .next_block_iter(Some(LABEL_DC_TOKEN_TRANSFER))
        {
            let transfer: FundsTransfer = BorshDeserialize::try_from_slice(entry.value())
                .unwrap_or_else(|e| {
                    ic_cdk::api::trap(&format!(
                        "Failed to deserialize transfer {:?} ==> {:?}",
                        entry, e
                    ));
                });
            txs.push(transfer.into());
            if txs.len() as u64 >= max_length {
                break;
            }
        }
    });
    txs
}
