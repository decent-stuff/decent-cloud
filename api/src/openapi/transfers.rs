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

#[cfg(test)]
mod tests {
    use crate::database::tokens::TokenTransfer;
    use crate::openapi::common::ApiResponse;

    fn sample_transfer() -> TokenTransfer {
        TokenTransfer {
            from_account: "abc123".to_string(),
            to_account: "def456".to_string(),
            amount_e9s: 1_000_000_000,
            fee_e9s: 10_000,
            memo: Some("test transfer".to_string()),
            created_at_ns: 1_700_000_000_000_000_000,
        }
    }

    #[test]
    fn test_token_transfer_serialization_field_names_and_values() {
        let transfer = sample_transfer();
        let json = serde_json::to_value(&transfer).unwrap();
        assert_eq!(json["from_account"], "abc123");
        assert_eq!(json["to_account"], "def456");
        assert_eq!(json["amount_e9s"], 1_000_000_000_i64);
        assert_eq!(json["fee_e9s"], 10_000);
        assert_eq!(json["memo"], "test transfer");
        assert_eq!(json["created_at_ns"], 1_700_000_000_000_000_000_i64);
    }

    #[test]
    fn test_token_transfer_memo_none_omitted_from_json() {
        let transfer = TokenTransfer {
            memo: None,
            ..sample_transfer()
        };
        let json = serde_json::to_value(&transfer).unwrap();
        assert!(
            json.get("memo").is_none(),
            "None memo should be absent from JSON"
        );
    }

    #[test]
    fn test_token_transfer_roundtrip() {
        let transfer = sample_transfer();
        let serialized = serde_json::to_string(&transfer).unwrap();
        let deserialized: TokenTransfer = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.from_account, transfer.from_account);
        assert_eq!(deserialized.amount_e9s, transfer.amount_e9s);
        assert_eq!(deserialized.memo, transfer.memo);
    }

    #[test]
    fn test_api_response_transfers_success() {
        let transfers = vec![sample_transfer()];
        let resp = ApiResponse {
            success: true,
            data: Some(transfers),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["from_account"], "abc123");
    }

    #[test]
    fn test_api_response_transfers_error() {
        let resp: ApiResponse<Vec<TokenTransfer>> = ApiResponse {
            success: false,
            data: None,
            error: Some("database connection failed".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json.get("data").is_none());
        assert_eq!(json["error"], "database connection failed");
    }

    #[test]
    fn test_api_response_balance_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(5_000_000_000_i64),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"], 5_000_000_000_i64);
    }

    #[test]
    fn test_api_response_balance_error() {
        let resp: ApiResponse<i64> = ApiResponse {
            success: false,
            data: None,
            error: Some("account not found".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "account not found");
    }
}
