use crate::enums::{Currency, ProductType, StockStatus, VirtualizationType};
use crate::errors::OfferingError;
use crate::server_offering::ServerOffering;
use serde::{Deserialize, Serialize};

/// Strong-typed 32-byte provider public key
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderPubkey([u8; 32]);

impl ProviderPubkey {
    /// Create a new ProviderPubkey from a 32-byte array
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create from a slice, returning error if not exactly 32 bytes
    pub fn from_slice(bytes: &[u8]) -> Result<Self, OfferingError> {
        if bytes.len() != 32 {
            return Err(OfferingError::InvalidPubkeyLength(bytes.len()));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(Self(array))
    }

    /// Get the underlying 32-byte array
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to Vec<u8> for compatibility
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Display as hex string for debugging
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// Offering key type (maps to unique_internal_identifier)
pub type OfferingKey = String;

/// Complete offering with provider context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offering {
    pub provider_pubkey: ProviderPubkey,
    pub server_offering: ServerOffering,
}

impl Offering {
    /// Create a new offering
    pub fn new(provider_pubkey: ProviderPubkey, server_offering: ServerOffering) -> Self {
        Self {
            provider_pubkey,
            server_offering,
        }
    }

    /// Get the offering key (unique_internal_identifier)
    pub fn key(&self) -> &str {
        &self.server_offering.unique_internal_identifier
    }

    /// Get the provider pubkey
    pub fn provider(&self) -> &ProviderPubkey {
        &self.provider_pubkey
    }
}

/// Filters for structured offering search
#[derive(Debug, Clone)]
pub enum OfferingFilter {
    PriceRange(f64, f64),
    ProductType(ProductType),
    Country(String),
    City(String),
    HasGPU(bool),
    Currency(Currency),
    StockStatus(StockStatus),
    MinMemoryGB(u32),
    MinCores(u32),
    VirtualizationType(VirtualizationType),
}

/// Compound search query
#[derive(Debug, Default)]
pub struct SearchQuery {
    pub provider_pubkey: Option<ProviderPubkey>,
    pub offering_key: Option<String>,
    pub text_filter: Option<String>,
    pub filters: Vec<OfferingFilter>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl SearchQuery {
    /// Create a new empty search query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by specific provider
    pub fn with_provider(mut self, provider: ProviderPubkey) -> Self {
        self.provider_pubkey = Some(provider);
        self
    }

    /// Filter by specific offering key
    pub fn with_key(mut self, key: &str) -> Self {
        self.offering_key = Some(key.to_string());
        self
    }

    /// Add text search filter
    pub fn with_text(mut self, text: &str) -> Self {
        self.text_filter = Some(text.to_string());
        self
    }

    /// Add structured filter
    pub fn with_filter(mut self, filter: OfferingFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Limit number of results
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skip first N results
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}
