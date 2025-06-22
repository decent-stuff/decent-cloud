#[derive(Debug, PartialEq)]
pub enum LedgerError {
    EntryNotFound,
    BlockEmpty,
    BlockCorrupted(String),
    UnsupportedBlockVersion(u32),
    Other(String),
}

impl<E: std::error::Error> From<E> for LedgerError {
    fn from(error: E) -> Self {
        LedgerError::Other(error.to_string())
    }
}

impl From<LedgerError> for anyhow::Error {
    fn from(error: LedgerError) -> Self {
        anyhow::anyhow!("{}", error)
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
            LedgerError::Other(err) => write!(f, "Other error: {}", err),
        }
    }
}

impl std::error::Error for LedgerError {}
