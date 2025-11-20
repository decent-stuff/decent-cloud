use super::common::{check_authorization, decode_pubkey, ApiResponse, ApiTags};
use crate::{auth::ApiAuthenticatedUser, database::Database};
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::Deserialize;
use std::sync::Arc;

// Request types for user operations
#[derive(Debug, Deserialize, Object)]
pub struct UpdateUserProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpsertContactRequest {
    pub contact_type: String,
    pub contact_value: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpsertSocialRequest {
    pub platform: String,
    pub username: String,
    pub profile_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct AddPublicKeyRequest {
    pub key_type: String,
    pub key_data: String,
    pub key_fingerprint: Option<String>,
    pub label: Option<String>,
}

pub struct UsersApi;

#[OpenApi]
impl UsersApi {
    /// Get user profile
    ///
    /// Returns profile information for a specific user
    #[oai(
        path = "/users/:pubkey/profile",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_profile(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::users::UserProfile>> {
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

        match db.get_user_profile(&pubkey_bytes).await {
            Ok(Some(profile)) => Json(ApiResponse {
                success: true,
                data: Some(profile),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("User not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user contacts
    ///
    /// Returns contact information for a specific user
    #[oai(
        path = "/users/:pubkey/contacts",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_contacts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserContact>>> {
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

        match db.get_user_contacts(&pubkey_bytes).await {
            Ok(contacts) => Json(ApiResponse {
                success: true,
                data: Some(contacts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user socials
    ///
    /// Returns social media profiles for a specific user
    #[oai(
        path = "/users/:pubkey/socials",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_socials(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserSocial>>> {
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

        match db.get_user_socials(&pubkey_bytes).await {
            Ok(socials) => Json(ApiResponse {
                success: true,
                data: Some(socials),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user public keys
    ///
    /// Returns public keys for a specific user
    #[oai(path = "/users/:pubkey/keys", method = "get", tag = "ApiTags::Users")]
    async fn get_user_public_keys(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserPublicKey>>> {
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

        match db.get_user_public_keys(&pubkey_bytes).await {
            Ok(keys) => Json(ApiResponse {
                success: true,
                data: Some(keys),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user activity
    ///
    /// Returns activity summary for a specific user
    #[oai(
        path = "/users/:pubkey/activity",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_activity(
        &self,
        db: Data<&Arc<Database>>,
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

    /// Update user profile
    ///
    /// Updates user profile information (requires authentication)
    #[oai(
        path = "/users/:pubkey/profile",
        method = "put",
        tag = "ApiTags::Users"
    )]
    async fn update_user_profile(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpdateUserProfileRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .upsert_user_profile(
                &auth.pubkey,
                req.display_name.as_deref(),
                req.bio.as_deref(),
                req.avatar_url.as_deref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Profile updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Upsert user contact
    ///
    /// Adds or updates a user contact (requires authentication)
    #[oai(
        path = "/users/:pubkey/contacts",
        method = "post",
        tag = "ApiTags::Users"
    )]
    async fn upsert_user_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpsertContactRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = crate::validation::validate_contact_type(&req.contact_type) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        if let Err(e) =
            crate::validation::validate_contact_value(&req.contact_type, &req.contact_value)
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        match db
            .upsert_user_contact(
                &auth.pubkey,
                &req.contact_type,
                &req.contact_value,
                req.verified,
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Contact added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete user contact
    ///
    /// Deletes a user contact (requires authentication)
    #[oai(
        path = "/users/:pubkey/contacts/:contact_id",
        method = "delete",
        tag = "ApiTags::Users"
    )]
    async fn delete_user_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contact_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_user_contact(&auth.pubkey, contact_id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Contact deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Upsert user social
    ///
    /// Adds or updates a user social profile (requires authentication)
    #[oai(
        path = "/users/:pubkey/socials",
        method = "post",
        tag = "ApiTags::Users"
    )]
    async fn upsert_user_social(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpsertSocialRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = crate::validation::validate_social_platform(&req.platform) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        if let Err(e) = crate::validation::validate_social_username(&req.username) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        if let Some(ref url) = req.profile_url {
            if let Err(e) = crate::validation::validate_url(url) {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        }

        match db
            .upsert_user_social(
                &auth.pubkey,
                &req.platform,
                &req.username,
                req.profile_url.as_deref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Social account added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete user social
    ///
    /// Deletes a user social profile (requires authentication)
    #[oai(
        path = "/users/:pubkey/socials/:social_id",
        method = "delete",
        tag = "ApiTags::Users"
    )]
    async fn delete_user_social(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        social_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_user_social(&auth.pubkey, social_id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Social account deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Add user public key
    ///
    /// Adds a new public key for a user (requires authentication)
    #[oai(path = "/users/:pubkey/keys", method = "post", tag = "ApiTags::Users")]
    async fn add_user_public_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<AddPublicKeyRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = crate::validation::validate_public_key(&req.key_type, &req.key_data) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        match db
            .add_user_public_key(
                &auth.pubkey,
                &req.key_type,
                &req.key_data,
                req.key_fingerprint.as_deref(),
                req.label.as_deref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Public key added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete user public key
    ///
    /// Deletes a user public key (requires authentication)
    #[oai(
        path = "/users/:pubkey/keys/:key_id",
        method = "delete",
        tag = "ApiTags::Users"
    )]
    async fn delete_user_public_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        key_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_user_public_key(&auth.pubkey, key_id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Public key deleted successfully".to_string()),
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
