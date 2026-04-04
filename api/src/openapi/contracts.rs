use super::common::{
    default_limit, ApiResponse, ApiTags, CancelContractRequest, ExtendContractRequest,
    ExtendContractResponse, RecordUsageRequest, RentalRequestResponse, RotateSshKeyRequest,
    SetAutoRenewRequest, UpdateIcpayTransactionRequest, VerifyCheckoutSessionRequest,
    VerifyCheckoutSessionResponse,
};
use crate::auth::{AdminAuthenticatedUser, ApiAuthenticatedUser};
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct ContractsApi;

/// Check whether the requester has exceeded their spending alert threshold and send a
/// notification if needed. Only fires once per day to avoid spam.
///
/// This is best-effort: any error is logged but does NOT affect contract creation.
pub async fn check_spending_alert_and_notify(db: &Database, requester_pubkey: &[u8]) {
    let pubkey_hex = hex::encode(requester_pubkey);

    let alert = match db.get_spending_alert(&pubkey_hex).await {
        Ok(Some(a)) => a,
        Ok(None) => return, // No alert configured
        Err(e) => {
            tracing::warn!("Failed to fetch spending alert for {}: {:#}", pubkey_hex, e);
            return;
        }
    };

    // Throttle: only notify once per day
    let now_secs = chrono::Utc::now().timestamp();
    if let Some(last) = alert.last_notified_at {
        if now_secs - last < 86_400 {
            return;
        }
    }

    let spending_usd = match db.get_current_month_spending_usd(requester_pubkey).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "Failed to fetch monthly spending for {}: {:#}",
                pubkey_hex,
                e
            );
            return;
        }
    };

    let limit = alert.monthly_limit_usd;
    let threshold_usd = limit * alert.alert_at_pct as f64 / 100.0;

    let (title, body) = if spending_usd >= limit {
        (
            "Budget cap reached".to_string(),
            format!(
                "Your monthly spending (${:.2}) has reached your ${:.2} budget limit.",
                spending_usd, limit
            ),
        )
    } else if spending_usd >= threshold_usd {
        (
            "Spending alert".to_string(),
            format!(
                "You've reached {}% of your monthly budget (${:.2} of ${:.2}).",
                alert.alert_at_pct, spending_usd, limit
            ),
        )
    } else {
        return; // Below threshold
    };

    if let Err(e) = db
        .insert_user_notification(requester_pubkey, "spending_alert", &title, &body, None)
        .await
    {
        tracing::warn!(
            "Failed to insert spending alert notification for {}: {:#}",
            pubkey_hex,
            e
        );
        return;
    }

    if let Err(e) = db.touch_spending_alert_notified_at(&pubkey_hex).await {
        tracing::warn!(
            "Failed to update last_notified_at for spending alert {}: {:#}",
            pubkey_hex,
            e
        );
    }
}

