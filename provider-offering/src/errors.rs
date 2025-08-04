/// Error types for parsing and operations
#[derive(Debug)]
pub enum OfferingError {
    ParseError(String),
    CsvError(csv::Error),
    IoError(std::io::Error),
    SerdeJsonError(serde_json::Error),
    SerializationError(String),
    InvalidPubkeyLength(usize),
    OfferingNotFound(String, String),
    ProviderNotFound(String),
}

impl From<csv::Error> for OfferingError {
    fn from(err: csv::Error) -> Self {
        OfferingError::CsvError(err)
    }
}

impl From<std::io::Error> for OfferingError {
    fn from(err: std::io::Error) -> Self {
        OfferingError::IoError(err)
    }
}

impl From<serde_json::Error> for OfferingError {
    fn from(err: serde_json::Error) -> Self {
        OfferingError::SerdeJsonError(err)
    }
}

impl std::fmt::Display for OfferingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OfferingError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            OfferingError::CsvError(err) => write!(f, "CSV error: {}", err),
            OfferingError::IoError(err) => write!(f, "IO error: {}", err),
            OfferingError::SerdeJsonError(err) => write!(f, "Serde JSON error: {}", err),
            OfferingError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            OfferingError::InvalidPubkeyLength(len) => {
                write!(f, "Invalid provider pubkey: expected 32 bytes, got {}", len)
            }
            OfferingError::OfferingNotFound(provider, key) => {
                write!(f, "Offering not found: provider={}, key={}", provider, key)
            }
            OfferingError::ProviderNotFound(provider) => {
                write!(f, "Provider not found: {}", provider)
            }
        }
    }
}

impl std::error::Error for OfferingError {}
