use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Request for next block sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextBlockSyncRequest {
    pub start_position: Option<u64>,
    pub include_data: bool,
    pub max_entries: Option<usize>,
}

impl Default for NextBlockSyncRequest {
    fn default() -> Self {
        Self {
            start_position: None,
            include_data: true,
            max_entries: None,
        }
    }
}

/// Response for next block sync
#[derive(Debug, Clone, Serialize, Deserialize, CandidType, Default)]
pub struct NextBlockSyncResponse {
    pub has_block: bool,
    pub block_header: Option<Vec<u8>>,
    pub block_data: Option<Vec<u8>>, // Reuse for serialized entries
    pub block_hash: Option<Vec<u8>>,
    pub block_position: Option<u64>,
    pub next_block_position: Option<u64>,
    pub entries_count: usize,
    pub more_blocks_available: bool,
}
