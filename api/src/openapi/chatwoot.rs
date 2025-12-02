use super::common::ApiResponse;
use crate::auth::ApiAuthenticatedUser;
use crate::chatwoot::generate_identity_hash;
use poem_openapi::{payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ChatwootIdentityResponse {
    /// User identifier for Chatwoot (hex-encoded pubkey)
    pub identifier: String,
    /// HMAC hash for identity validation
    pub identifier_hash: String,
}

pub struct ChatwootApi;

#[OpenApi]
impl ChatwootApi {
    /// Get Chatwoot identity hash
    ///
    /// Returns the identifier and HMAC hash for Chatwoot widget authentication.
    /// Used by the frontend to authenticate users in the Chatwoot widget.
    #[oai(
        path = "/chatwoot/identity",
        method = "get",
        tag = "super::common::ApiTags::Chatwoot"
    )]
    async fn get_identity(
        &self,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<ChatwootIdentityResponse>> {
        let hmac_secret = match std::env::var("CHATWOOT_HMAC_SECRET") {
            Ok(secret) => secret,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Chatwoot not configured".to_string()),
                })
            }
        };

        let identifier = hex::encode(&user.pubkey);
        let identifier_hash = generate_identity_hash(&identifier, &hmac_secret);

        Json(ApiResponse {
            success: true,
            data: Some(ChatwootIdentityResponse {
                identifier,
                identifier_hash,
            }),
            error: None,
        })
    }
}
