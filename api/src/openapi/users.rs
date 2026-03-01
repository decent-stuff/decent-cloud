use super::common::{ApiResponse, MarkReadRequest, UnreadCountResponse, UserNotificationResponse};
use super::providers::BandwidthHistoryResponse;
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request body to create an API token
#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreateApiTokenRequest {
    pub name: String,
    /// Expiry in days. None = never expires.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub expires_in_days: Option<i64>,
}

/// Response for a newly created token (includes raw token value — shown once).
#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreatedApiTokenResponse {
    pub id: String,
    pub name: String,
    /// The raw token value — store it securely, it will not be shown again.
    pub token: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub expires_at: Option<i64>,
}

/// Token summary for listing (no raw token value).
#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ApiTokenSummary {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub last_used_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub expires_at: Option<i64>,
    pub is_active: bool,
}

/// Decode pubkey hex and verify it matches the authenticated user.
/// Returns an error string on failure.
fn decode_and_verify_pubkey(pubkey_hex: &str, auth_pubkey: &[u8]) -> Result<Vec<u8>, String> {
    let pubkey_bytes = hex::decode(pubkey_hex).map_err(|_| "Invalid pubkey format".to_string())?;
    if auth_pubkey != pubkey_bytes.as_slice() {
        return Err("Unauthorized: can only access your own data".to_string());
    }
    Ok(pubkey_bytes)
}

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

    /// Save an offering to watchlist
    ///
    /// Add an offering to the authenticated user's personal watchlist.
    /// Idempotent: saving an already-saved offering succeeds silently.
    #[oai(
        path = "/users/:pubkey/saved-offerings/:offering_id",
        method = "post",
        tag = "super::common::ApiTags::Users"
    )]
    async fn save_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        offering_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };
        match db.save_offering(&pubkey_bytes, offering_id.0).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: None,
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove an offering from watchlist
    ///
    /// Remove an offering from the authenticated user's personal watchlist.
    /// Idempotent: unsaving a non-saved offering succeeds silently.
    #[oai(
        path = "/users/:pubkey/saved-offerings/:offering_id",
        method = "delete",
        tag = "super::common::ApiTags::Users"
    )]
    async fn unsave_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        offering_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };
        match db.unsave_offering(&pubkey_bytes, offering_id.0).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: None,
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get saved offerings
    ///
    /// Returns all offerings saved to the authenticated user's watchlist, most-recently saved first.
    #[oai(
        path = "/users/:pubkey/saved-offerings",
        method = "get",
        tag = "super::common::ApiTags::Users"
    )]
    async fn get_saved_offerings(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::offerings::Offering>>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };
        match db.get_saved_offerings(&pubkey_bytes).await {
            Ok(offerings) => Json(ApiResponse {
                success: true,
                data: Some(offerings),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get saved offering IDs
    ///
    /// Returns the IDs of all offerings saved by the authenticated user (for bulk UI highlighting).
    #[oai(
        path = "/users/:pubkey/saved-offering-ids",
        method = "get",
        tag = "super::common::ApiTags::Users"
    )]
    async fn get_saved_offering_ids(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<i64>>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };
        match db.get_saved_offering_ids(&pubkey_bytes).await {
            Ok(ids) => Json(ApiResponse {
                success: true,
                data: Some(ids),
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

    /// Get user notifications
    ///
    /// Returns the last 50 notifications for the authenticated user, newest first.
    #[oai(
        path = "/users/:pubkey/notifications",
        method = "get",
        tag = "super::common::ApiTags::Users"
    )]
    async fn get_user_notifications(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<UserNotificationResponse>>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db.get_user_notifications(&pubkey_bytes, 50).await {
            Ok(notifications) => {
                let response = notifications
                    .into_iter()
                    .map(|n| UserNotificationResponse {
                        id: n.id,
                        notification_type: n.notification_type,
                        title: n.title,
                        body: n.body,
                        contract_id: n.contract_id,
                        read_at: n.read_at,
                        created_at: n.created_at,
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

    /// Get unread notification count
    ///
    /// Returns the number of unread notifications for the authenticated user.
    #[oai(
        path = "/users/:pubkey/notifications/unread-count",
        method = "get",
        tag = "super::common::ApiTags::Users"
    )]
    async fn get_unread_count(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<UnreadCountResponse>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db.get_unread_count(&pubkey_bytes).await {
            Ok(count) => Json(ApiResponse {
                success: true,
                data: Some(UnreadCountResponse {
                    unread_count: count,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Mark notifications as read
    ///
    /// Marks the specified notification IDs as read.
    /// If the `ids` array is empty, all notifications for the user are marked as read.
    #[oai(
        path = "/users/:pubkey/notifications/mark-read",
        method = "post",
        tag = "super::common::ApiTags::Users"
    )]
    async fn mark_notifications_read(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        body: Json<MarkReadRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        let result = if body.ids.is_empty() {
            db.mark_all_notifications_read(&pubkey_bytes).await
        } else {
            db.mark_notifications_read(&body.ids, &pubkey_bytes).await
        };

        match result {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: None,
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create API token
    ///
    /// Generates a new long-lived API token for programmatic access.
    /// The raw token is returned once and must be stored securely.
    #[oai(
        path = "/users/:pubkey/api-tokens",
        method = "post",
        tag = "super::common::ApiTags::Users"
    )]
    async fn create_api_token(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        body: Json<CreateApiTokenRequest>,
    ) -> Json<ApiResponse<CreatedApiTokenResponse>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db
            .create_api_token(&pubkey_bytes, &body.0.name, body.0.expires_in_days)
            .await
        {
            Ok((token, raw_hex)) => Json(ApiResponse {
                success: true,
                data: Some(CreatedApiTokenResponse {
                    id: token.id.to_string(),
                    name: token.name,
                    token: raw_hex,
                    created_at: token.created_at,
                    expires_at: token.expires_at,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List API tokens
    ///
    /// Returns all API tokens for the authenticated user (raw token values are not returned).
    #[oai(
        path = "/users/:pubkey/api-tokens",
        method = "get",
        tag = "super::common::ApiTags::Users"
    )]
    async fn list_api_tokens(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<ApiTokenSummary>>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db.list_api_tokens(&pubkey_bytes).await {
            Ok(tokens) => Json(ApiResponse {
                success: true,
                data: Some(
                    tokens
                        .into_iter()
                        .map(|t| {
                            let is_active = t.is_active().unwrap_or(false);
                            ApiTokenSummary {
                                id: t.id.to_string(),
                                name: t.name,
                                created_at: t.created_at,
                                last_used_at: t.last_used_at,
                                expires_at: t.expires_at,
                                is_active,
                            }
                        })
                        .collect(),
                ),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Revoke API token
    ///
    /// Revokes an API token by setting its revoked_at timestamp.
    /// Only the token owner can revoke their own tokens.
    #[oai(
        path = "/users/:pubkey/api-tokens/:token_id",
        method = "delete",
        tag = "super::common::ApiTags::Users"
    )]
    async fn revoke_api_token(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        token_id: Path<String>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_and_verify_pubkey(&pubkey.0, &auth.pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        let token_uuid = match uuid::Uuid::parse_str(&token_id.0) {
            Ok(u) => u,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid token ID format".to_string()),
                })
            }
        };

        match db.revoke_api_token(token_uuid, &pubkey_bytes).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: None,
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
        assert!(
            json["data"].is_object(),
            "data should be a UserActivity object"
        );
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
        assert!(json.get("data").is_none());
        assert_eq!(
            json["error"],
            "Unauthorized: can only access your own activity"
        );
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
        let history = db
            .get_bandwidth_history(&contract_id_hex, 100)
            .await
            .unwrap();
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
