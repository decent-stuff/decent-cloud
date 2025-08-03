//! Serialization methods and pagination for ProviderOfferings.

use crate::{OfferingError, ProviderOfferings};
use serde::{Deserialize, Serialize};

/// Response wrapper for paginated results with size control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedOfferingsResponse {
    pub offerings: Vec<String>, // JSON strings for compatibility
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
    pub has_more: bool,
    pub total_bytes: usize,
}

impl PaginatedOfferingsResponse {
    pub fn new(
        offerings: Vec<ProviderOfferings>,
        total_count: usize,
        page: usize,
        page_size: usize,
        max_bytes: usize,
    ) -> Result<Self, OfferingError> {
        let mut json_offerings = Vec::new();
        let mut total_bytes = 0;
        let has_more = (page + 1) * page_size < total_count;

        for offering in offerings {
            let json = offering.serialize_as_json()?;
            let json_bytes = json.len();

            // Check if adding this offering would exceed the limit
            if total_bytes + json_bytes > max_bytes && !json_offerings.is_empty() {
                break;
            }

            json_offerings.push(json);
            total_bytes += json_bytes;
        }

        Ok(Self {
            offerings: json_offerings,
            total_count,
            page,
            page_size,
            has_more,
            total_bytes,
        })
    }

    /// Convert back to ProviderOfferings for processing
    pub fn to_provider_offerings(&self) -> Result<Vec<ProviderOfferings>, OfferingError> {
        self.offerings
            .iter()
            .map(|json| ProviderOfferings::deserialize_from_json(json))
            .collect()
    }
}

/// Utility functions for offering search responses
pub struct OfferingResponseBuilder {
    max_response_bytes: usize,
    current_bytes: usize,
    offerings: Vec<(Vec<u8>, Vec<u8>)>, // (pubkey, serialized_offering) pairs
}

impl OfferingResponseBuilder {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_response_bytes: max_bytes,
            current_bytes: 0,
            offerings: Vec::new(),
        }
    }

    /// Try to add an offering to the response
    pub fn try_add_offering(
        &mut self,
        offering: &ProviderOfferings,
    ) -> Result<bool, OfferingError> {
        let json = offering.serialize_as_json()?;
        let json_bytes = json.as_bytes();

        // Check if this would exceed the limit
        if self.current_bytes + json_bytes.len() > self.max_response_bytes
            && !self.offerings.is_empty()
        {
            return Ok(false); // Can't fit, but response has content
        }

        self.offerings
            .push((offering.provider_pubkey.clone(), json_bytes.to_vec()));
        self.current_bytes += json_bytes.len();
        Ok(true)
    }

    /// Get the final response
    pub fn build(self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.offerings
    }

    /// Get current response size
    pub fn current_size(&self) -> usize {
        self.current_bytes
    }

    /// Check if response has room for more
    pub fn has_room(&self, estimated_bytes: usize) -> bool {
        self.current_bytes + estimated_bytes <= self.max_response_bytes
    }
}
