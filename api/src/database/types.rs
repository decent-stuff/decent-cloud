use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct LedgerEntryData {
    pub label: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub block_timestamp_ns: u64,
    pub block_hash: Vec<u8>,
    pub block_offset: u64,
}

#[derive(Clone)]
pub struct Database {
    pub(crate) pool: SqlitePool,
}
