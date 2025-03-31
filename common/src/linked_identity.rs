use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
use ic_cdk::println;
use ledger_map::LedgerMap;
use serde::{Deserialize, Serialize};

use crate::{fn_info, AHashMap, DccIdentity};

// Label for storing LinkedIdentityRecord in the LedgerMap
pub const LABEL_LINKED_IDENTITY: &str = "LinkedIdentity";

// Maximum number of linked identities allowed per primary
pub const MAX_LINKED_IDENTITIES: usize = 32;

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
pub struct LinkedIdentityRecordV1 {
    primary_identity: Vec<u8>, // Public key bytes of the primary identity
    secondary_identities: Vec<Vec<u8>>, // List of public key bytes for secondary identities
    created_at: u64,           // Timestamp when the record was created
    last_updated_at: u64,      // Timestamp when the record was last updated
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone)]
pub enum LinkedIdentityRecord {
    V1(LinkedIdentityRecordV1),
}

impl LinkedIdentityRecord {
    /// Creates a new LinkedIdentityRecord with the given primary identity
    pub fn new(primary_identity: &DccIdentity, timestamp: u64) -> Self {
        LinkedIdentityRecord::V1(LinkedIdentityRecordV1 {
            primary_identity: primary_identity.to_bytes_verifying(),
            secondary_identities: Vec::new(),
            created_at: timestamp,
            last_updated_at: timestamp,
        })
    }

    /// Returns the primary identity public key bytes
    pub fn primary_identity(&self) -> &[u8] {
        match self {
            LinkedIdentityRecord::V1(record) => &record.primary_identity,
        }
    }

    /// Returns the list of secondary identity public key bytes
    pub fn secondary_identities(&self) -> &[Vec<u8>] {
        match self {
            LinkedIdentityRecord::V1(record) => &record.secondary_identities,
        }
    }

    /// Returns the timestamp when the record was created
    pub fn created_at(&self) -> u64 {
        match self {
            LinkedIdentityRecord::V1(record) => record.created_at,
        }
    }

    /// Returns the timestamp when the record was last updated
    pub fn last_updated_at(&self) -> u64 {
        match self {
            LinkedIdentityRecord::V1(record) => record.last_updated_at,
        }
    }

    /// Adds a secondary identity to the record
    pub fn add_secondary_identity(
        &mut self,
        secondary_identity: &DccIdentity,
        timestamp: u64,
    ) -> Result<(), String> {
        match self {
            LinkedIdentityRecord::V1(record) => {
                let secondary_bytes = secondary_identity.to_bytes_verifying();

                // Check if the secondary identity is the same as the primary
                if record.primary_identity == secondary_bytes {
                    return Err("Cannot add primary identity as a secondary identity".to_string());
                }

                // Check if the secondary identity is already in the list
                if record
                    .secondary_identities
                    .iter()
                    .any(|id| id == &secondary_bytes)
                {
                    return Err("Secondary identity already linked".to_string());
                }

                // Check if we've reached the maximum number of linked identities
                if record.secondary_identities.len() >= MAX_LINKED_IDENTITIES {
                    return Err(format!(
                        "Cannot add more than {} linked identities",
                        MAX_LINKED_IDENTITIES
                    ));
                }

                // Add the secondary identity
                record.secondary_identities.push(secondary_bytes);
                record.last_updated_at = timestamp;
                Ok(())
            }
        }
    }

    /// Removes a secondary identity from the record
    pub fn remove_secondary_identity(
        &mut self,
        secondary_identity: &DccIdentity,
        timestamp: u64,
    ) -> Result<(), String> {
        match self {
            LinkedIdentityRecord::V1(record) => {
                let secondary_bytes = secondary_identity.to_bytes_verifying();

                // Find the index of the secondary identity
                let index = record
                    .secondary_identities
                    .iter()
                    .position(|id| id == &secondary_bytes);

                match index {
                    Some(idx) => {
                        // Remove the secondary identity
                        record.secondary_identities.remove(idx);
                        record.last_updated_at = timestamp;
                        Ok(())
                    }
                    None => Err("Secondary identity not found".to_string()),
                }
            }
        }
    }

    /// Checks if the given identity is authorized to act on behalf of the primary identity
    pub fn is_authorized(&self, identity: &DccIdentity) -> bool {
        let identity_bytes = identity.to_bytes_verifying();

        // Check if the identity is the primary
        if self.primary_identity() == identity_bytes {
            return true;
        }

        // Check if the identity is one of the secondaries
        self.secondary_identities()
            .iter()
            .any(|id| id == &identity_bytes)
    }
}

