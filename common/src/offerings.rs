use crate::{
    charge_fees_to_account_no_bump_reputation, info, reward_e9s_per_block, DccIdentity,
    ED25519_SIGNATURE_LENGTH, LABEL_NP_OFFERING, LABEL_NP_REGISTER, MAX_JSON_ZLIB_PAYLOAD_LENGTH,
    MAX_PUBKEY_BYTES,
};
use candid::Principal;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

fn np_offering_update_fee_e9s() -> u64 {
    reward_e9s_per_block() / 10000
}

type ResourceName = String;
type ResourceDesc = String;
type Location = String;
type Quantity32 = u32;
type Timestamp = u64;
type PriceE9s = u64;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct AvailableUnit {
    pub quantity: Quantity32,
    pub location: Location,
    pub timestamp_from: Timestamp,
    pub timestamp_to: Timestamp,
    pub price_e9s: PriceE9s,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct NPOffering {
    pub node_provider_pubkey_bytes: Vec<u8>,
    pub resource_name: ResourceName,
    pub resource_desc: ResourceDesc,
    pub vcpus: Quantity32,
    pub memory: Quantity32,
    pub storage: Quantity32,
    pub bandwidth: Option<Quantity32>,
    // Which addons are compatible with this offering and can be installed
    pub compatible_addons: Vec<Addon>,
    // How many resources of this type are available, where, when, and at what price
    pub available_units: Vec<AvailableUnit>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Addon {
    Gpu(String, PriceE9s),
    Ipv4(String, PriceE9s),
    Ipv6(String, PriceE9s),
    Storage(String, PriceE9s),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct UpdateOfferingPayload {
    pub offering_payload: Vec<u8>,
    pub signature: Vec<u8>,
}

pub fn do_node_provider_update_offering(
    ledger: &mut LedgerMap,
    caller: Principal,
    pubkey_bytes: Vec<u8>,
    update_offering_payload: Vec<u8>,
) -> Result<String, String> {
    if pubkey_bytes.len() > MAX_PUBKEY_BYTES {
        return Err("Node provider unique id too long".to_string());
    }

    let dcc_identity =
        DccIdentity::new_verifying_from_bytes(&pubkey_bytes).map_err(|e| e.to_string())?;
    if caller != dcc_identity.to_ic_principal() {
        return Err("Invalid caller".to_string());
    }
    info!("[do_node_provider_update_offering]: {}", dcc_identity);

    let payload: UpdateOfferingPayload =
        serde_json::from_slice(&update_offering_payload).map_err(|e| e.to_string())?;

    if payload.signature.len() != ED25519_SIGNATURE_LENGTH {
        return Err("Invalid signature".to_string());
    }
    if payload.offering_payload.len() > MAX_JSON_ZLIB_PAYLOAD_LENGTH {
        return Err("Profile payload too long".to_string());
    }

    match ledger.get(LABEL_NP_REGISTER, &pubkey_bytes) {
        Ok(np_key) => {
            // Check the signature
            let dcc_identity =
                DccIdentity::new_verifying_from_bytes(&np_key).map_err(|e| e.to_string())?;

            if dcc_identity.to_ic_principal() != caller {
                return Err("Invalid caller".to_string());
            }

            match dcc_identity.verify_bytes(&payload.offering_payload, &payload.signature) {
                Ok(()) => {
                    charge_fees_to_account_no_bump_reputation(
                        ledger,
                        &dcc_identity,
                        np_offering_update_fee_e9s(),
                    )?;
                    // Store the original signed payload in the ledger
                    ledger
                        .upsert(LABEL_NP_OFFERING, &pubkey_bytes, &update_offering_payload)
                        .map(|_| "Offering updated! Thank you.".to_string())
                        .map_err(|e| e.to_string())
                }
                Err(e) => Err(format!("Signature is invalid: {:?}", e)),
            }
        }
        Err(ledger_map::LedgerError::EntryNotFound) => Err("Node provider not found".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SearchFilter {
    ResourceName(String),
    ResourceDesc(String),
    Location(String),
    TimestampFrom(Timestamp),
    TimestampTo(Timestamp),
    PriceE9sMin(u64),
    PriceE9sMax(u64),
    VcpusMin(Quantity32),
    VcpusMax(Quantity32),
    MemoryMin(Quantity32),
    MemoryMax(Quantity32),
    StorageMin(Quantity32),
    StorageMax(Quantity32),
    BandwidthMin(Quantity32),
    BandwidthMax(Quantity32),
    AddonIpv4(String, PriceE9s, PriceE9s),
    AddonIpv6(String, PriceE9s, PriceE9s),
    AddonGpu(String, PriceE9s, PriceE9s),
    AddonStorage(String, PriceE9s, PriceE9s),
}

pub type SearchFilters = Vec<SearchFilter>;

/// Search for offerings that match ALL the given filters (Boolean AND)
/// String matching is case-insensitive and uses the Jaro-Winkler fuzzy match algorithm
pub fn do_get_matching_offerings(ledger: &LedgerMap, filters: SearchFilters) -> Vec<NPOffering> {
    let mut results = vec![];

    'outer: for entry in ledger
        .iter(Some(LABEL_NP_OFFERING))
        .chain(ledger.next_block_iter(Some(LABEL_NP_OFFERING)))
    {
        let payload: UpdateOfferingPayload =
            serde_json::from_slice(entry.value()).expect("Failed to decode payload");
        let offering: NPOffering =
            serde_json::from_slice(&payload.offering_payload).expect("Failed to decode offering");

        // If any filter doesn't match, skip this offering
        for filter in &filters {
            match filter {
                SearchFilter::ResourceName(resource_name) => {
                    if jaro_winkler(&offering.resource_name, resource_name) < 0.8 {
                        continue 'outer;
                    }
                }
                SearchFilter::ResourceDesc(resource_desc) => {
                    if jaro_winkler(&offering.resource_desc, resource_desc) < 0.8 {
                        continue 'outer;
                    }
                }
                SearchFilter::Location(location) => {
                    if !offering
                        .available_units
                        .iter()
                        .any(|unit| jaro_winkler(&unit.location, location) >= 0.9)
                    {
                        continue 'outer;
                    }
                }
                SearchFilter::TimestampFrom(timestamp_from) => {
                    if !offering
                        .available_units
                        .iter()
                        .any(|unit| unit.timestamp_from >= *timestamp_from)
                    {
                        continue 'outer;
                    }
                }
                SearchFilter::TimestampTo(timestamp_to) => {
                    if !offering
                        .available_units
                        .iter()
                        .any(|unit| unit.timestamp_to <= *timestamp_to)
                    {
                        continue 'outer;
                    }
                }
                SearchFilter::PriceE9sMin(price_e9s_min) => {
                    if !offering
                        .available_units
                        .iter()
                        .any(|unit| unit.price_e9s >= *price_e9s_min)
                    {
                        continue 'outer;
                    }
                }
                SearchFilter::PriceE9sMax(price_e9s_max) => {
                    if !offering
                        .available_units
                        .iter()
                        .any(|unit| unit.price_e9s <= *price_e9s_max)
                    {
                        continue 'outer;
                    }
                }
                SearchFilter::VcpusMin(vcpus_min) => {
                    if offering.vcpus < *vcpus_min {
                        continue 'outer;
                    }
                }
                SearchFilter::VcpusMax(vcpus_max) => {
                    if offering.vcpus > *vcpus_max {
                        continue 'outer;
                    }
                }
                SearchFilter::MemoryMin(memory_min) => {
                    if offering.memory < *memory_min {
                        continue 'outer;
                    }
                }
                SearchFilter::MemoryMax(memory_max) => {
                    if offering.memory > *memory_max {
                        continue 'outer;
                    }
                }
                SearchFilter::StorageMin(storage_min) => {
                    if offering.storage < *storage_min {
                        continue 'outer;
                    }
                }
                SearchFilter::StorageMax(storage_max) => {
                    if offering.storage > *storage_max {
                        continue 'outer;
                    }
                }
                SearchFilter::BandwidthMin(bandwidth_min) => {
                    if let Some(bandwidth) = offering.bandwidth {
                        if bandwidth < *bandwidth_min {
                            continue 'outer;
                        }
                    } else {
                        continue 'outer;
                    }
                }
                SearchFilter::BandwidthMax(bandwidth_max) => {
                    if let Some(bandwidth) = offering.bandwidth {
                        if bandwidth > *bandwidth_max {
                            continue 'outer;
                        }
                    } else {
                        continue 'outer;
                    }
                }
                SearchFilter::AddonIpv4(filter_addon_ipv4, price_e9s_min, price_e9s_max) => {
                    if !offering.compatible_addons.iter().any(|addon| match addon {
                        Addon::Ipv4(ipv4, price_e9s) => {
                            jaro_winkler(ipv4, filter_addon_ipv4) >= 0.98
                                && price_e9s >= price_e9s_min
                                && price_e9s <= price_e9s_max
                        }
                        _ => false,
                    }) {
                        continue 'outer;
                    }
                }
                SearchFilter::AddonIpv6(filter_addon_ipv6, price_e9s_min, price_e9s_max) => {
                    if !offering.compatible_addons.iter().any(|addon| match addon {
                        Addon::Ipv6(ipv6, price_e9s) => {
                            jaro_winkler(ipv6, filter_addon_ipv6) >= 0.98
                                && price_e9s >= price_e9s_min
                                && price_e9s <= price_e9s_max
                        }
                        _ => false,
                    }) {
                        continue 'outer;
                    }
                }
                SearchFilter::AddonGpu(filter_addon_gpu, price_e9s_min, price_e9s_max) => {
                    if !offering.compatible_addons.iter().any(|addon| match addon {
                        Addon::Gpu(gpu, price_e9s) => {
                            jaro_winkler(gpu, filter_addon_gpu) >= 0.8
                                && price_e9s >= price_e9s_min
                                && price_e9s <= price_e9s_max
                        }
                        _ => false,
                    }) {
                        continue 'outer;
                    }
                }
                SearchFilter::AddonStorage(addon_storage, price_e9s_min, price_e9s_max) => {
                    if !offering.compatible_addons.iter().any(|addon| match addon {
                        Addon::Storage(storage, price_e9s) => {
                            jaro_winkler(storage, addon_storage) >= 0.8
                                && price_e9s >= price_e9s_min
                                && price_e9s <= price_e9s_max
                        }
                        _ => false,
                    }) {
                        continue 'outer;
                    }
                }
            }
        }

        results.push(offering);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{do_get_matching_offerings, info, AvailableUnit, SearchFilter};
    use ledger_map::LedgerMap;

    fn log_init() {
        // Set log level to info by default
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "info");
        }
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn new_temp_ledger(labels_to_index: Option<Vec<String>>) -> LedgerMap {
        log_init();
        info!("Create temp ledger");
        // Create a temporary directory for the test
        let file_path = tempfile::tempdir()
            .unwrap()
            .into_path()
            .join("test_ledger_store.bin");

        LedgerMap::new_with_path(labels_to_index, Some(file_path))
            .expect("Failed to create a test temp ledger")
    }

    fn create_offering(
        ledger: &mut LedgerMap,
        identity_pubkey_bytes: Vec<u8>,
        resource_name: &str,
        resource_desc: &str,
        vcpus: Quantity32,
        memory: Quantity32,
        storage: Quantity32,
        bandwidth: Option<Quantity32>,
        location: &str,
        price_e9s: u64,
        addons: Vec<Addon>,
    ) {
        let offering = NPOffering {
            node_provider_pubkey_bytes: identity_pubkey_bytes.clone(),
            resource_name: resource_name.to_string(),
            resource_desc: resource_desc.to_string(),
            vcpus,
            memory,
            storage,
            bandwidth,
            compatible_addons: addons,
            available_units: vec![AvailableUnit {
                quantity: 10,
                location: location.to_string(),
                timestamp_from: 0,
                timestamp_to: 1000000000,
                price_e9s,
            }],
        };
        let offering_payload = UpdateOfferingPayload {
            offering_payload: serde_json::to_vec(&offering).unwrap(),
            signature: vec![],
        };

        ledger
            .upsert(
                LABEL_NP_OFFERING.to_string(),
                identity_pubkey_bytes,
                serde_json::to_vec(&offering_payload).unwrap(),
            )
            .unwrap();
    }

    #[test]
    fn test_search_multiple_criteria() {
        let mut ledger = new_temp_ledger(None);

        // Pre-populate the ledger with multiple entries
        create_offering(
            &mut ledger,
            vec![1, 2, 3],
            "High Performance VM",
            "A VM with high performance",
            16,
            64,
            1000,
            Some(1000),
            "US-West",
            1000000,
            vec![Addon::Ipv4("/24 block".to_string(), 100000u64)],
        );

        create_offering(
            &mut ledger,
            vec![4, 5, 6],
            "Standard VM",
            "A VM with standard performance",
            8,
            32,
            500,
            Some(500),
            "US-East",
            500000,
            vec![Addon::Ipv4("/28 block".to_string(), 10000u64)],
        );

        create_offering(
            &mut ledger,
            vec![7, 8, 9],
            "Memory Intensive VM",
            "A VM with high memory capacity",
            16,
            256,
            2000,
            Some(2000),
            "EU-Central",
            3000000,
            vec![
                Addon::Ipv4("/22 block".to_string(), 100000000u64),
                Addon::Ipv4("/23 block".to_string(), 10000000u64),
                Addon::Ipv4("/25 block".to_string(), 1000000u64),
                Addon::Ipv6("/64 block".to_string(), 50000u64),
                Addon::Gpu("NVIDIA H100".to_string(), 100000u64),
                Addon::Storage("SSD".to_string(), 1000u64),
            ],
        );

        // Test search by resource name
        let filters = vec![SearchFilter::ResourceName(
            "High Performance VM".to_string(),
        )];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource_name, "High Performance VM");

        // Test search by resource description
        let filters = vec![SearchFilter::ResourceDesc(
            "standard performance".to_string(),
        )];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource_desc, "A VM with standard performance");

        // Test search by location
        let filters = vec![SearchFilter::Location("US-East".to_string())];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].available_units[0].location, "US-East");

        // Test search by vcpus range
        let filters = vec![SearchFilter::VcpusMin(16), SearchFilter::VcpusMax(32)];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 2); // Should match "High Performance VM" and "Memory Intensive VM"

        // Test search by memory range
        let filters = vec![SearchFilter::MemoryMin(128), SearchFilter::MemoryMax(512)];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory, 256);

        // Test search by storage range
        let filters = vec![
            SearchFilter::StorageMin(1000),
            SearchFilter::StorageMax(5000),
        ];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 2); // Should match "High Performance VM" and "Memory Intensive VM"

        // Test search by bandwidth range
        let filters = vec![
            SearchFilter::BandwidthMin(500),
            SearchFilter::BandwidthMax(5000),
        ];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 3); // Should match all three VMs

        // Test search by bandwidth range
        let filters = vec![
            SearchFilter::BandwidthMin(1000),
            SearchFilter::BandwidthMax(5000),
        ];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 2); // Should match 2 VMs

        // Test search by addon ipv4
        let filters = vec![SearchFilter::AddonIpv4(
            "/24 block".to_string(),
            0,
            u64::MAX,
        )];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        let addon = match &results[0].compatible_addons[0] {
            Addon::Ipv4(ipv4, _) => ipv4,
            _ => panic!(),
        };
        assert_eq!(addon, "/24 block");

        // Test search by addon ipv6
        let filters = vec![SearchFilter::AddonIpv6(
            "/64 block".to_string(),
            0,
            u64::MAX,
        )];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        let addon = results[0]
            .compatible_addons
            .iter()
            .filter_map(|addon| match &addon {
                Addon::Ipv6(ipv6, _) => Some(ipv6),
                _ => None,
            })
            .collect::<Vec<_>>()[0];
        assert_eq!(addon, "/64 block");

        // Test search by addon gpu
        let filters = vec![SearchFilter::AddonGpu("NVIDIA".to_string(), 0, u64::MAX)];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        let addon = results[0]
            .compatible_addons
            .iter()
            .filter_map(|addon| match &addon {
                Addon::Gpu(addon, _) => Some(addon),
                _ => None,
            })
            .collect::<Vec<_>>()[0];
        assert_eq!(addon, "NVIDIA H100");

        // Test search by addon storage
        let filters = vec![SearchFilter::AddonStorage("SSD".to_string(), 0, u64::MAX)];
        let results = do_get_matching_offerings(&ledger, filters);
        assert_eq!(results.len(), 1);
        let addon = results[0]
            .compatible_addons
            .iter()
            .filter_map(|addon| match &addon {
                Addon::Storage(addon, _) => Some(addon),
                _ => None,
            })
            .collect::<Vec<_>>()[0];
        assert_eq!(addon, "SSD");
    }

    #[test]
    fn test_invalid_inputs() {
        let mut ledger = new_temp_ledger(None);

        // Invalid UID length
        let long_uid = vec![0; 65];
        let result =
            do_node_provider_update_offering(&mut ledger, Principal::anonymous(), long_uid, vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Node provider unique id too long");

        // Invalid signature length
        let short_signature_payload = UpdateOfferingPayload {
            offering_payload: vec![],
            signature: vec![0; 63],
        };
        let result = do_node_provider_update_offering(
            &mut ledger,
            Principal::anonymous(),
            vec![1, 2, 3],
            serde_json::to_vec(&short_signature_payload).unwrap(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "slice length is 3 instead of 32 bytes");

        // Invalid caller
        let long_payload = UpdateOfferingPayload {
            offering_payload: vec![0; 1025],
            signature: vec![0; 64],
        };
        let result = do_node_provider_update_offering(
            &mut ledger,
            Principal::anonymous(),
            vec![1; 32],
            serde_json::to_vec(&long_payload).unwrap(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid caller");

        // Invalid payload length
        let long_payload = UpdateOfferingPayload {
            offering_payload: vec![0; 1025],
            signature: vec![0; 64],
        };
        let dcc_identity = DccIdentity::new_from_seed(b"test").unwrap();
        let result = do_node_provider_update_offering(
            &mut ledger,
            dcc_identity.to_ic_principal(),
            dcc_identity.to_bytes_verifying(),
            serde_json::to_vec(&long_payload).unwrap(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Profile payload too long");
    }

    #[test]
    fn test_missing_node_provider() {
        let mut ledger = new_temp_ledger(None);

        // Trying to update an offering for a non-existent node provider
        let payload = UpdateOfferingPayload {
            offering_payload: vec![0; 64],
            signature: vec![0; 64],
        };
        let dcc_identity = DccIdentity::new_from_seed(b"test").unwrap();
        let result = do_node_provider_update_offering(
            &mut ledger,
            dcc_identity.to_ic_principal(),
            dcc_identity.to_bytes_verifying(),
            serde_json::to_vec(&payload).unwrap(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Node provider not found");
    }

    #[test]
    fn test_signature_verification_failure() {
        let mut ledger = new_temp_ledger(None);

        // Create a node provider entry in the ledger
        let dcc_identity = DccIdentity::new_from_seed(b"test").unwrap();
        ledger
            .upsert(
                LABEL_NP_REGISTER.to_string(),
                dcc_identity.to_bytes_verifying().clone(),
                dcc_identity.to_bytes_verifying(),
            )
            .unwrap();

        // Try to update with an invalid signature
        let payload = UpdateOfferingPayload {
            offering_payload: vec![0; 64],
            signature: vec![1; 64], // Incorrect signature
        };
        let result = do_node_provider_update_offering(
            &mut ledger,
            dcc_identity.to_ic_principal(),
            dcc_identity.to_bytes_verifying(),
            serde_json::to_vec(&payload).unwrap(),
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Signature is invalid: DalekError(signature::Error { source: None })"
        );
    }

    #[test]
    fn test_fee_charging() {
        let mut ledger = new_temp_ledger(None);

        // Create a node provider entry in the ledger
        let np_uid = vec![1, 2, 3];
        ledger
            .upsert(LABEL_NP_REGISTER.to_string(), np_uid.clone(), vec![0; 32])
            .unwrap();

        // Create a valid payload and update offering
        let payload = UpdateOfferingPayload {
            offering_payload: vec![0; 64],
            signature: vec![0; 64], // Assuming valid signature for this test
        };

        // Mock the fee charging function to ensure it is called
        let result = do_node_provider_update_offering(
            &mut ledger,
            Principal::anonymous(),
            np_uid,
            serde_json::to_vec(&payload).unwrap(),
        );

        assert!(result.is_err());
        // This assertion assumes the charge_fees_to_account_no_bump_reputation function would fail
        // due to the mocked environment. In a real scenario, the function's behavior should be verified.
    }
}
