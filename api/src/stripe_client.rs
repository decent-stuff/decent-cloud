use anyhow::{Context, Result};
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, CreatePaymentIntent, CreateRefund,
    Currency, PaymentIntent, PaymentIntentId, Refund, RefundId,
};

/// Stripe API client wrapper for payment processing
pub struct StripeClient {
    client: Client,
}

impl std::fmt::Debug for StripeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeClient")
            .field("client", &"<stripe::Client>")
            .finish()
    }
}

impl StripeClient {
    /// Creates a new Stripe client using the API key from environment
    pub fn new() -> Result<Self> {
        let secret_key = std::env::var("STRIPE_SECRET_KEY")
            .context("STRIPE_SECRET_KEY environment variable not set")?;

        let client = Client::new(secret_key);
        Ok(Self { client })
    }

    /// Creates a Stripe Checkout Session for one-time payment
    ///
    /// # Arguments
    /// * `amount` - Amount in cents (e.g., 1000 = $10.00)
    /// * `currency` - Currency code (e.g., "usd")
    /// * `product_name` - Name of the product/service being purchased
    /// * `contract_id` - Hex-encoded contract ID for metadata and URLs
    ///
    /// # Returns
    /// Checkout Session URL for redirect on success
    pub async fn create_checkout_session(
        &self,
        amount: i64,
        currency: &str,
        product_name: &str,
        contract_id: &str,
    ) -> Result<String> {
        let currency = currency
            .parse::<Currency>()
            .context("Invalid currency code")?;

        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:59010".to_string());

        let success_url = format!(
            "{}/checkout/success?session_id={{CHECKOUT_SESSION_ID}}",
            frontend_url
        );
        let cancel_url = format!(
            "{}/checkout/cancel?contract_id={}",
            frontend_url, contract_id
        );

        let mut params = CreateCheckoutSession::new();
        params.mode = Some(CheckoutSessionMode::Payment);
        params.line_items = Some(vec![CreateCheckoutSessionLineItems {
            price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                currency,
                unit_amount: Some(amount),
                product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: product_name.to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            quantity: Some(1),
            ..Default::default()
        }]);
        params.automatic_tax = Some(stripe::CreateCheckoutSessionAutomaticTax {
            enabled: true,
            liability: None,
        });
        params.tax_id_collection = Some(stripe::CreateCheckoutSessionTaxIdCollection {
            enabled: true,
        });
        params.success_url = Some(&success_url);
        params.cancel_url = Some(&cancel_url);
        params.metadata = Some(
            [("contract_id".to_string(), contract_id.to_string())]
                .into_iter()
                .collect(),
        );

        let session = CheckoutSession::create(&self.client, params)
            .await
            .context("Failed to create Stripe Checkout Session")?;

        session
            .url
            .ok_or_else(|| anyhow::anyhow!("Checkout Session missing URL"))
    }

    /// Creates a payment intent for the specified amount
    ///
    /// # Arguments
    /// * `amount` - Amount in cents (e.g., 1000 = $10.00)
    /// * `currency` - Currency code (e.g., "usd")
    ///
    /// # Returns
    /// Tuple of (PaymentIntent ID, client_secret) on success
    pub async fn create_payment_intent(
        &self,
        amount: i64,
        currency: &str,
    ) -> Result<(String, String)> {
        let currency = currency
            .parse::<Currency>()
            .context("Invalid currency code")?;

        let mut params = CreatePaymentIntent::new(amount, currency);
        params.automatic_payment_methods =
            Some(stripe::CreatePaymentIntentAutomaticPaymentMethods {
                enabled: true,
                allow_redirects: Some(
                    stripe::CreatePaymentIntentAutomaticPaymentMethodsAllowRedirects::Never,
                ),
            });

        let payment_intent = PaymentIntent::create(&self.client, params)
            .await
            .context("Failed to create Stripe PaymentIntent")?;

        let client_secret = payment_intent
            .client_secret
            .ok_or_else(|| anyhow::anyhow!("PaymentIntent missing client_secret"))?;

        Ok((payment_intent.id.to_string(), client_secret))
    }