/// Creates a new linked identity record for the given primary identity
#[named]
pub fn create_linked_record(
    ledger: &mut LedgerMap,
    primary_identity: &DccIdentity,
    timestamp: u64,
) -> Result<(), String> {
    let primary_bytes = primary_identity.to_bytes_verifying();

    // Check if a record already exists for this primary identity
    if ledger.contains(LABEL_LINKED_IDENTITY, &primary_bytes) {
        return Err("Linked identity record already exists for this primary identity".to_string());
    }

    // Create a new record
    let record = LinkedIdentityRecord::new(primary_identity, timestamp);
    let record_bytes = borsh::to_vec(&record).map_err(|e| e.to_string())?;

    // Store the record in the ledger
    ledger
        .upsert(LABEL_LINKED_IDENTITY, &primary_bytes, record_bytes)
        .map_err(|e| e.to_string())?;

    fn_info!("Created linked identity record for {}", primary_identity);
    Ok(())
}

/// Gets or creates a linked identity record for the given primary identity
#[named]
pub fn get_or_create_linked_record(
    ledger: &mut LedgerMap,
    primary_identity: &DccIdentity,
    timestamp: u64,
) -> Result<LinkedIdentityRecord, String> {
    let primary_bytes = primary_identity.to_bytes_verifying();

    // Check if a record already exists for this primary identity
    if ledger.contains(LABEL_LINKED_IDENTITY, &primary_bytes) {
        // Get the existing record
        let record_bytes = ledger
            .get(LABEL_LINKED_IDENTITY, &primary_bytes)
            .map_err(|e| e.to_string())?;

        LinkedIdentityRecord::try_from_slice(&record_bytes).map_err(|e| e.to_string())
    } else {
        // Create a new record
        let record = LinkedIdentityRecord::new(primary_identity, timestamp);
        let record_bytes = borsh::to_vec(&record).map_err(|e| e.to_string())?;

        // Store the record in the ledger
        ledger
            .upsert(LABEL_LINKED_IDENTITY, &primary_bytes, record_bytes)
            .map_err(|e| e.to_string())?;

        fn_info!("Created linked identity record for {}", primary_identity);
        Ok(record)
    }
}

/// Adds a secondary identity to the linked identity record
#[named]
pub fn add_secondary_identity(
    ledger: &mut LedgerMap,
    primary_identity: &DccIdentity,
    secondary_identity: &DccIdentity,
    timestamp: u64,
) -> Result<(), String> {
    let primary_bytes = primary_identity.to_bytes_verifying();

    // Get or create the record
    let mut record = get_or_create_linked_record(ledger, primary_identity, timestamp)?;

    // Add the secondary identity
    record.add_secondary_identity(secondary_identity, timestamp)?;

    // Update the record in the ledger
    let record_bytes = borsh::to_vec(&record).map_err(|e| e.to_string())?;
    ledger
        .upsert(LABEL_LINKED_IDENTITY, &primary_bytes, record_bytes)
        .map_err(|e| e.to_string())?;

    fn_info!(
        "Added secondary identity {} to primary identity {}",
        secondary_identity,
        primary_identity
    );
    Ok(())
}

/// Removes a secondary identity from the linked identity record
#[named]
pub fn remove_secondary_identity(
    ledger: &mut LedgerMap,
    primary_identity: &DccIdentity,
    secondary_identity: &DccIdentity,
    timestamp: u64,
) -> Result<(), String> {
    let primary_bytes = primary_identity.to_bytes_verifying();

    // Check if a record exists for this primary identity
    if !ledger.contains(LABEL_LINKED_IDENTITY, &primary_bytes) {
        return Err("No linked identity record found for this primary identity".to_string());
    }

    // Get the record
    let record_bytes = ledger
        .get(LABEL_LINKED_IDENTITY, &primary_bytes)
        .map_err(|e| e.to_string())?;

    let mut record =
        LinkedIdentityRecord::try_from_slice(&record_bytes).map_err(|e| e.to_string())?;

    // Remove the secondary identity
    record.remove_secondary_identity(secondary_identity, timestamp)?;

    // Update the record in the ledger
    let record_bytes = borsh::to_vec(&record).map_err(|e| e.to_string())?;
    ledger
        .upsert(LABEL_LINKED_IDENTITY, &primary_bytes, record_bytes)
        .map_err(|e| e.to_string())?;

    fn_info!(
        "Removed secondary identity {} from primary identity {}",
        secondary_identity,
        primary_identity
    );
    Ok(())
}

