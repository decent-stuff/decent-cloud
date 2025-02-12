#[allow(unused_imports)]
use ic_cdk::println;
// Standard description: https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md
// Reference implementation: https://github.com/dfinity/ic/blob/master/rs/rosetta-api/icrc1/ledger/src/main.rs

use crate::canister_backend::pre_icrc3;
use icrc_ledger_types::icrc3::blocks::DataCertificate as DataCertificatePreIcrc3;
use icrc_ledger_types::icrc3::transactions::{GetTransactionsRequest, GetTransactionsResponse};

#[ic_cdk::query]
async fn get_transactions(req: GetTransactionsRequest) -> GetTransactionsResponse {
    pre_icrc3::_get_transactions(req).await
}

// #[ic_cdk::query]
// fn get_blocks(req: GetBlocksRequest) -> GetBlocksResponse {
//     pre_icrc3::_get_blocks(req)
// }

#[ic_cdk::query]
fn get_data_certificate() -> DataCertificatePreIcrc3 {
    pre_icrc3::_get_data_certificate()
}
