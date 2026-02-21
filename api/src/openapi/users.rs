use super::common::ApiResponse;
use super::providers::BandwidthHistoryResponse;
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct UsersApi;

#[OpenApi]
impl UsersApi {
    /// Get user activity
    ///
    /// Returns activity summary for a specific user (blockchain-based).
    /// Requires authentication - user can only access their own activity.
    #[oai(
        path = "/users/:pubkey/activity",
        method = "get",
        tag = "super::common::ApiTags::Users"
    )]
    async fn get_user_activity(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::users::UserActivity>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        // Authorization: user can only access their own activity
        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own activity".to_string()),
            });
        }

        match db.get_user_activity(&pubkey_bytes).await {
            Ok(activity) => Json(ApiResponse {
                success: true,
                data: Some(activity),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get bandwidth history for a contract (tenant/user view)
    ///
    /// Returns bandwidth history records for a contract the authenticated user owns as requester.
    /// Requires the requesting user to be the contract's requester.
    #[oai(
        path = "/users/:pubkey/contracts/:contract_id/bandwidth",
        method = "get",
        tag = "super::common::ApiTags::Users"
    )]
    async fn get_user_contract_bandwidth(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<Vec<BandwidthHistoryResponse>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        // Authorization: user can only access their own data
        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own contracts".to_string()),
            });
        }

        // Verify the contract belongs to this requester
        let requester_hex = match db.get_contract_requester_hex(&contract_id.0).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        if requester_hex != pubkey.0 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: contract does not belong to you".to_string()),
            });
        }

        match db.get_bandwidth_history(&contract_id.0, 100).await {
            Ok(records) => {
                let response: Vec<BandwidthHistoryResponse> = records
                    .into_iter()
                    .map(|r| BandwidthHistoryResponse {
                        bytes_in: r.bytes_in as u64,
                        bytes_out: r.bytes_out as u64,
                        recorded_at_ns: r.recorded_at_ns,
                    })
                    .collect();
                Json(ApiResponse {
                    success: true,
                    data: Some(response),
                    error: None,
                })
            }
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
    use crate::database::test_helpers::setup_test_db;
    use crate::database::users::UserActivity;
    use crate::openapi::common::ApiResponse;
    use crate::openapi::providers::BandwidthHistoryResponse;

    fn empty_activity() -> UserActivity {
        UserActivity {
            offerings_provided: vec![],
            rentals_as_requester: vec![],
            rentals_as_provider: vec![],
        }
    }

    #[test]
    fn test_user_activity_serialization_field_names() {
        let activity = empty_activity();
        let json = serde_json::to_value(&activity).unwrap();
        // UserActivity has no rename_all, so field names are snake_case
        assert!(json.get("offerings_provided").is_some());
        assert!(json.get("rentals_as_requester").is_some());
        assert!(json.get("rentals_as_provider").is_some());
    }

    #[test]
    fn test_user_activity_empty_arrays() {
        let activity = empty_activity();
        let json = serde_json::to_value(&activity).unwrap();
        assert_eq!(json["offerings_provided"].as_array().unwrap().len(), 0);
        assert_eq!(json["rentals_as_requester"].as_array().unwrap().len(), 0);
        assert_eq!(json["rentals_as_provider"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_api_response_user_activity_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(empty_activity()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json["data"].is_object(), "data should be a UserActivity object");
        assert!(json["data"]["offerings_provided"].is_array());
    }

    #[test]
    fn test_api_response_user_activity_error() {
        let resp: ApiResponse<UserActivity> = ApiResponse {
            success: false,
            data: None,
            error: Some("Unauthorized: can only access your own activity".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "Unauthorized: can only access your own activity");
    }

    #[test]
    fn test_api_response_user_activity_invalid_pubkey_error() {
        let resp: ApiResponse<UserActivity> = ApiResponse {
            success: false,
            data: None,
            error: Some("Invalid pubkey format".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Invalid pubkey format");
    }

    // ── get_user_contract_bandwidth ownership verification ───────────────────

    #[tokio::test]
    async fn test_user_bandwidth_returns_history_for_requester() {
        let db = setup_test_db().await;
        let contract_id = vec![0x11u8; 32];
        let requester_pk = vec![0x22u8; 32];
        let provider_pk = vec![0x33u8; 32];
        let contract_id_hex = hex::encode(&contract_id);
        let requester_hex = hex::encode(&requester_pk);

        // Insert contract
        sqlx::query!(
            r#"INSERT INTO contract_sign_requests
               (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns,
                payment_method, payment_status, currency)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
            contract_id.as_slice(),
            requester_pk.as_slice(),
            "ssh-ed25519 AAAA",
            "email:user@example.com",
            provider_pk.as_slice(),
            "offer-1",
            1_000_000_000i64,
            "test",
            0i64,
            "stripe",
            "succeeded",
            "USD"
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Record bandwidth
        db.record_bandwidth(&contract_id_hex, "slug1", &requester_hex, 1000, 2000)
            .await
            .unwrap();

        // Verify ownership check: requester matches
        let requester_result = db
            .get_contract_requester_hex(&contract_id_hex)
            .await
            .unwrap();
        assert_eq!(requester_result, Some(requester_hex.clone()));

        // Fetch bandwidth and confirm data is present
        let history = db.get_bandwidth_history(&contract_id_hex, 100).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].bytes_in, 1000);
        assert_eq!(history[0].bytes_out, 2000);
    }

    #[tokio::test]
    async fn test_user_bandwidth_rejects_non_requester() {
        let db = setup_test_db().await;
        let contract_id = vec![0x44u8; 32];
        let requester_pk = vec![0x55u8; 32];
        let other_pk = vec![0x66u8; 32];
        let provider_pk = vec![0x77u8; 32];
        let contract_id_hex = hex::encode(&contract_id);
        let other_hex = hex::encode(&other_pk);

        // Insert contract with requester_pk
        sqlx::query!(
            r#"INSERT INTO contract_sign_requests
               (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns,
                payment_method, payment_status, currency)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
            contract_id.as_slice(),
            requester_pk.as_slice(),
            "ssh-ed25519 AAAA",
            "email:user@example.com",
            provider_pk.as_slice(),
            "offer-1",
            1_000_000_000i64,
            "test",
            0i64,
            "stripe",
            "succeeded",
            "USD"
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Ownership check: other_pk is NOT the requester
        let requester_result = db
            .get_contract_requester_hex(&contract_id_hex)
            .await
            .unwrap();
        assert_ne!(requester_result, Some(other_hex));
    }

    #[test]
    fn test_bandwidth_history_response_serializes_camelcase() {
        let resp = BandwidthHistoryResponse {
            bytes_in: 1024,
            bytes_out: 512,
            recorded_at_ns: 1_700_000_000_000_000_000,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["bytesIn"], 1024_u64);
        assert_eq!(json["bytesOut"], 512_u64);
        assert_eq!(json["recordedAtNs"], 1_700_000_000_000_000_000_i64);
    }

    #[test]
    fn test_api_response_bandwidth_success_wraps_correctly() {
        let records = vec![BandwidthHistoryResponse {
            bytes_in: 2048,
            bytes_out: 4096,
            recorded_at_ns: 9_000_000,
        }];
        let resp = ApiResponse {
            success: true,
            data: Some(records),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json["data"].is_array());
        assert_eq!(json["data"][0]["bytesIn"], 2048_u64);
    }
}
