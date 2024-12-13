use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
use ledger_map::LedgerMap;
use serde::{Deserialize, Serialize};

use crate::{
    amount_as_string, charge_fees_to_account_and_bump_reputation, contract_sign_fee_e9s, fn_info,
    ContractSignRequestPayload, DccIdentity, LABEL_CONTRACT_SIGN_REPLY,
    LABEL_CONTRACT_SIGN_REQUEST,
};

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ContractSignReplyV1 {
    requester_pubkey_bytes: Vec<u8>, // Public key of the original requester
    request_memo: String,            // Memo field of the original request
    contract_id: Vec<u8>,            // Contract ID of the request that we are replying to
    sign_accepted: bool, // True/False to mark whether the signing was accepted or rejected by the provider
    response_text: String, // Thank you note, or similar on success. Reason the request failed on failure.
    response_details: String, // Instructions or a link to the detailed instructions: describing next steps, further information, etc.
}

// Main struct for Offering Request
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractSignReply {
    V1(ContractSignReplyV1),
}

impl ContractSignReply {
    pub fn contract_id(&self) -> &[u8] {
        match self {
            ContractSignReply::V1(payload) => payload.contract_id.as_slice(),
        }
    }
    pub fn requester_pubkey_bytes(&self) -> &[u8] {
        match self {
            ContractSignReply::V1(payload) => payload.requester_pubkey_bytes.as_slice(),
        }
    }
    pub fn request_memo(&self) -> &String {
        match self {
            ContractSignReply::V1(payload) => &payload.request_memo,
        }
    }
    pub fn sign_accepted(&self) -> bool {
        match self {
            ContractSignReply::V1(payload) => payload.sign_accepted,
        }
    }
    pub fn response_text(&self) -> &String {
        match self {
            ContractSignReply::V1(payload) => &payload.response_text,
        }
    }
    pub fn response_details(&self) -> &String {
        match self {
            ContractSignReply::V1(payload) => &payload.response_details,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ContractSignReplyPayloadV1 {
    payload_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractSignReplyPayload {
    V1(ContractSignReplyPayloadV1),
}

impl ContractSignReplyPayload {
    pub fn new(payload_bytes: Vec<u8>, crypto_signature: Vec<u8>) -> ContractSignReplyPayload {
        ContractSignReplyPayload::V1(ContractSignReplyPayloadV1 {
            payload_bytes,
            crypto_signature,
        })
    }
}

#[named]
pub fn do_contract_sign_reply(
    ledger: &mut LedgerMap,
    pubkey_bytes: Vec<u8>,
    reply_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes).unwrap();
    dcc_id.verify_bytes(&reply_serialized, &crypto_signature)?;

    fn_info!("{}", dcc_id);

    let cs_reply = ContractSignReply::try_from_slice(&reply_serialized).unwrap();
    let cs_req = ledger
        .get(LABEL_CONTRACT_SIGN_REQUEST, cs_reply.contract_id())
        .unwrap();
    let cs_req = ContractSignRequestPayload::try_from_slice(&cs_req).unwrap();
    let cs_req = cs_req
        .deserialize_contract_sign_request()
        .expect("Error deserializing original contract sign request");
    if pubkey_bytes != cs_req.provider_pubkey_bytes() {
        return Err(format!(
            "Contract signing reply signed and submitted by {} does not match the provider public key {} from contract req 0x{}",
            dcc_id, DccIdentity::new_verifying_from_bytes(cs_req.provider_pubkey_bytes()).unwrap(), hex::encode(cs_reply.contract_id())
        ));
    }
    let payload = ContractSignReplyPayload::new(reply_serialized, crypto_signature);
    let payload_bytes = borsh::to_vec(&payload).unwrap();

    let fees = contract_sign_fee_e9s(cs_req.payment_amount());
    charge_fees_to_account_and_bump_reputation(ledger, &dcc_id, &dcc_id, fees)?;

    ledger.upsert(
        LABEL_CONTRACT_SIGN_REPLY,
        &pubkey_bytes,
        payload_bytes,
    )
    .map(|_| {
        format!(
            "Contract signing reply submitted! Thank you. You have been charged {} tokens as a fee, and your reputation has been bumped accordingly",
            amount_as_string(fees)
        )
    })
    .map_err(|e| e.to_string())
}