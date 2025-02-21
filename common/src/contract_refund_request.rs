use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
use ic_cdk::println;
use ledger_map::LedgerMap;
use serde::{Deserialize, Serialize};

use crate::{fn_info, DccIdentity};

// TODO

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractRefundRequest {
    V1(ContractRefundRequestV1),
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ContractRefundRequestV1 {
    /// The bytes of the public key of the requester, as a vec of u8.
    /// This is used to identify the requester and to verify the signature.
    pub requester_pubkey_bytes: Vec<u8>,
    /// The instance id for which the refund is requested.
    pub instance_id: String,
    /// The signature of the whole refund request.
    /// This is used to verify that the refund request was made by the requester.
    pub crypto_sig: Vec<u8>,
}

#[named]
pub fn do_contract_refund_request(
    _ledger_map: &LedgerMap,
    pubkey_bytes: Vec<u8>,
    payload_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes).unwrap();
    dcc_id.verify_bytes(&payload_serialized, &crypto_signature)?;

    fn_info!("{}", dcc_id);

    todo!()
}