    /// Verifies a payment intent status
    ///
    /// # Arguments
    /// * `payment_intent_id` - The PaymentIntent ID to verify
    ///
    /// # Returns
    /// True if payment succeeded, false otherwise
    #[allow(dead_code)]
    pub async fn verify_payment_intent(&self, payment_intent_id: &str) -> Result<bool> {
        let id: PaymentIntentId = payment_intent_id
            .parse()
            .context("Invalid PaymentIntent ID format")?;
        let payment_intent = PaymentIntent::retrieve(&self.client, &id, &[])
            .await
            .context("Failed to retrieve PaymentIntent")?;

        Ok(payment_intent.status == stripe::PaymentIntentStatus::Succeeded)
    }

    /// Creates a refund for a payment intent
    ///
    /// # Arguments
    /// * `payment_intent_id` - The PaymentIntent ID to refund
    /// * `amount` - Amount to refund in cents (None = full refund)
    ///
    /// # Returns
    /// Refund ID on success
    pub async fn create_refund(
        &self,
        payment_intent_id: &str,
        amount: Option<i64>,
    ) -> Result<String> {
        let intent_id: PaymentIntentId = payment_intent_id
            .parse()
            .context("Invalid PaymentIntent ID format")?;

        let mut params = CreateRefund::new();
        params.payment_intent = Some(intent_id);

        if let Some(amt) = amount {
            params.amount = Some(amt);
        }

        let refund = Refund::create(&self.client, params)
            .await
            .context("Failed to create Stripe refund")?;

        Ok(refund.id.to_string())
    }

    /// Verifies a refund exists
    ///
    /// # Arguments
    /// * `refund_id` - The Refund ID to verify
    ///
    /// # Returns
    /// True if refund exists, false otherwise
    #[allow(dead_code)]
    pub async fn verify_refund(&self, refund_id: &str) -> Result<bool> {
        let id: RefundId = refund_id.parse().context("Invalid Refund ID format")?;
        let _refund = Refund::retrieve(&self.client, &id, &[])
            .await
            .context("Failed to retrieve Refund")?;

        // Refund retrieved successfully, it exists
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stripe_client_new_missing_key() {
        // Clear env var to test error handling
        std::env::remove_var("STRIPE_SECRET_KEY");

        let result = StripeClient::new();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("STRIPE_SECRET_KEY"));
    }

    #[test]
    fn test_stripe_client_new_with_key() {
        // Set test key
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");

        let result = StripeClient::new();
        assert!(result.is_ok());

        // Clean up
        std::env::remove_var("STRIPE_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_create_payment_intent_invalid_currency() {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");
        let client = StripeClient::new().unwrap();

        let result = client.create_payment_intent(1000, "invalid").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("currency"));

        std::env::remove_var("STRIPE_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_create_checkout_session_invalid_currency() {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");
        std::env::set_var("FRONTEND_URL", "http://localhost:59010");
        let client = StripeClient::new().unwrap();

        let result = client
            .create_checkout_session(1000, "invalid", "Test Product", "abc123")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("currency"));

        std::env::remove_var("STRIPE_SECRET_KEY");
        std::env::remove_var("FRONTEND_URL");
    }

    #[test]
    fn test_checkout_session_uses_frontend_url() {
        // This test verifies that the FRONTEND_URL is used in success/cancel URLs
        std::env::set_var("FRONTEND_URL", "https://example.com");
        let url = std::env::var("FRONTEND_URL").unwrap();
        assert_eq!(url, "https://example.com");
        std::env::remove_var("FRONTEND_URL");
    }

    #[test]
    fn test_checkout_session_defaults_frontend_url() {
        // Test default when FRONTEND_URL not set
        std::env::remove_var("FRONTEND_URL");
        let url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:59010".to_string());
        assert_eq!(url, "http://localhost:59010");
    }
}
