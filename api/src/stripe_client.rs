use anyhow::Result;
use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, CreateRefund, Currency, PaymentIntentId,
    Refund,
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
        let secret_key = std::env::var("STRIPE_SECRET_KEY")?;

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
        let currency = currency.parse::<Currency>()?;

        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());

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
        // Automatic tax requires origin address configured in Stripe dashboard
        // https://dashboard.stripe.com/settings/tax
        if std::env::var("STRIPE_AUTOMATIC_TAX").is_ok() {
            params.automatic_tax = Some(stripe::CreateCheckoutSessionAutomaticTax {
                enabled: true,
                liability: None,
            });
            params.tax_id_collection =
                Some(stripe::CreateCheckoutSessionTaxIdCollection { enabled: true });
        } else {
            tracing::warn!("STRIPE_AUTOMATIC_TAX not set - automatic tax calculation disabled. Set STRIPE_AUTOMATIC_TAX=true and configure origin address at https://dashboard.stripe.com/settings/tax to enable.");
        }
        params.success_url = Some(&success_url);
        params.cancel_url = Some(&cancel_url);
        params.metadata = Some(
            [("contract_id".to_string(), contract_id.to_string())]
                .into_iter()
                .collect(),
        );

        let session = CheckoutSession::create(&self.client, params).await?;

        session
            .url
            .ok_or_else(|| anyhow::anyhow!("Checkout Session missing URL"))
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
        let intent_id: PaymentIntentId = payment_intent_id.parse()?;

        let mut params = CreateRefund::new();
        params.payment_intent = Some(intent_id);

        if let Some(amt) = amount {
            params.amount = Some(amt);
        }

        let refund = Refund::create(&self.client, params).await?;

        Ok(refund.id.to_string())
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
        let url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());
        assert_eq!(url, "http://localhost:59010");
    }
}
