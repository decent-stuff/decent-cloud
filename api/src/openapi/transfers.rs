use super::common::{default_limit, ApiResponse, ApiTags};
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct TransfersApi;

#[OpenApi]
impl TransfersApi {
    /// Get recent transfers
    ///
    /// Returns the most recent token transfers
    #[oai(path = "/transfers", method = "get", tag = "ApiTags::Transfers")]
    async fn get_recent_transfers(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>> {
        match db.get_recent_transfers(limit.0).await {
            Ok(transfers) => Json(ApiResponse {
                success: true,
                data: Some(transfers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get account transfers
    ///
    /// Returns transfers for a specific account
    #[oai(
        path = "/accounts/:account/transfers",
        method = "get",
        tag = "ApiTags::Transfers"
    )]
    async fn get_account_transfers(
        &self,
        db: Data<&Arc<Database>>,
        account: Path<String>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>> {
        match db.get_account_transfers(&account.0, limit.0).await {
            Ok(transfers) => Json(ApiResponse {
                success: true,
                data: Some(transfers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get account balance
    ///
    /// Returns the balance for a specific account
    #[oai(
        path = "/accounts/:account/balance",
        method = "get",
        tag = "ApiTags::Transfers"
    )]
    async fn get_account_balance(
        &self,
        db: Data<&Arc<Database>>,
        account: Path<String>,
    ) -> Json<ApiResponse<i64>> {
        match db.get_account_balance(&account.0).await {
            Ok(balance) => Json(ApiResponse {
                success: true,
                data: Some(balance),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }
}
