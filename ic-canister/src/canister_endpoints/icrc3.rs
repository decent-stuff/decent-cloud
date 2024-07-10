#[allow(unused_imports)]
use ic_cdk::println;
// Standard description: https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md
// Reference implementation: https://github.com/dfinity/ic/blob/master/rs/rosetta-api/icrc1/ledger/src/main.rs

use crate::canister_backend::{icrc3, pre_icrc3};

/// ICRC-3 MUST: Endpoint for listing all the canisters and their contained blocks.
#[ic_cdk::query]
fn icrc3_get_archives(
    args: Icrc3RequestArgsHistoryForPrincipal,
) -> Icrc3ResponseHistoryForPrincipal {
    icrc3::_icrc3_get_archives(args)
}

/// ICRC-3 MUST:
/// - certify the last block (tip) recorded and
/// - allow to download the certificate via the icrc3_get_tip_certificate endpoint.
/// The certificate follows the IC Specification for Certificates.
/// The certificate is comprised of a tree containing the certified data and the signature.
/// The tree MUST contain two labelled values (leafs):
/// 1. last_block_index: the index of the last block in the chain. The values must be expressed as leb128
/// 2. last_block_hash: the hash of the last block in the chain
#[ic_cdk::query]
fn icrc3_get_tip_certificate() -> Option<icrc3::block_types::ICRC3DataCertificate> {
    icrc3::_icrc3_get_tip_certificate()
}

/// ICRC-3 MUST:
/// Servers must serve the block log as a list of Value where each Value represent a single block in the block log.
/// An ICRC-3 compliant Block
///  1. MUST be a Value of variant Map
///  2. MUST contain a field phash: Blob which is the hash of its parent if it has a parent block
///  3. SHOULD contain a field btype: String which uniquely describes the type of the Block.
///     If this field is not set then the block type falls back to ICRC-1 and ICRC-2 for backward
///     compatibility purposes
#[ic_cdk::query]
fn icrc3_get_blocks(args: Vec<GetBlocksRequest>) -> GetBlocksResult {
    _icrc3_get_blocks(args)
}

/// ICRC-3 MUST: List all the supported block types via the endpoint icrc3_supported_block_types.
/// The Ledger MUST return only blocks with btype set to one of the values returned by this endpoint.
#[ic_cdk::query]
fn icrc3_supported_block_types() -> Vec<icrc_ledger_types::icrc3::blocks::SupportedBlockType> {
    _icrc3_supported_block_types()
}
