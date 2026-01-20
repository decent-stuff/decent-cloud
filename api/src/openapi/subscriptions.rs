use super::common::{ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::database::{AccountSubscription, Database, SubscriptionPlan};
use crate::stripe_client::StripeClient;
use poem::web::Data;
use poem_openapi::{payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Response with checkout URL
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct CheckoutUrlResponse {
    pub checkout_url: String,
}

/// Response with portal URL
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct PortalUrlResponse {
    pub portal_url: String,
}

/// Request to create subscription checkout
#[derive(Debug, Deserialize, Object)]
pub struct CreateSubscriptionCheckoutRequest {
    /// Plan ID to subscribe to
    pub plan_id: String,
}

/// Request to cancel subscription
#[derive(Debug, Deserialize, Object)]
pub struct CancelSubscriptionRequest {
    /// If true, cancel at end of billing period; if false, cancel immediately
    pub at_period_end: bool,
}

pub struct SubscriptionsApi;

#[OpenApi]
impl SubscriptionsApi {
    /// List subscription plans
    ///
    /// Returns all available subscription plans with their features and pricing
    #[oai(
        path = "/subscriptions/plans",
        method = "get",
        tag = "ApiTags::Subscriptions"
    )]
    async fn list_plans(
        &self,
        db: Data<&Arc<Database>>,
    ) -> Json<ApiResponse<Vec<SubscriptionPlan>>> {
        match db.list_subscription_plans().await {
            Ok(plans) => Json(ApiResponse {
                success: true,
                data: Some(plans),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get current subscription
    ///
    /// Returns the authenticated user's current subscription details
    #[oai(
        path = "/subscriptions/current",
        method = "get",
        tag = "ApiTags::Subscriptions"
    )]
    async fn get_current(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<AccountSubscription>> {
        // Get account ID from public key
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found for this key".to_string()),
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

        match db.get_account_subscription(&account_id).await {
            Ok(subscription) => Json(ApiResponse {
                success: true,
                data: Some(subscription),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create subscription checkout
    ///
    /// Creates a Stripe Checkout session for subscribing to a plan.
    /// Returns a URL to redirect the user to.
    #[oai(
        path = "/subscriptions/checkout",
        method = "post",
        tag = "ApiTags::Subscriptions"
    )]
    async fn create_checkout(
        &self,
        db: Data<&Arc<Database>>,
        stripe_client: Data<&Option<Arc<StripeClient>>>,
        auth: ApiAuthenticatedUser,
        body: Json<CreateSubscriptionCheckoutRequest>,
    ) -> Json<ApiResponse<CheckoutUrlResponse>> {
        // Check if Stripe is configured
        let stripe = match stripe_client.as_ref() {
            Some(s) => s,
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Stripe not configured".to_string()),
                })
            }
        };

        // Get account ID from public key
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found for this key".to_string()),
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

        // Get the plan
        let plan = match db.get_subscription_plan(&body.plan_id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Plan '{}' not found", body.plan_id)),
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

        // Verify plan has a Stripe price
        let stripe_price_id = match &plan.stripe_price_id {
            Some(id) if !id.is_empty() => id,
            _ => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Plan '{}' is not available for subscription (no Stripe price configured)",
                        body.plan_id
                    )),
                })
            }
        };

        // Get account for email
        let account = match db.get_account_by_id(&account_id).await {
            Ok(Some(a)) => a,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
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

        let account_id_hex = hex::encode(&account_id);

        // Get or create Stripe customer
        let customer_id = match db.get_stripe_customer_id(&account_id).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                // Create new customer
                match stripe
                    .get_or_create_customer(
                        &account_id_hex,
                        account.email.as_deref(),
                        account.display_name.as_deref(),
                    )
                    .await
                {
                    Ok(id) => {
                        // Save customer ID
                        if let Err(e) = db.set_stripe_customer_id(&account_id, &id).await {
                            tracing::error!("Failed to save Stripe customer ID: {:#}", e);
                        }
                        id
                    }
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(format!("Failed to create Stripe customer: {}", e)),
                        })
                    }
                }
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Create checkout session
        let trial_days = if plan.trial_days > 0 {
            plan.trial_days as u32
        } else {
            0
        };

        match stripe
            .create_subscription_checkout(
                &customer_id,
                stripe_price_id,
                trial_days,
                &account_id_hex,
            )
            .await
        {
            Ok(url) => Json(ApiResponse {
                success: true,
                data: Some(CheckoutUrlResponse { checkout_url: url }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create checkout session: {}", e)),
            }),
        }
    }

    /// Create billing portal session
    ///
    /// Creates a Stripe Billing Portal session for managing subscription.
    /// Returns a URL to redirect the user to.
    #[oai(
        path = "/subscriptions/portal",
        method = "post",
        tag = "ApiTags::Subscriptions"
    )]
    async fn create_portal(
        &self,
        db: Data<&Arc<Database>>,
        stripe_client: Data<&Option<Arc<StripeClient>>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<PortalUrlResponse>> {
        // Check if Stripe is configured
        let stripe = match stripe_client.as_ref() {
            Some(s) => s,
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Stripe not configured".to_string()),
                })
            }
        };

        // Get account ID from public key
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found for this key".to_string()),
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

        // Get Stripe customer ID
        let customer_id = match db.get_stripe_customer_id(&account_id).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("No Stripe customer found. Subscribe to a plan first.".to_string()),
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

        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());
        let return_url = format!("{}/dashboard/account/subscription", frontend_url);

        match stripe
            .create_portal_session(&customer_id, &return_url)
            .await
        {
            Ok(url) => Json(ApiResponse {
                success: true,
                data: Some(PortalUrlResponse { portal_url: url }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create portal session: {}", e)),
            }),
        }
    }

    /// Cancel subscription
    ///
    /// Cancels the user's current subscription.
    #[oai(
        path = "/subscriptions/cancel",
        method = "post",
        tag = "ApiTags::Subscriptions"
    )]
    async fn cancel(
        &self,
        db: Data<&Arc<Database>>,
        stripe_client: Data<&Option<Arc<StripeClient>>>,
        auth: ApiAuthenticatedUser,
        body: Json<CancelSubscriptionRequest>,
    ) -> Json<ApiResponse<AccountSubscription>> {
        // Check if Stripe is configured
        let stripe = match stripe_client.as_ref() {
            Some(s) => s,
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Stripe not configured".to_string()),
                })
            }
        };

        // Get account ID from public key
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found for this key".to_string()),
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

        // Get current subscription
        let subscription = match db.get_account_subscription(&account_id).await {
            Ok(s) => s,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Check if there's a Stripe subscription to cancel
        let stripe_subscription_id = match &subscription.stripe_subscription_id {
            Some(id) if !id.is_empty() => id,
            _ => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("No active subscription to cancel".to_string()),
                })
            }
        };

        // Cancel via Stripe
        match stripe
            .cancel_subscription(stripe_subscription_id, body.at_period_end)
            .await
        {
            Ok(_) => {
                // Refresh subscription data
                match db.get_account_subscription(&account_id).await {
                    Ok(updated) => Json(ApiResponse {
                        success: true,
                        data: Some(updated),
                        error: None,
                    }),
                    Err(e) => Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!(
                            "Subscription canceled but failed to refresh: {}",
                            e
                        )),
                    }),
                }
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to cancel subscription: {}", e)),
            }),
        }
    }
}
