use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, info, reward_e9s_per_block, warn,
    Balance, DccIdentity, ED25519_SIGNATURE_LENGTH, LABEL_NP_OFFERING, MAX_NP_OFFERING_BYTES,
    MAX_PUBKEY_BYTES,
};

pub struct OfferingRequestPayloadV1 {
    requester_pubkey_bytes: Vec<u8>,
    requester_ssh_pubkey: String,
    requester_contact: String,
    provider_pubkey_bytes: Vec<u8>,
    offering_id: String,
    instance_id: Option<String>,
    instance_config: Option<String>,
    payment_amount: u64,
    rent_period_seconds: u64,
    rent_start_timestamp: Option<u64>,
    request_memo: String,
    signature: Vec<u8>,
}

pub enum OfferingRequestPayload {
    V1(OfferingRequestPayloadV1),
}

impl OfferingRequestPayload {
    fn new(
        requester_pubkey_bytes: Vec<u8>,
        requester_ssh_pubkey: String,
        requester_contact: String,
        provider_pubkey_bytes: Vec<u8>,
        offering_id: String,
        instance_id: Option<String>,
        instance_config: Option<String>,
        payment_amount: u64,
        rent_period_seconds: u64,
        rent_start_timestamp: Option<u64>,
        request_memo: String,
        signature: Vec<u8>,
    ) -> Self {
        OfferingRequestPayload::V1(OfferingRequestPayloadV1 {
            requester_pubkey_bytes,
            requester_ssh_pubkey,
            requester_contact,
            provider_pubkey_bytes,
            offering_id,
            instance_id,
            instance_config,
            payment_amount,
            rent_period_seconds,
            rent_start_timestamp,
            request_memo,
            signature,
        })
    }
}
