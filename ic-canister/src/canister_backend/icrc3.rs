// Standard description: https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md
// Reference implementation: https://github.com/dfinity/ic/blob/master/rs/rosetta-api/icrc1/ledger/src/main.rs

pub mod block_types;

use crate::canister_backend::generic::LEDGER_MAP;
use block_types::*;
use blocks::*;
use icrc_ledger_types::icrc3::blocks::{GetBlocksRequest, GetBlocksResult};

// The structs below do not seem to be available in the upstream crate:
// https://crates.io/crates/icrc-ledger-types/0.1.5

/// The history of blocks that the provided principal still did not fetch.
#[derive(CandidType, Deserialize)]
pub struct Icrc3RequestArgsHistoryForPrincipal {
    // Equivalent to: GetArchivesArgs in the reference ICRC-3 Ledger Canister
    pub from: Option<Principal>,
}

pub type Icrc3ResponseHistoryForPrincipal = Vec<Icrc3HistoryForPrincipal>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Icrc3HistoryForPrincipal {
    // The id of the archive
    pub canister_id: Principal,

    // The first block in the archive
    pub start: Nat,

    // The last block in the archive
    pub end: Nat,
}

// ICRC-3 has a notion of "archive" canisters, where all transactions and blocks are permanently stored.
// We store all transactions in the same kv store, so we just need to satisfy the same interface.
pub fn _icrc3_get_archives(
    args: Icrc3RequestArgsHistoryForPrincipal,
) -> Icrc3ResponseHistoryForPrincipal {
    LEDGER_MAP.with(|ledger| icrc3_get_history_canisters_and_block_numbers(&ledger.borrow(), args))
}

pub fn _icrc3_get_tip_certificate() -> Option<ICRC3DataCertificate> {
    let certificate = ByteBuf::from(ic_cdk::api::data_certificate()?);
    let hash_tree = LEDGER_MAP.with(|ledger| ledger.construct_hash_tree());
    let mut tree_buf = vec![];
    ciborium::ser::into_writer(&hash_tree, &mut tree_buf).unwrap();
    Some(icrc_ledger_types::icrc3::blocks::ICRC3DataCertificate {
        certificate,
        hash_tree: ByteBuf::from(tree_buf),
    })
}

pub fn _icrc3_get_blocks(args: Vec<GetBlocksRequest>) -> Icrc3GetBlocksResult {
    LEDGER_MAP.with(|ledger| ledger.icrc3_get_blocks(args))
}

pub fn _icrc3_supported_block_types() -> Vec<icrc_ledger_types::icrc3::blocks::SupportedBlockType> {
    use icrc_ledger_types::icrc3::blocks::SupportedBlockType;

    vec![
        SupportedBlockType {
            block_type: "1burn".to_string(),
            url: "https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-1/README.md"
                .to_string(),
        },
        SupportedBlockType {
            block_type: "1mint".to_string(),
            url: "https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-1/README.md"
                .to_string(),
        },
    ]
}

fn icrc3_generic_block_tx_from_funds_transfer(transfer: &FundsTransfer) -> Map {}

// https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md#icrc-1-and-icrc-2-block-schema
fn icrc3_generic_block_from_ledger_block(ledger_block: &LedgerBlock) -> Vec<GenericBlock> {
    let mut block = Map::new();

    // phash + ts are mandatory
    block.insert(
        "phash".to_string(),
        Value::Blob(ByteBuf::from(ledger_block.phash())),
    );
    block.insert("ts".to_string(), Value::Nat(ledger_block.timestamp()));

    for tx_val in ledger_block
        .entries()
        .into_iter()
        .filter(|e| e.label() == LABEL_DC_TOKEN_TRANSFER)
        .map(|e| {
            let transfer: FundsTransfer = BorshDeserialize::try_from_slice(e.value())
                .unwrap_or_else(|e| {
                    ic_cdk::api::trap(&format!(
                        "Failed to deserialize transfer {:?} ==> {:?}",
                        e, e
                    ));
                });
            let mut tx_val = Map::new();
            tx_val.insert("tid".to_string(), Value::Blob(ByteBuf::from(e.key())));
            tx_val.insert("from".to_string(), icrc3_value_account(transfer.from()));
            tx_val.insert("to".to_string(), icrc3_value_account(transfer.to()));
            tx_val.insert(
                "memo".to_string(),
                Value::Blob(ByteBuf::from(transfer.memo())),
            );
            tx_val.insert("ts".to_string(), Value::Nat(transfer.created_at_time()));
            tx_val
        })
    {
        block.insert("tx".to_string(), Value::Map(val));
        Value::Map(block)
    }
}

fn icrc3_value_account(account: Account) -> Value {
    let mut parts = Vec::new().reserve(2);
    parts.push(Value::blob(account.owner.as_slice()));
    if let Some(subaccount) = account.subaccount {
        parts.push(Value::blob(subaccount.as_slice()));
    }
    Value::Array(parts)
}

