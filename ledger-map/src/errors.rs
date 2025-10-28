#[derive(Debug, PartialEq)]
pub enum LedgerError {
    EntryNotFound,
    BlockEmpty,
    BlockCorrupted(String),
    UnsupportedBlockVersion(u32),
    TooManyEntriesInBlock(String),
    BlockTooLarge(String),
    SerializationError(String),
    Other(String),
}

impl From<std::io::Error> for LedgerError {
    fn from(error: std::io::Error) -> Self {
        LedgerError::Other(error.to_string())
    }
}

impl From<LedgerError> for String {
    fn from(error: LedgerError) -> Self {
        error.to_string()
    }
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::EntryNotFound => write!(f, "Entry not found"),
            LedgerError::BlockEmpty => write!(f, "Block is empty"),
            LedgerError::BlockCorrupted(err) => write!(f, "Block corrupted: {}", err),
            LedgerError::UnsupportedBlockVersion(version) => {
                write!(f, "Unsupported block version: {}", version)
            }
            LedgerError::TooManyEntriesInBlock(err) => {
                write!(f, "Too many entries in block: {}", err)
            }
            LedgerError::BlockTooLarge(err) => write!(f, "Block too large: {}", err),
            LedgerError::SerializationError(err) => write!(f, "Serialization error: {}", err),
            LedgerError::Other(err) => write!(f, "Other error: {}", err),
        }
    }
}

impl std::error::Error for LedgerError {}
