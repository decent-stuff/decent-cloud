use super::common::ApiResponse;
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
}

#[cfg(test)]
mod tests {
    use crate::database::users::UserActivity;
    use crate::openapi::common::ApiResponse;

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
}