/// Sets a new primary identity for the linked identity record
#[named]
pub fn set_primary_identity(
    ledger: &mut LedgerMap,
    old_primary: &DccIdentity,
    new_primary: &DccIdentity,
    timestamp: u64,
) -> Result<(), String> {
    let old_primary_bytes = old_primary.to_bytes_verifying();
    let new_primary_bytes = new_primary.to_bytes_verifying();

    // Check if a record exists for the old primary identity
    if !ledger.contains(LABEL_LINKED_IDENTITY, &old_primary_bytes) {
        return Err("No linked identity record found for the old primary identity".to_string());
    }

    // Check if a record already exists for the new primary identity
    if ledger.contains(LABEL_LINKED_IDENTITY, &new_primary_bytes) {
        return Err(
            "Linked identity record already exists for the new primary identity".to_string(),
        );
    }

    // Get the record for the old primary
    let record_bytes = ledger
        .get(LABEL_LINKED_IDENTITY, &old_primary_bytes)
        .map_err(|e| e.to_string())?;

    let mut record =
        LinkedIdentityRecord::try_from_slice(&record_bytes).map_err(|e| e.to_string())?;

    // Check if the new primary is already a secondary identity
    match &mut record {
        LinkedIdentityRecord::V1(record_v1) => {
            let new_primary_pos = record_v1
                .secondary_identities
                .iter()
                .position(|id| id == &new_primary_bytes);

            if let Some(idx) = new_primary_pos {
                // Remove the new primary from the secondary identities
                record_v1.secondary_identities.remove(idx);

                // Add the old primary as a secondary identity
                record_v1
                    .secondary_identities
                    .push(old_primary_bytes.clone());

                // Update the primary identity
                record_v1.primary_identity = new_primary_bytes.clone();
                record_v1.last_updated_at = timestamp;

                // Update the record in the ledger under the new primary key
                let record_bytes = borsh::to_vec(&record).map_err(|e| e.to_string())?;
                ledger
                    .upsert(LABEL_LINKED_IDENTITY, &new_primary_bytes, record_bytes)
                    .map_err(|e| e.to_string())?;

                // Remove the record under the old primary key
                ledger
                    .delete(LABEL_LINKED_IDENTITY, &old_primary_bytes)
                    .map_err(|e| e.to_string())?;

                fn_info!(
                    "Changed primary identity from {} to {}",
                    old_primary,
                    new_primary
                );
                Ok(())
            } else {
                Err("New primary identity is not a secondary identity in the record".to_string())
            }
        }
    }
}

/// Lists all identities linked to the given primary identity
#[named]
pub fn list_linked_identities(
    ledger: &LedgerMap,
    primary_identity: &DccIdentity,
) -> Result<Vec<Vec<u8>>, String> {
    let primary_bytes = primary_identity.to_bytes_verifying();

    // Check if a record exists for this primary identity
    if !ledger.contains(LABEL_LINKED_IDENTITY, &primary_bytes) {
        return Err("No linked identity record found for this primary identity".to_string());
    }

    // Get the record
    let record_bytes = ledger
        .get(LABEL_LINKED_IDENTITY, &primary_bytes)
        .map_err(|e| e.to_string())?;

    let record = LinkedIdentityRecord::try_from_slice(&record_bytes).map_err(|e| e.to_string())?;

    // Return the list of secondary identities
    let mut result = Vec::new();
    result.push(primary_bytes.clone()); // Include the primary identity
    result.extend_from_slice(record.secondary_identities());

    Ok(result)
}

