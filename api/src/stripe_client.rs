use anyhow::{Context, Result};
use stripe::{
    Client, CreatePaymentIntent, CreateRefund, Currency, PaymentIntent, PaymentIntentId, Refund,
    RefundId,
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
}
