use crate::debug;
use crate::partition_table;
use borsh::{BorshDeserialize, BorshSerialize};

/// Struct representing the metadata of the ledger.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct MetadataV1 {
    /// The number of blocks in the ledger so far.
    num_blocks: usize,
    /// The offset in the persistent storage where the previous-to-tip block was written.
    prev_block_start_pos: Option<u64>,
    /// The chain hash of the entire ledger.
    tip_block_chain_hash: Vec<u8>,
    /// The timestamp of the last block
    tip_block_timestamp_ns: u64,
    /// The offset in the persistent storage where the tip (last completed) block is written.
    tip_block_start_pos: Option<u64>,
    /// The offset in the persistent storage where the next block will be written.
    next_block_start_pos: u64,
    /// The offset in the persistent storage where the first block was written.
    first_block_start_pos: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub enum Metadata {
    V1(MetadataV1),
}

impl Default for Metadata {
    fn default() -> Self {
        let next_block_start_pos = partition_table::get_data_partition().start_lba;
        debug!("next_block_start_pos: 0x{:0x}", next_block_start_pos);
        Metadata::V1(MetadataV1 {
            num_blocks: 0,
            prev_block_start_pos: None,
            tip_block_chain_hash: Vec::new(),
            tip_block_timestamp_ns: 0,
            tip_block_start_pos: Some(next_block_start_pos),
            next_block_start_pos,
            first_block_start_pos: next_block_start_pos,
        })
    }
}

impl Metadata {
    pub fn new() -> Self {
        Metadata::default()
    }

    pub fn clear(&mut self) {
        *self = Metadata::default();
    }

    pub fn num_blocks(&self) -> usize {
        match self {
            Metadata::V1(metadata) => metadata.num_blocks,
        }
    }

    pub fn prev_block_start_pos(&self) -> Option<u64> {
        match self {
            Metadata::V1(metadata) => metadata.prev_block_start_pos,
        }
    }

    pub fn tip_block_chain_hash(&self) -> &[u8] {
        match self {
            Metadata::V1(metadata) => metadata.tip_block_chain_hash.as_slice(),
        }
    }

    pub fn tip_block_timestamp_ns(&self) -> u64 {
        match self {
            Metadata::V1(metadata) => metadata.tip_block_timestamp_ns,
        }
    }

    pub fn tip_block_start_pos(&self) -> Option<u64> {
        match self {
            Metadata::V1(metadata) => metadata.tip_block_start_pos,
        }
    }

    pub fn next_block_start_pos(&self) -> u64 {
        match self {
            Metadata::V1(metadata) => metadata.next_block_start_pos,
        }
    }

    pub fn first_block_start_pos(&self) -> u64 {
        match self {
            Metadata::V1(metadata) => metadata.first_block_start_pos,
        }
    }

    pub fn update_from_appended_block(
        &mut self,
        new_chain_hash: &[u8],
        block_timestamp_ns: u64,
        next_block_start_pos: u64,
    ) {
        match self {
            Metadata::V1(metadata) => {
                metadata.num_blocks += 1;
                let block_start_pos = metadata.next_block_start_pos;
                metadata.prev_block_start_pos = metadata.tip_block_start_pos;
                metadata.tip_block_chain_hash = new_chain_hash.to_vec();
                metadata.tip_block_timestamp_ns = block_timestamp_ns;
                metadata.tip_block_start_pos = Some(metadata.next_block_start_pos);
                metadata.next_block_start_pos = next_block_start_pos;
                if block_start_pos > 0 && block_start_pos < metadata.first_block_start_pos {
                    metadata.first_block_start_pos = block_start_pos;
                }
            }
        }
    }

    pub(crate) fn get_last_block_chain_hash(&self) -> &[u8] {
        match self {
            Metadata::V1(metadata) => metadata.tip_block_chain_hash.as_slice(),
        }
    }

    pub(crate) fn get_last_block_timestamp_ns(&self) -> u64 {
        match self {
            Metadata::V1(metadata) => metadata.tip_block_timestamp_ns,
        }
    }
}