/// Checks if the signing identity is authorized to act on behalf of the primary identity
#[named]
pub fn authorize_operation(
    ledger: &LedgerMap,
    primary_identity: &DccIdentity,
    signing_identity: &DccIdentity,
) -> Result<bool, String> {
    let primary_bytes = primary_identity.to_bytes_verifying();

    // If the signing identity is the primary identity, it's authorized
    if primary_identity.to_bytes_verifying() == signing_identity.to_bytes_verifying() {
        return Ok(true);
    }

    // Check if a record exists for this primary identity
    if !ledger.contains(LABEL_LINKED_IDENTITY, &primary_bytes) {
        return Ok(false);
    }

    // Get the record
    let record_bytes = ledger
        .get(LABEL_LINKED_IDENTITY, &primary_bytes)
        .map_err(|e| e.to_string())?;

    let record = LinkedIdentityRecord::try_from_slice(&record_bytes).map_err(|e| e.to_string())?;

    // Check if the signing identity is authorized
    Ok(record.is_authorized(signing_identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_timestamp_ns;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_identity() -> DccIdentity {
        let signing_key = SigningKey::generate(&mut OsRng);
        DccIdentity::new_signing(&signing_key).unwrap()
    }

    #[test]
    fn test_create_linked_record() {
        let mut ledger = LedgerMap::new();
        let primary = create_test_identity();
        let timestamp = get_timestamp_ns();

        // Create a new record
        let result = create_linked_record(&mut ledger, &primary, timestamp);
        assert!(result.is_ok());

        // Try to create it again (should fail)
        let result = create_linked_record(&mut ledger, &primary, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_secondary_identity() {
        let mut ledger = LedgerMap::new();
        let primary = create_test_identity();
        let secondary1 = create_test_identity();
        let secondary2 = create_test_identity();
        let timestamp = get_timestamp_ns();

        // Add a secondary identity
        let result = add_secondary_identity(&mut ledger, &primary, &secondary1, timestamp);
        assert!(result.is_ok());

        // Add another secondary identity
        let result = add_secondary_identity(&mut ledger, &primary, &secondary2, timestamp);
        assert!(result.is_ok());

        // Try to add the primary as a secondary (should fail)
        let result = add_secondary_identity(&mut ledger, &primary, &primary, timestamp);
        assert!(result.is_err());

        // Try to add the same secondary again (should fail)
        let result = add_secondary_identity(&mut ledger, &primary, &secondary1, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_secondary_identity() {
        let mut ledger = LedgerMap::new();
        let primary = create_test_identity();
        let secondary = create_test_identity();
        let timestamp = get_timestamp_ns();

        // Add a secondary identity
        let result = add_secondary_identity(&mut ledger, &primary, &secondary, timestamp);
        assert!(result.is_ok());

        // Remove the secondary identity
        let result = remove_secondary_identity(&mut ledger, &primary, &secondary, timestamp);
        assert!(result.is_ok());

        // Try to remove it again (should fail)
        let result = remove_secondary_identity(&mut ledger, &primary, &secondary, timestamp);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_primary_identity() {
        let mut ledger = LedgerMap::new();
        let old_primary = create_test_identity();
        let new_primary = create_test_identity();
        let timestamp = get_timestamp_ns();

        // Add the new primary as a secondary identity
        let result = add_secondary_identity(&mut ledger, &old_primary, &new_primary, timestamp);
        assert!(result.is_ok());

        // Set the new primary
        let result = set_primary_identity(&mut ledger, &old_primary, &new_primary, timestamp);
        assert!(result.is_ok());

        // Check that the record exists under the new primary
        let result = list_linked_identities(&ledger, &new_primary);
        assert!(result.is_ok());
        let identities = result.unwrap();
        assert_eq!(identities.len(), 2); // New primary + old primary as secondary

        // Check that the record doesn't exist under the old primary
        let result = list_linked_identities(&ledger, &old_primary);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_linked_identities() {
        let mut ledger = LedgerMap::new();
        let primary = create_test_identity();
        let secondary1 = create_test_identity();
        let secondary2 = create_test_identity();
        let timestamp = get_timestamp_ns();

        // Add secondary identities
        add_secondary_identity(&mut ledger, &primary, &secondary1, timestamp).unwrap();
        add_secondary_identity(&mut ledger, &primary, &secondary2, timestamp).unwrap();

        // List the linked identities
        let result = list_linked_identities(&ledger, &primary);
        assert!(result.is_ok());
        let identities = result.unwrap();
        assert_eq!(identities.len(), 3); // Primary + 2 secondaries
    }

    #[test]
    fn test_authorize_operation() {
        let mut ledger = LedgerMap::new();
        let primary = create_test_identity();
        let secondary = create_test_identity();
        let unauthorized = create_test_identity();
        let timestamp = get_timestamp_ns();

        // Add a secondary identity
        add_secondary_identity(&mut ledger, &primary, &secondary, timestamp).unwrap();

        // Check authorization
        let result = authorize_operation(&ledger, &primary, &primary);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result = authorize_operation(&ledger, &primary, &secondary);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result = authorize_operation(&ledger, &primary, &unauthorized);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_max_linked_identities() {
        let mut ledger = LedgerMap::new();
        let primary = create_test_identity();
        let timestamp = get_timestamp_ns();

        // Add MAX_LINKED_IDENTITIES secondary identities
        for _ in 0..MAX_LINKED_IDENTITIES {
            let secondary = create_test_identity();
            let result = add_secondary_identity(&mut ledger, &primary, &secondary, timestamp);
            assert!(result.is_ok());
        }

        // Try to add one more (should fail)
        let secondary = create_test_identity();
        let result = add_secondary_identity(&mut ledger, &primary, &secondary, timestamp);
        assert!(result.is_err());
    }
}
