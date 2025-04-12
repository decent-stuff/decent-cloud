use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, fn_info, reputation_get,
    reward_e9s_per_block, DccIdentity, TokenAmountE9s, LABEL_NP_PROFILE, MAX_NP_PROFILE_BYTES,
};
use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use np_profile::Profile;
use serde::Serialize;

pub fn np_profile_update_fee_e9s() -> TokenAmountE9s {
    reward_e9s_per_block() / 1000
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub struct UpdateProfilePayloadV1 {
    pub profile_bytes: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub enum UpdateProfilePayload {
    V1(UpdateProfilePayloadV1),
}

impl UpdateProfilePayload {
    pub fn new(
        update_profile_payload: &[u8],
        crypto_signature_bytes: &[u8],
    ) -> Result<Self, String> {
        if update_profile_payload.len() > MAX_NP_PROFILE_BYTES {
            return Err("Profile payload too long".to_string());
        }
        Ok(UpdateProfilePayload::V1(UpdateProfilePayloadV1 {
            profile_bytes: update_profile_payload.to_vec(),
            signature: crypto_signature_bytes.to_vec(),
        }))
    }

    pub fn deserialize_unchecked(data: &[u8]) -> Result<UpdateProfilePayload, String> {
        UpdateProfilePayload::try_from_slice(data).map_err(|e| e.to_string())
    }

    pub fn deserialize_update_profile(&self) -> Result<Profile, String> {
        match self {
            UpdateProfilePayload::V1(payload) => Profile::try_from_slice(&payload.profile_bytes)
                .map(|v| v.compute_json_value())
                .map_err(|e| e.to_string()),
        }
    }
}

#[named]
pub fn do_node_provider_update_profile(
    ledger: &mut LedgerMap,
    pubkey_bytes: Vec<u8>,
    profile_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes).unwrap();
    dcc_id.verify_bytes(&profile_serialized, &crypto_signature)?;
    fn_info!("{} => {} bytes", dcc_id, profile_serialized.len());

    let payload = UpdateProfilePayload::new(&profile_serialized, &crypto_signature)?;
    let payload_bytes = borsh::to_vec(&payload).unwrap();

    let fees = np_profile_update_fee_e9s();
    charge_fees_to_account_no_bump_reputation(ledger, &dcc_id.as_icrc_compatible_account(), fees, "update-profile")?;

    // Store the original signed payload in the ledger, to enable future verification
    ledger
        .upsert(LABEL_NP_PROFILE, &pubkey_bytes, payload_bytes)
        .map(|_| {
            format!(
                "Profile updated! Thank you. You have been charged {} tokens",
                amount_as_string(fees),
            )
        })
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct NodeProviderProfileWithReputation {
    pub profile: np_profile::Profile,
    pub reputation: u64,
}

pub fn do_node_provider_get_profile(
    ledger: &LedgerMap,
    pubkey_bytes: Vec<u8>,
) -> Option<NodeProviderProfileWithReputation> {
    match ledger.get(LABEL_NP_PROFILE, &pubkey_bytes) {
        // Don't check the signature to save time
        Ok(data) => UpdateProfilePayload::deserialize_unchecked(&data)
            .ok()?
            .deserialize_update_profile()
            .ok()
            .map(|profile| NodeProviderProfileWithReputation {
                profile,
                reputation: reputation_get(&pubkey_bytes),
            }),
        Err(_) => None,
    }
}
