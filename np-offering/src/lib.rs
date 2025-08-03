//! # np-offering: Network Provider Offering Management
//!
//! A robust framework for managing server offerings from network providers
//! in the Decent Cloud ecosystem with efficient search and indexing capabilities.

// Public modules
pub mod csv_constants;
pub mod enums;
pub mod errors;
pub mod registry;
pub mod serialization;
pub mod server_offering;
pub mod types;

// Private modules
mod provider_offerings;

// Re-export main public types
pub use enums::*;
pub use errors::OfferingError;
pub use provider_offerings::ProviderOfferings;
pub use registry::OfferingRegistry;
pub use server_offering::ServerOffering;
pub use types::{Offering, OfferingFilter, OfferingKey, ProviderPubkey, SearchQuery};
