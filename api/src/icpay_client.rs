use anyhow::{Context, Result};

/// ICPay API client wrapper for payment verification
///
/// Note: ICPay does not provide a Rust SDK. This is a stub implementation
/// that prepares the structure for future HTTP-based payment verification.
pub struct IcpayClient {
    #[allow(dead_code)]
    secret_key: String,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl std::fmt::Debug for IcpayClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IcpayClient")
            .field("secret_key", &"<redacted>")
            .field("client", &"<reqwest::Client>")
            .finish()
    }
}

impl IcpayClient {
    /// Creates a new ICPay client using the API secret key from environment
    pub fn new() -> Result<Self> {
        let secret_key = std::env::var("ICPAY_SECRET_KEY")
            .context("ICPAY_SECRET_KEY environment variable not set")?;

        let client = reqwest::Client::new();
        Ok(Self { secret_key, client })
    }

    /// Verifies a payment by contract metadata
    ///
    /// # Arguments
    /// * `contract_id` - The contract ID to verify payment for
    ///
    /// # Returns
    /// True if a completed payment with matching metadata exists, false otherwise
    ///
    /// # TODO
    /// This is a stub implementation. When ICPay REST API documentation becomes available:
    /// 1. Implement HTTP request to ICPay's getPaymentsByMetadata endpoint
    /// 2. Parse response and check for payment.status === 'completed'
    /// 3. Verify payment.metadata.contractId matches the provided contract_id
    /// 4. Return appropriate Result<bool> based on verification
    ///
    /// Example implementation sketch:
    /// ```ignore
    /// let response = self.client
    ///     .post("https://api.icpay.org/v1/payments/by-metadata")
    ///     .header("Authorization", format!("Bearer {}", self.secret_key))
    ///     .json(&serde_json::json!({ "metadata": { "contractId": contract_id } }))
    ///     .send()
    ///     .await?;
    /// let payments: Vec<Payment> = response.json().await?;
    /// Ok(payments.iter().any(|p| p.status == "completed"))
    /// ```
    #[allow(dead_code)]
    pub async fn verify_payment_by_metadata(&self, contract_id: &str) -> Result<bool> {
        tracing::info!(
            contract_id = %contract_id,
            "ICPay payment verification stub called - assuming payment is valid"
        );

        // TODO: Replace this stub with actual HTTP request to ICPay API
        // For now, trust that the frontend has completed the payment
        // and the transaction_id has been stored in the database
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icpay_client_new_missing_key() {
        // Clear env var to test error handling
        std::env::remove_var("ICPAY_SECRET_KEY");

        let result = IcpayClient::new();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ICPAY_SECRET_KEY"));
    }

    #[test]
    fn test_icpay_client_new_with_key() {
        // Set test key
        std::env::set_var("ICPAY_SECRET_KEY", "sk_test_dummy");

        let result = IcpayClient::new();
        assert!(result.is_ok());

        // Clean up
        std::env::remove_var("ICPAY_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_verify_payment_stub() {
        std::env::set_var("ICPAY_SECRET_KEY", "sk_test_dummy");
        let client = IcpayClient::new().unwrap();

        // Stub implementation always returns true
        let result = client.verify_payment_by_metadata("test-contract-123").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        std::env::remove_var("ICPAY_SECRET_KEY");
    }
}
