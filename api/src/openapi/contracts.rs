use super::common::{
    default_limit, ApiResponse, ApiTags, CancelContractRequest, ExtendContractRequest,
    ExtendContractResponse, RentalRequestResponse, UpdateIcpayTransactionRequest,
};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct ContractsApi;

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

    let stripe_client =
        crate::stripe_client::StripeClient::new().map_err(|e| e.to_string())?;

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
    /// List contracts
    ///
    /// Returns a paginated list of all contracts
    #[oai(path = "/contracts", method = "get", tag = "ApiTags::Contracts")]
    async fn list_contracts(
        &self,
        db: Data<&Arc<Database>>,
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
    /// Returns details of a specific contract
    #[oai(path = "/contracts/:id", method = "get", tag = "ApiTags::Contracts")]
    async fn get_contract(
        &self,
        db: Data<&Arc<Database>>,
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
            Ok(Some(contract)) => Json(ApiResponse {
                success: true,
                data: Some(contract),
                error: None,
            }),
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

    /// Get user contracts
    ///
    /// Returns contracts for a specific user (as requester)
    #[oai(
        path = "/users/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_contracts(
        &self,
        db: Data<&Arc<Database>>,
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

        match db.create_rental_request(&auth.pubkey, params.0).await {
            Ok(contract_id) => {
                let checkout_url = if payment_method.to_lowercase() == "stripe" {
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
                } else {
                    None
                };

                Json(ApiResponse {
                    success: true,
                    data: Some(RentalRequestResponse {
                        contract_id: hex::encode(&contract_id),
                        message: "Rental request created successfully".to_string(),
                        client_secret: None,
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

    /// Get contract extensions
    ///
    /// Returns extension history for a contract
    #[oai(
        path = "/contracts/:id/extensions",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_extensions(
        &self,
        db: Data<&Arc<Database>>,
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

        // Verify contract exists and user is the requester
        match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => {
                if contract.requester_pubkey != hex::encode(&auth.pubkey) {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(
                            "Unauthorized: only requester can update transaction ID".to_string(),
                        ),
                    });
                }
            }
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
}