/// Helper function to create Stripe checkout session and update contract
async fn create_stripe_checkout_session(
    db: &Database,
    contract_id: &[u8],
    currency: &str,
    product_name: &str,
) -> Result<String, String> {
    let contract = db
        .get_contract(contract_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Contract created but not found".to_string())?;

    // Validate currency is supported by Stripe
    if !dcc_common::is_stripe_supported_currency(currency) {
        return Err(format!(
            "Currency '{}' is not supported by Stripe",
            currency
        ));
    }

    let stripe_client = crate::stripe_client::StripeClient::new().map_err(|e| e.to_string())?;

    // Convert e9s to cents (divide by 10^7)
    let amount_cents = contract.payment_amount_e9s / 10_000_000;
    let contract_id_hex = hex::encode(contract_id);
    let checkout_url = stripe_client
        .create_checkout_session(
            amount_cents,
            &currency.to_lowercase(),
            product_name,
            &contract_id_hex,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(checkout_url)
}

#[OpenApi]
impl ContractsApi {
    /// List contracts (admin only)
    ///
    /// Returns a paginated list of all contracts. Requires admin authentication.
    #[oai(path = "/contracts", method = "get", tag = "ApiTags::Admin")]
    async fn list_contracts(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        match db.list_contracts(limit.0, offset.0).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract by ID
    ///
    /// Returns details of a specific contract. User must be the requester or provider.
    #[oai(path = "/contracts/:id", method = "get", tag = "ApiTags::Contracts")]
    async fn get_contract(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::Contract>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => {
                // Authorization: user must be requester or provider
                let user_pubkey = hex::encode(&auth.pubkey);
                if contract.requester_pubkey != user_pubkey
                    && contract.provider_pubkey != user_pubkey
                {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Unauthorized: you are not a party to this contract".into()),
                    });
                }
                Json(ApiResponse {
                    success: true,
                    data: Some(contract),
                    error: None,
                })
            }
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Contract not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get encrypted credentials for a contract
    ///
    /// Returns encrypted VM credentials (e.g., root password) for a contract.
    /// Only the contract requester can retrieve credentials.
    /// Credentials are encrypted with the requester's public key and can only
    /// be decrypted with their private key.
    /// Credentials auto-expire 7 days after provisioning.
    #[oai(
        path = "/contracts/:id/credentials",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_credentials(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Get contract to verify authorization
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        // Only the requester can retrieve credentials
        // (Provider should already know the password since they set it up)
        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: only the contract requester can access credentials".to_string(),
                ),
            });
        }

        // Get encrypted credentials
        match db.get_encrypted_credentials(&contract_id).await {
            Ok(Some(credentials)) => Json(ApiResponse {
                success: true,
                data: Some(credentials),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("No credentials available (expired or not set)".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Request password reset for a contract
    ///
    /// Requests a password reset for a provisioned VM. The provider's agent
    /// will pick up the request and execute the reset via SSH.
    /// Only the contract requester can request a reset.
    #[oai(
        path = "/contracts/:id/reset-password",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn request_password_reset(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Get contract to verify authorization and status
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        // Only the requester can request password reset
        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: only the contract requester can request password reset"
                        .to_string(),
                ),
            });
        }

        // Verify contract is in operational status
        let status = contract.status.to_lowercase();
        if status != "provisioned" && status != "active" {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Password reset can only be requested for provisioned or active contracts"
                        .to_string(),
                ),
            });
        }

        match db.request_password_reset(&contract_id).await {
            Ok(_) => {
                if let Ok(Some(full_contract)) = db.get_contract(&contract_id).await {
                    if let Err(e) = crate::rental_notifications::notify_provider_password_reset(
                        db.as_ref(),
                        email_service.as_ref(),
                        &full_contract,
                        false,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to notify provider of password reset for contract {}: {}",
                            hex::encode(&contract_id),
                            e
                        );
                    }
                }
                Json(ApiResponse {
                    success: true,
                    data: Some(
                        "Password reset requested. The provider will reset the password shortly."
                            .to_string(),
                    ),
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

    /// Rotate SSH key for a contract
    ///
    /// Replaces the SSH public key on an active contract. The provider's agent
    /// will inject the new key into the running VM. Requires authentication -
    /// only the contract requester can rotate their SSH key.
    #[oai(
        path = "/contracts/:id/rotate-ssh-key",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn rotate_ssh_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<RotateSshKeyRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Unauthorized: only the contract requester can rotate SSH keys".to_string(),
                ),
            });
        }

        let status = contract.status.to_lowercase();
        if status != "provisioned" && status != "active" {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "SSH key rotation can only be requested for provisioned or active contracts"
                        .to_string(),
                ),
            });
        }

        let new_key = req.0.new_ssh_pubkey.trim().to_string();
        if new_key.is_empty() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("new_ssh_pubkey cannot be empty".to_string()),
            });
        }

        let ssh_key_pattern = regex::Regex::new(
            r"^ssh-(rsa|ed25519|ecdsa|dss)\s+[A-Za-z0-9+/]+={0,3}(\s+.*)?$",
        )
        .unwrap();
        if !ssh_key_pattern.is_match(&new_key) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Invalid SSH public key format. Expected: ssh-(rsa|ed25519|ecdsa|dss) <base64> [comment]"
                        .to_string(),
                ),
            });
        }

        match db
            .request_ssh_key_rotation(&contract_id, &new_key)
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some(
                    "SSH key rotation requested. The provider will inject the new key shortly."
                        .to_string(),
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

    /// Get user contracts
    ///
    /// Returns contracts for a specific user (as requester).
    /// Requires authentication - user can only access their own contracts.
    #[oai(
        path = "/users/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_contracts(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
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

        // Authorization: user can only access their own contracts
        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own contracts".to_string()),
            });
        }

        match db.get_user_contracts(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create rental request
    ///
    /// Creates a new contract rental request (requires authentication)
    #[oai(path = "/contracts", method = "post", tag = "ApiTags::Contracts")]
    async fn create_rental_request(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        params: Json<crate::database::contracts::RentalRequestParams>,
    ) -> Json<ApiResponse<RentalRequestResponse>> {
        // Validate SSH public key is provided and has valid format
        match &params.0.ssh_pubkey {
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("ssh_pubkey is required for server access".to_string()),
                })
            }
            Some(key) if key.trim().is_empty() => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("ssh_pubkey cannot be empty".to_string()),
                })
            }
            Some(key) => {
                // Validate SSH key format: ssh-(rsa|ed25519|ecdsa|dss) <base64data> [optional comment]
                let ssh_key_pattern = regex::Regex::new(
                    r"^ssh-(rsa|ed25519|ecdsa|dss)\s+[A-Za-z0-9+/]+={0,3}(\s+.*)?$",
                )
                .expect("valid regex");
                if !ssh_key_pattern.is_match(key.trim()) {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(
                            "Invalid SSH key format. Expected: ssh-ed25519 AAAA... or ssh-rsa AAAA..."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        let payment_method = match params.0.payment_method.clone() {
            Some(pm) => pm,
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("payment_method is required".to_string()),
                })
            }
        };

        // Get offering to retrieve currency
        let offering = match db.get_offering(params.0.offering_db_id).await {
            Ok(Some(offering)) => offering,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Offering not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to retrieve offering: {}", e)),
                })
            }
        };

        // Check visibility: public, shared (allowlist), or self-rental
        let requester_pubkey_hex = hex::encode(&auth.pubkey);
        let is_self_rental = requester_pubkey_hex == offering.pubkey;

        // Use unified access check for visibility
        let can_access = match db
            .can_access_offering(
                params.0.offering_db_id,
                &offering.visibility,
                &offering.pubkey,
                Some(&auth.pubkey),
            )
            .await
        {
            Ok(access) => access,
            Err(e) => {
                tracing::error!("Failed to check offering access: {:#?}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Internal error checking access".to_string()),
                });
            }
        };

        if !can_access {
            // Return "not found" to avoid leaking existence of private offerings
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Offering not found".to_string()),
            });
        }

        // Require email verification to rent (anti-Sybil: each real email can only be verified once)
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to look up account by pubkey: {:#?}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Internal error looking up account".to_string()),
                });
            }
        };
        if let Some(ref account_id) = account_id {
            match db.get_account(account_id).await {
                Ok(Some(account)) if !account.email_verified => {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(
                            "Email verification required. Please verify your email address before creating rentals. Check your inbox for the verification link.".to_string(),
                        ),
                    });
                }
                Ok(_) => {} // verified or account not found
                Err(e) => {
                    tracing::error!("Failed to get account: {:#?}", e);
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Internal error checking account status".to_string()),
                    });
                }
            }
        }
        // If no account found (None), allow rental for keypair-only users

        match db.create_rental_request(&auth.pubkey, params.0).await {
            Ok(contract_id) => {
                // Self-rental: no payment needed, skip Stripe checkout
                // Also applies to ICPay which is pre-paid
                let checkout_url = if is_self_rental || payment_method.to_lowercase() != "stripe" {
                    // Self-rental or ICPay: payment_status is "succeeded" immediately, try auto-accept
                    match db.try_auto_accept_contract(&contract_id).await {
                        Ok(true) => {
                            if let Err(e) = db
                                .try_activate_self_provisioned_contract(&contract_id)
                                .await
                            {
                                tracing::warn!(
                                    "Self-provisioned fulfillment failed for contract {}: {:#}",
                                    hex::encode(&contract_id),
                                    e
                                );
                            }

                            // Auto-accepted, try to trigger Hetzner provisioning
                            if let Err(e) = db.try_trigger_hetzner_provisioning(&contract_id).await
                            {
                                tracing::warn!(
                                    "Hetzner provisioning trigger failed for contract {}: {}",
                                    hex::encode(&contract_id),
                                    e
                                );
                            }
                        }
                        Ok(false) => {} // Not eligible for auto-accept
                        Err(e) => {
                            tracing::warn!(
                                "Auto-accept check failed for contract {}: {}",
                                hex::encode(&contract_id),
                                e
                            );
                        }
                    }
                    None
                } else {
                    // Stripe payment required
                    match create_stripe_checkout_session(
                        &db,
                        &contract_id,
                        &offering.currency,
                        &offering.offer_name,
                    )
                    .await
                    {
                        Ok(url) => Some(url),
                        Err(e) => {
                            return Json(ApiResponse {
                                success: false,
                                data: None,
                                error: Some(e),
                            })
                        }
                    }
                };

                // Best-effort: check spending alert and notify if threshold exceeded
                check_spending_alert_and_notify(&db, &auth.pubkey).await;

                let message = if is_self_rental {
                    "Self-rental created successfully (no payment required)".to_string()
                } else {
                    "Rental request created successfully".to_string()
                };

                Json(ApiResponse {
                    success: true,
                    data: Some(RentalRequestResponse {
                        contract_id: hex::encode(&contract_id),
                        message,
                        checkout_url,
                    }),
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

    /// Extend contract
    ///
    /// Extends a contract duration (requires authentication)
    #[oai(
        path = "/contracts/:id/extend",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn extend_contract(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<ExtendContractRequest>,
    ) -> Json<ApiResponse<ExtendContractResponse>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db
            .extend_contract(
                &contract_id,
                &auth.pubkey,
                req.extension_hours,
                req.memo.clone(),
            )
            .await
        {
            Ok(extension_payment_e9s) => match db.get_contract(&contract_id).await {
                Ok(Some(contract)) => Json(ApiResponse {
                    success: true,
                    data: Some(ExtendContractResponse {
                        extension_payment_e9s,
                        new_end_timestamp_ns: contract.end_timestamp_ns.unwrap_or(0),
                        message: format!("Contract extended by {} hours", req.extension_hours),
                    }),
                    error: None,
                }),
                _ => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(
                        "Contract extended but failed to retrieve updated details".to_string(),
                    ),
                }),
            },
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Cancel contract
    ///
    /// Cancels a rental contract (requires authentication)
    #[oai(
        path = "/contracts/:id/cancel",
        method = "put",
        tag = "ApiTags::Contracts"
    )]
    async fn cancel_contract(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<CancelContractRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Create Stripe and ICPay clients for potential refund processing
        let stripe_client = crate::stripe_client::StripeClient::new().ok();
        let icpay_client = crate::icpay_client::IcpayClient::new().ok();

        match db
            .cancel_contract(
                &contract_id,
                &auth.pubkey,
                req.memo.as_deref(),
                stripe_client.as_ref(),
                icpay_client.as_ref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Rental request cancelled successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Set auto-renew preference
    ///
    /// Opts in or out of automatic renewal before the contract expires.
    /// Only the contract requester may change this setting.
    #[oai(
        path = "/contracts/:id/auto-renew",
        method = "put",
        tag = "ApiTags::Contracts"
    )]
    async fn set_auto_renew(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<SetAutoRenewRequest>,
    ) -> Json<ApiResponse<crate::database::contracts::Contract>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db
            .set_contract_auto_renew(&contract_id, &auth.pubkey, req.0.auto_renew)
            .await
        {
            Ok(()) => match db.get_contract(&contract_id).await {
                Ok(Some(contract)) => Json(ApiResponse {
                    success: true,
                    data: Some(contract),
                    error: None,
                }),
                Ok(None) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found after update".to_string()),
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                }),
            },
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract extensions
    ///
    /// Returns extension history for a contract. User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/extensions",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_extensions(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractExtension>>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".into()),
            });
        }

        match db.get_contract_extensions(&contract_id).await {
            Ok(extensions) => Json(ApiResponse {
                success: true,
                data: Some(extensions),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract health checks
    ///
    /// Returns the most recent health check results for a contract.
    /// Both the requester and provider can view health checks for their contract.
    #[oai(
        path = "/contracts/:id/health",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_health(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractHealthCheck>>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".into()),
            });
        }

        match db.get_recent_health_checks(&contract_id, 20).await {
            Ok(checks) => Json(ApiResponse {
                success: true,
                data: Some(checks),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract health summary
    ///
    /// Returns aggregated uptime metrics (total checks, uptime %, avg latency) for a contract.
    /// Both the requester and provider can view the health summary for their contract.
    #[oai(
        path = "/contracts/:id/health-summary",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_health_summary(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::ContractHealthSummary>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".into()),
            });
        }

        match db.get_contract_health_summary(&contract_id).await {
            Ok(summary) => Json(ApiResponse {
                success: true,
                data: Some(summary),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update ICPay transaction ID
    ///
    /// Updates the ICPay transaction ID for a contract after payment (requires authentication)
    #[oai(
        path = "/contracts/:id/icpay-transaction",
        method = "put",
        tag = "ApiTags::Contracts"
    )]
    async fn update_icpay_transaction(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<UpdateIcpayTransactionRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Verify contract exists, user is the requester, and payment hasn't been confirmed
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => contract,
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

        if contract.requester_pubkey != hex::encode(&auth.pubkey) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: only requester can update transaction ID".to_string()),
            });
        }

        // Prevent updating transaction ID if payment already confirmed by webhook
        if contract.icpay_payment_id.is_some() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Transaction ID already confirmed by payment webhook - cannot update".into(),
                ),
            });
        }

        match db
            .update_icpay_transaction_id(&contract_id, &req.transaction_id)
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("ICPay transaction ID updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Verify Stripe checkout session and sync payment status
    ///
    /// Verifies a Stripe checkout session is paid and updates the contract.
    /// This is a fallback for when webhooks fail or are delayed.
    #[oai(
        path = "/contracts/verify-checkout",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn verify_checkout_session(
        &self,
        db: Data<&Arc<Database>>,
        email_service: Data<&Option<Arc<email_utils::EmailService>>>,
        req: Json<VerifyCheckoutSessionRequest>,
    ) -> Json<ApiResponse<VerifyCheckoutSessionResponse>> {
        let stripe_client = match crate::stripe_client::StripeClient::new() {
            Ok(c) => c,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Stripe not configured: {}", e)),
                })
            }
        };

        // Retrieve session from Stripe
        let session_result = match stripe_client
            .retrieve_checkout_session(&req.session_id)
            .await
        {
            Ok(Some(result)) => result,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Payment not yet completed".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to retrieve session: {}", e)),
                })
            }
        };

        let contract_id_bytes = match hex::decode(&session_result.contract_id) {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid contract_id in session: {}", e)),
                })
            }
        };

        // Update contract payment status (idempotent - safe to call multiple times)
        let tax_amount_e9s = session_result.tax_amount_cents.map(|c| c * 10_000_000);
        if let Err(e) = db
            .update_checkout_session_payment(
                &contract_id_bytes,
                &session_result.session_id,
                tax_amount_e9s,
                session_result.customer_tax_id.as_deref(),
                session_result.reverse_charge,
                session_result.invoice_id.as_deref(),
            )
            .await
        {
            tracing::error!(
                "Failed to update payment status for contract {}: {}",
                session_result.contract_id,
                e
            );
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to update payment status: {}", e)),
            });
        }

        tracing::info!(
            "Payment verified via session lookup for contract {}",
            session_result.contract_id
        );

        // Handle receipt: if we have Stripe invoice ID, send now; otherwise schedule for polling
        if session_result.invoice_id.is_some() {
            // Stripe invoice is ready, send receipt now
            match crate::receipts::send_payment_receipt(
                db.as_ref(),
                &contract_id_bytes,
                email_service.as_ref(),
            )
            .await
            {
                Ok(0) => {
                    tracing::debug!(
                        "Receipt already sent for contract {}",
                        session_result.contract_id
                    );
                }
                Ok(receipt_num) => {
                    tracing::info!(
                        "Sent receipt #{} for contract {} via verify-checkout (with Stripe invoice)",
                        receipt_num,
                        session_result.contract_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to send receipt for contract {}: {}",
                        session_result.contract_id,
                        e
                    );
                    // Don't fail - payment was verified successfully
                }
            }
        } else {
            // Stripe invoice not ready yet - schedule for background polling
            // This matches the behavior of checkout.session.completed webhook
            if let Err(e) = db.schedule_pending_stripe_receipt(&contract_id_bytes).await {
                tracing::error!(
                    "Failed to schedule pending receipt for contract {}: {}",
                    session_result.contract_id,
                    e
                );
                // Don't fail - payment was verified successfully
            } else {
                tracing::info!(
                    "Scheduled pending receipt for contract {} (waiting for Stripe invoice)",
                    session_result.contract_id
                );
            }
        }

        // Try auto-accept if provider has it enabled
        match db.try_auto_accept_contract(&contract_id_bytes).await {
            Ok(true) => {
                if let Err(e) = db
                    .try_trigger_hetzner_provisioning(&contract_id_bytes)
                    .await
                {
                    tracing::warn!(
                        "Hetzner provisioning trigger failed for contract {}: {}",
                        session_result.contract_id,
                        e
                    );
                }
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(
                    "Auto-accept check failed for contract {}: {}",
                    session_result.contract_id,
                    e
                );
            }
        }

        Json(ApiResponse {
            success: true,
            data: Some(VerifyCheckoutSessionResponse {
                contract_id: session_result.contract_id,
                payment_status: "succeeded".to_string(),
            }),
            error: None,
        })
    }

    /// Record usage event for a contract
    ///
    /// Records a usage event (heartbeat, session start/end) for billing purposes.
    /// User must be the provider or an authorized agent.
    #[oai(
        path = "/contracts/:id/usage",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn record_usage(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<RecordUsageRequest>,
    ) -> Json<ApiResponse<i64>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Authorization: verify user is the provider
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: only provider can record usage".to_string()),
            });
        }

        // Record the usage event
        match db
            .record_usage_event(
                &contract_id,
                &req.event_type,
                req.units_delta,
                req.heartbeat_at,
                req.source.as_deref(),
                req.metadata.as_deref(),
            )
            .await
        {
            Ok(event_id) => Json(ApiResponse {
                success: true,
                data: Some(event_id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get current usage for a contract
    ///
    /// Returns the current billing period usage for a contract.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/usage",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_usage(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::ContractUsage>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".to_string()),
            });
        }

        match db.get_current_usage(&contract_id).await {
            Ok(Some(usage)) => Json(ApiResponse {
                success: true,
                data: Some(usage),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("No active billing period for this contract".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Submit feedback for a contract
    ///
    /// Submit structured Y/N feedback after a contract is completed/cancelled.
    /// Only the contract requester (renter) can submit feedback, and only once per contract.
    #[oai(
        path = "/contracts/:id/feedback",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn submit_feedback(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<crate::database::stats::SubmitFeedbackInput>,
    ) -> Json<ApiResponse<crate::database::stats::ContractFeedback>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db
            .submit_contract_feedback(&contract_id, &auth.pubkey, &req.0)
            .await
        {
            Ok(feedback) => Json(ApiResponse {
                success: true,
                data: Some(feedback),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get recipe execution log for a contract
    ///
    /// Returns the combined stdout/stderr from the post-provision script execution.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/recipe-log",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_recipe_log(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Option<String>>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".into()),
            });
        }

        match db.get_recipe_log_for_contract(&contract_id).await {
            Ok(log) => Json(ApiResponse {
                success: true,
                data: Some(log),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract event timeline
    ///
    /// Returns all timeline events for a contract in chronological order.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/events",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_events(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractEvent>>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".into()),
            });
        }

        match db.get_contract_events(&contract_id).await {
            Ok(events) => Json(ApiResponse {
                success: true,
                data: Some(events),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get feedback for a contract
    ///
    /// Returns the feedback submitted for a specific contract, if any.
    /// User must be the requester or provider.
    #[oai(
        path = "/contracts/:id/feedback",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_feedback(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Option<crate::database::stats::ContractFeedback>>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        // Authorization: verify user is a party to this contract
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
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

        let user_pubkey = hex::encode(&auth.pubkey);
        if contract.requester_pubkey != user_pubkey && contract.provider_pubkey != user_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: you are not a party to this contract".to_string()),
            });
        }

        match db.get_contract_feedback(&contract_id).await {
            Ok(feedback) => Json(ApiResponse {
                success: true,
                data: Some(feedback),
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
    use crate::database::contracts::RentalRequestParams;
    use crate::database::contracts::{Contract, ContractExtension, ContractUsage};
    use crate::database::stats::ContractFeedback;
    use crate::openapi::common::ApiResponse;
    use crate::openapi::common::{
        CancelContractRequest, ExtendContractRequest, ExtendContractResponse, RecordUsageRequest,
        RentalRequestResponse, UpdateIcpayTransactionRequest, VerifyCheckoutSessionRequest,
        VerifyCheckoutSessionResponse,
    };

    fn sample_contract() -> Contract {
        Contract {
            contract_id: "deadbeef".to_string(),
            requester_pubkey: "aabbcc".to_string(),
            requester_ssh_pubkey: "ssh-ed25519 AAAA test".to_string(),
            requester_contact: "user@example.com".to_string(),
            provider_pubkey: "ddeeff".to_string(),
            offering_id: "offering-1".to_string(),
            region_name: None,
            instance_config: None,
            payment_amount_e9s: 5_000_000_000,
            start_timestamp_ns: Some(1_700_000_000_000_000_000),
            end_timestamp_ns: Some(1_700_003_600_000_000_000),
            duration_hours: Some(1),
            original_duration_hours: Some(1),
            request_memo: "test memo".to_string(),
            created_at_ns: 1_699_990_000_000_000_000,
            status: "requested".to_string(),
            provisioning_instance_details: None,
            provisioning_completed_at_ns: None,
            payment_method: "stripe".to_string(),
            stripe_payment_intent_id: None,
            stripe_customer_id: None,
            icpay_transaction_id: None,
            payment_status: "pending".to_string(),
            currency: "usd".to_string(),
            refund_amount_e9s: None,
            stripe_refund_id: None,
            refund_created_at_ns: None,
            status_updated_at_ns: None,
            icpay_payment_id: None,
            icpay_refund_id: None,
            total_released_e9s: None,
            last_release_at_ns: None,
            tax_amount_e9s: None,
            tax_rate_percent: None,
            tax_type: None,
            tax_jurisdiction: None,
            customer_tax_id: None,
            reverse_charge: None,
            buyer_address: None,
            stripe_invoice_id: None,
            receipt_number: None,
            receipt_sent_at_ns: None,
            stripe_subscription_id: None,
            subscription_status: None,
            current_period_end_ns: None,
            cancel_at_period_end: false,
            auto_renew: false,
            gateway_slug: None,
            gateway_subdomain: None,
            gateway_ssh_port: None,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
            password_reset_requested_at_ns: None,
            ssh_key_rotation_requested_at_ns: None,
            offering_name: None,
            operating_system: None,
        }
    }

    #[test]
    fn test_extend_contract_request_deserialization_with_memo() {
        let json = r#"{"extensionHours":24,"memo":"extend for a day"}"#;
        let req: ExtendContractRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.extension_hours, 24);
        assert_eq!(req.memo.as_deref(), Some("extend for a day"));
    }

    #[test]
    fn test_extend_contract_request_deserialization_without_memo() {
        let json = r#"{"extensionHours":48,"memo":null}"#;
        let req: ExtendContractRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.extension_hours, 48);
        assert!(req.memo.is_none());
    }

    #[test]
    fn test_extend_contract_response_serialization_field_names() {
        let resp = ExtendContractResponse {
            extension_payment_e9s: 1_000_000_000,
            new_end_timestamp_ns: 1_700_003_600_000_000_000,
            message: "Contract extended by 1 hours".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        // camelCase from #[serde(rename_all = "camelCase")]
        assert_eq!(json["extensionPaymentE9s"], 1_000_000_000_i64);
        assert_eq!(json["newEndTimestampNs"], 1_700_003_600_000_000_000_i64);
        assert_eq!(json["message"], "Contract extended by 1 hours");
    }

    #[test]
    fn test_cancel_contract_request_with_memo() {
        let json = r#"{"memo":"no longer needed"}"#;
        let req: CancelContractRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.memo.as_deref(), Some("no longer needed"));
    }

    #[test]
    fn test_cancel_contract_request_without_memo() {
        let json = r#"{"memo":null}"#;
        let req: CancelContractRequest = serde_json::from_str(json).unwrap();
        assert!(req.memo.is_none());
    }

    #[test]
    fn test_rental_request_response_with_checkout_url() {
        let resp = RentalRequestResponse {
            contract_id: "deadbeef".to_string(),
            message: "Rental request created successfully".to_string(),
            checkout_url: Some("https://checkout.stripe.com/pay/cs_test_abc".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["contractId"], "deadbeef");
        assert_eq!(json["message"], "Rental request created successfully");
        assert_eq!(
            json["checkoutUrl"],
            "https://checkout.stripe.com/pay/cs_test_abc"
        );
    }

    #[test]
    fn test_rental_request_response_without_checkout_url() {
        let resp = RentalRequestResponse {
            contract_id: "cafebabe".to_string(),
            message: "Self-rental created successfully (no payment required)".to_string(),
            checkout_url: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["contractId"], "cafebabe");
        assert!(json.get("checkoutUrl").is_none());
    }

    #[test]
    fn test_api_response_contract_success() {
        let contract = sample_contract();
        let resp = ApiResponse {
            success: true,
            data: Some(contract),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json.get("error").is_none());
        assert_eq!(json["data"]["contract_id"], "deadbeef");
        assert_eq!(json["data"]["status"], "requested");
        assert_eq!(json["data"]["payment_amount_e9s"], 5_000_000_000_i64);
    }

    #[test]
    fn test_api_response_contract_error() {
        let resp: ApiResponse<Contract> = ApiResponse {
            success: false,
            data: None,
            error: Some("Contract not found".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json.get("data").is_none());
        assert_eq!(json["error"], "Contract not found");
    }

    #[test]
    fn test_api_response_extend_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(ExtendContractResponse {
                extension_payment_e9s: 500_000_000,
                new_end_timestamp_ns: 1_700_007_200_000_000_000,
                message: "Contract extended by 2 hours".to_string(),
            }),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json.get("error").is_none());
        assert_eq!(json["data"]["extensionPaymentE9s"], 500_000_000_i64);
    }

    #[test]
    fn test_api_response_extend_error() {
        let resp: ApiResponse<ExtendContractResponse> = ApiResponse {
            success: false,
            data: None,
            error: Some("Invalid contract ID format".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json.get("data").is_none());
        assert_eq!(json["error"], "Invalid contract ID format");
    }

    #[test]
    fn test_contract_extension_serialization() {
        let ext = ContractExtension {
            id: 7,
            contract_id: vec![0xde, 0xad],
            extended_by_pubkey: vec![0xca, 0xfe],
            extension_hours: 12,
            extension_payment_e9s: 250_000_000,
            previous_end_timestamp_ns: 1_700_000_000_000_000_000,
            new_end_timestamp_ns: 1_700_043_200_000_000_000,
            extension_memo: Some("extend overnight".to_string()),
            created_at_ns: 1_699_990_000_000_000_000,
        };
        let json = serde_json::to_value(&ext).unwrap();
        assert_eq!(json["id"], 7_i64);
        assert_eq!(json["extension_hours"], 12_i64);
        assert_eq!(json["extension_payment_e9s"], 250_000_000_i64);
        assert_eq!(json["extension_memo"], "extend overnight");
        assert_eq!(
            json["previous_end_timestamp_ns"],
            1_700_000_000_000_000_000_i64
        );
        assert_eq!(json["new_end_timestamp_ns"], 1_700_043_200_000_000_000_i64);
        assert_eq!(json["created_at_ns"], 1_699_990_000_000_000_000_i64);
    }

    #[test]
    fn test_contract_extension_memo_none_serializes_as_null() {
        let ext = ContractExtension {
            id: 1,
            contract_id: vec![],
            extended_by_pubkey: vec![],
            extension_hours: 1,
            extension_payment_e9s: 0,
            previous_end_timestamp_ns: 0,
            new_end_timestamp_ns: 0,
            extension_memo: None,
            created_at_ns: 0,
        };
        let json = serde_json::to_value(&ext).unwrap();
        assert!(
            json.get("extension_memo").is_none(),
            "None memo should be absent from JSON"
        );
    }

    #[test]
    fn test_contract_usage_serialization() {
        let usage = ContractUsage {
            id: 3,
            contract_id: "deadbeef".to_string(),
            billing_period_start: 1_700_000_000,
            billing_period_end: 1_700_003_600,
            units_used: 1.5,
            units_included: Some(10.0),
            overage_units: 0.0,
            estimated_charge_cents: Some(50),
            reported_to_stripe: false,
            stripe_usage_record_id: None,
            created_at: 1_700_000_001,
            updated_at: 1_700_003_601,
            billing_unit: "hour".to_string(),
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(json["id"], 3_i64);
        assert_eq!(json["contract_id"], "deadbeef");
        assert_eq!(json["units_used"], 1.5_f64);
        assert_eq!(json["units_included"], 10.0_f64);
        assert_eq!(json["overage_units"], 0.0_f64);
        assert_eq!(json["estimated_charge_cents"], 50_i64);
        assert_eq!(json["reported_to_stripe"], false);
        assert!(json.get("stripe_usage_record_id").is_none());
        assert_eq!(json["billing_unit"], "hour");
    }

    #[test]
    fn test_feedback_input_deserialization() {
        // SubmitFeedbackInput has no serde rename, so field names are snake_case
        let json = r#"{"service_matched_description":true,"would_rent_again":false}"#;
        let input: crate::database::stats::SubmitFeedbackInput =
            serde_json::from_str(json).unwrap();
        assert!(input.service_matched_description);
        assert!(!input.would_rent_again);
    }

    #[test]
    fn test_contract_feedback_serialization() {
        let feedback = ContractFeedback {
            contract_id: "deadbeef".to_string(),
            provider_pubkey: "aabbcc".to_string(),
            service_matched_description: true,
            would_rent_again: true,
            created_at_ns: 1_700_000_000_000_000_000,
        };
        let json = serde_json::to_value(&feedback).unwrap();
        assert_eq!(json["contract_id"], "deadbeef");
        assert_eq!(json["provider_pubkey"], "aabbcc");
        assert_eq!(json["service_matched_description"], true);
        assert_eq!(json["would_rent_again"], true);
        assert_eq!(json["created_at_ns"], 1_700_000_000_000_000_000_i64);
    }

    #[test]
    fn test_api_response_contract_feedback_success() {
        let feedback = ContractFeedback {
            contract_id: "cafebabe".to_string(),
            provider_pubkey: "112233".to_string(),
            service_matched_description: false,
            would_rent_again: false,
            created_at_ns: 0,
        };
        let resp = ApiResponse {
            success: true,
            data: Some(feedback),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["contract_id"], "cafebabe");
        assert_eq!(json["data"]["service_matched_description"], false);
    }

    #[test]
    fn test_verify_checkout_session_request_deserialization() {
        let json = r#"{"sessionId":"cs_test_abc123"}"#;
        let req: VerifyCheckoutSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.session_id, "cs_test_abc123");
    }

    #[test]
    fn test_verify_checkout_session_response_serialization() {
        let resp = VerifyCheckoutSessionResponse {
            contract_id: "deadbeef".to_string(),
            payment_status: "succeeded".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["contractId"], "deadbeef");
        assert_eq!(json["paymentStatus"], "succeeded");
    }

    #[test]
    fn test_update_icpay_transaction_request_deserialization() {
        let json = r#"{"transactionId":"tx-001"}"#;
        let req: UpdateIcpayTransactionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.transaction_id, "tx-001");
    }

    #[test]
    fn test_record_usage_request_deserialization_all_fields() {
        let json = r#"{"eventType":"heartbeat","unitsDelta":1.0,"heartbeatAt":1700000000,"source":"agent-01","metadata":"{}"}"#;
        let req: RecordUsageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event_type, "heartbeat");
        assert_eq!(req.units_delta, Some(1.0));
        assert_eq!(req.heartbeat_at, Some(1_700_000_000));
        assert_eq!(req.source.as_deref(), Some("agent-01"));
        assert_eq!(req.metadata.as_deref(), Some("{}"));
    }

    #[test]
    fn test_record_usage_request_deserialization_minimal() {
        let json = r#"{"eventType":"session_start"}"#;
        let req: RecordUsageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event_type, "session_start");
        assert!(req.units_delta.is_none());
        assert!(req.heartbeat_at.is_none());
        assert!(req.source.is_none());
        assert!(req.metadata.is_none());
    }

    #[test]
    fn test_rental_request_params_deserialization() {
        // RentalRequestParams has no serde rename, so field names are snake_case
        let json = r#"{"offering_db_id":42,"ssh_pubkey":"ssh-ed25519 AAAA test","payment_method":"stripe","duration_hours":24}"#;
        let params: RentalRequestParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.offering_db_id, 42);
        assert_eq!(params.ssh_pubkey.as_deref(), Some("ssh-ed25519 AAAA test"));
        assert_eq!(params.payment_method.as_deref(), Some("stripe"));
        assert_eq!(params.duration_hours, Some(24));
    }

    #[test]
    fn test_api_response_verify_checkout_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(VerifyCheckoutSessionResponse {
                contract_id: "abc".to_string(),
                payment_status: "succeeded".to_string(),
            }),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["paymentStatus"], "succeeded");
    }

    #[test]
    fn test_contract_health_check_serialization() {
        use crate::database::contracts::ContractHealthCheck;
        let check = ContractHealthCheck {
            id: 1,
            contract_id: "abc123".to_string(),
            checked_at: 1_700_000_000_000_000_000i64,
            status: "healthy".to_string(),
            latency_ms: Some(42),
            details: None,
            created_at: 1_700_000_000_000_000_001i64,
        };
        let v = serde_json::to_value(&check).unwrap();
        assert_eq!(v["status"], "healthy");
        assert_eq!(v["latencyMs"], 42);
        assert_eq!(v["contractId"], "abc123");
        // details is None: field should be absent or null per skip_serializing_if_is_none
        assert!(v.get("details").is_none_or(|d| d.is_null()));
    }

    #[test]
    fn test_contract_health_check_unhealthy_serialization() {
        use crate::database::contracts::ContractHealthCheck;
        let check = ContractHealthCheck {
            id: 2,
            contract_id: "def456".to_string(),
            checked_at: 1_700_000_001_000_000_000i64,
            status: "unhealthy".to_string(),
            latency_ms: None,
            details: Some(r#"{"error":"timeout"}"#.to_string()),
            created_at: 1_700_000_001_000_000_001i64,
        };
        let v = serde_json::to_value(&check).unwrap();
        assert_eq!(v["status"], "unhealthy");
        assert!(v.get("latencyMs").is_none_or(|d| d.is_null()));
        assert_eq!(v["details"], r#"{"error":"timeout"}"#);
    }

    #[test]
    fn test_contract_health_summary_serialization() {
        use crate::database::contracts::ContractHealthSummary;
        let summary = ContractHealthSummary {
            total_checks: 10,
            healthy_checks: 8,
            unhealthy_checks: 1,
            unknown_checks: 1,
            uptime_percent: 80.0,
            avg_latency_ms: Some(15.5),
            last_checked_at: Some(1_700_000_000_000_000_000i64),
        };
        let v = serde_json::to_value(&summary).unwrap();
        assert_eq!(v["totalChecks"], 10_i64);
        assert_eq!(v["healthyChecks"], 8_i64);
        assert_eq!(v["unhealthyChecks"], 1_i64);
        assert_eq!(v["unknownChecks"], 1_i64);
        assert_eq!(v["uptimePercent"], 80.0_f64);
        assert_eq!(v["avgLatencyMs"], 15.5_f64);
        assert_eq!(v["lastCheckedAt"], 1_700_000_000_000_000_000_i64);
    }

    #[test]
    fn test_contract_health_summary_no_checks_serialization() {
        use crate::database::contracts::ContractHealthSummary;
        let summary = ContractHealthSummary {
            total_checks: 0,
            healthy_checks: 0,
            unhealthy_checks: 0,
            unknown_checks: 0,
            uptime_percent: 0.0,
            avg_latency_ms: None,
            last_checked_at: None,
        };
        let v = serde_json::to_value(&summary).unwrap();
        assert_eq!(v["totalChecks"], 0_i64);
        assert_eq!(v["uptimePercent"], 0.0_f64);
        // None fields omitted by skip_serializing_if_is_none
        assert!(v.get("avgLatencyMs").is_none_or(|d| d.is_null()));
        assert!(v.get("lastCheckedAt").is_none_or(|d| d.is_null()));
    }

    #[test]
    fn test_api_response_health_summary_unauthorized_error() {
        use crate::database::contracts::ContractHealthSummary;
        let resp: ApiResponse<ContractHealthSummary> = ApiResponse {
            success: false,
            data: None,
            error: Some("Unauthorized: you are not a party to this contract".to_string()),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["success"], false);
        assert!(v.get("data").is_none());
        assert_eq!(
            v["error"],
            "Unauthorized: you are not a party to this contract"
        );
    }
}