#[derive(CandidType, Serialize)]
pub struct BlockWithId {
    pub id: u64,
    pub block: GenericBlock,
}

pub fn get_blocks(arg: GetBlocksRequest) -> GetBlocksResponse {
    const MAX_BLOCKS_PER_RESPONSE: u64 = 100;

    let (start_block_idx, max_length) = arg
        .as_start_and_length()
        .unwrap_or_else(|msg| ic_cdk::api::trap(&msg));
    let max_length = max_length.min(MAX_BLOCKS_PER_RESPONSE);

    let next_block_tx_count = LEDGER_MAP.with(|ledger| {
        ledger
            .borrow()
            .get_next_block_entries_count(Some(LABEL_DC_TOKEN_TRANSFER))
    }) as u64;

    let mut response = GetBlocksResponse {
        first_index: Nat::from(0u64),
        chain_length: Nat::from(get_ledger_blocks_with_txs_committed() + next_block_tx_count),
        certificate: None,
        blocks: vec![],
        archived_blocks: vec![],
    };

    LEDGER_MAP.with(|ledger| {
        for (block_idx, block) in ledger.borrow().iter_raw().enumerate() {
            // Ignore blocks indexes before the provided start value
            if (block_idx as u64) < start_block_idx {
                continue;
            }

            // Decode the block and count the number of transactions
            let block = block.unwrap_or_else(|e| {
                ic_cdk::api::trap(&format!("Failed to deserialize block: {}", e));
            });
            let block = icrc3_generic_block_from_ledger_block(&block);
            if block.is_empty() {
                continue;
            }

            response.first_index = response.first_index.min((block_idx as u64).into());
            response.blocks.extend(block);
            if response.blocks.len() as u64 >= max_length {
                break;
            }
        }
    });

    response
}

use super::block_types::*;
use crate::canister_backend::generic::LEDGER_MAP;
use candid::Nat;
use icrc_ledger_types::icrc3::archive;
use icrc_ledger_types::icrc3::blocks::{GetBlocksRequest, GetBlocksResponse};
use ledger_map::LedgerMap;

/// Returns a list of Ledger canisters and the block numbers they hold.
/// The [args] can be used to support pagination.
/// The canister id in the args basically means: provide me all canister args after this.
/// Since we have only one canister, we return either a single entry (this canister), or an empty list.
pub fn icrc3_get_history_canisters_and_block_numbers(
    ledger: &LedgerMap,
    args: Icrc3RequestArgsHistoryForPrincipal,
) -> Icrc3ResponseHistoryForPrincipal {
    let my_canister_id = ic_cdk::api::id();
    if let Some(arg_principal) = args.from {
        if arg_principal <= my_canister_id {
            vec![]
        } else {
            vec![Icrc3HistoryForPrincipal {
                canister_id: my_canister_id,
                start: Nat::from(0),
                end: Nat::from(ledger.num_blocks()),
            }]
        }
    }
}

pub fn icrc3_get_blocks(ledger: &LedgerMap, args: Vec<GetBlocksRequest>) -> Icrc3GetBlocksResult {
    const MAX_BLOCKS_PER_RESPONSE: u64 = 100;

    let mut blocks = vec![];
    let mut archived_blocks_by_callback = BTreeMap::new();
    for arg in args {
        let (start, length) = arg
            .as_start_and_length()
            .unwrap_or_else(|msg| ic_cdk::api::trap(&msg));
        let max_length = MAX_BLOCKS_PER_RESPONSE.saturating_sub(blocks.len() as u64);
        if max_length == 0 {
            break;
        }
        let length = max_length.min(length).min(usize::MAX as u64) as usize;
        let (first_index, local_blocks, archived_ranges) = self.query_blocks(
            start,
            length,
            |block| ICRC3Value::from(encoded_block_to_generic_block(block)),
            |canister_id| {
                QueryArchiveFn::<Vec<GetBlocksRequest>, Icrc3GetBlocksResult>::new(
                    canister_id,
                    "icrc3_get_blocks",
                )
            },
        );
        for (id, block) in (first_index..).zip(local_blocks) {
            blocks.push(icrc_ledger_types::icrc3::blocks::BlockWithId {
                id: Nat::from(id),
                block,
            });
        }
        for ArchivedRange {
            start,
            length,
            callback,
        } in archived_ranges
        {
            let request = GetBlocksRequest { start, length };
            archived_blocks_by_callback
                .entry(callback)
                .or_insert(vec![])
                .push(request);
        }
        if blocks.len() as u64 >= MAX_BLOCKS_PER_RESPONSE {
            break;
        }
    }
    let mut archived_blocks = vec![];
    for (callback, args) in archived_blocks_by_callback {
        archived_blocks.push(Icrc3ArchivedBlocks { args, callback });
    }
    Icrc3GetBlocksResult {
        log_length: Nat::from(self.blockchain.chain_length()),
        blocks,
        archived_blocks,
    }
}
