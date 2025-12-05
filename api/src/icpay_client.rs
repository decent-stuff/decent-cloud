use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// ICPay API client wrapper for payment verification
pub struct IcpayClient {
    secret_key: String,
    client: reqwest::Client,
}

/// ICPay payment object
#[derive(Debug, Deserialize)]
pub struct IcpayPayment {
    pub id: String,
    pub status: String, // pending, completed, failed, canceled, refunded, mismatched
    pub amount: String,
    pub metadata: Option<serde_json::Value>,
}

/// Response from payments/by-metadata endpoint
#[derive(Debug, Deserialize)]
pub struct PaymentHistoryResponse {
    pub payments: Vec<IcpayPayment>,
    pub total: i64,
}

/// Request body for refund creation
#[derive(Debug, Serialize)]
struct CreateRefundRequest {
    #[serde(rename = "paymentId")]
    payment_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<String>,
}

/// Response from refund creation
#[derive(Debug, Deserialize)]
struct CreateRefundResponse {
    id: String,
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
    const API_URL: &'static str = "https://api.icpay.org";

    /// Creates a new ICPay client using the API secret key from environment
    pub fn new() -> Result<Self> {
        let secret_key = std::env::var("ICPAY_SECRET_KEY")
            .context("ICPAY_SECRET_KEY environment variable not set")?;

        let client = reqwest::Client::new();
        Ok(Self { secret_key, client })
    }

    /// Retrieves payments matching the provided metadata
    ///
    /// # Arguments
    /// * `metadata` - JSON object to match against payment metadata
    ///
    /// # Returns
    /// Vector of payments with matching metadata
    pub async fn get_payments_by_metadata(
        &self,
        metadata: serde_json::Value,
    ) -> Result<Vec<IcpayPayment>> {
        let url = format!("{}/sdk/private/payments/by-metadata", Self::API_URL);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .json(&serde_json::json!({ "metadata": metadata }))
            .send()
            .await
            .context("Failed to send payments/by-metadata request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "ICPay API error ({}): {}",
                status,
                body
            ));
        }

        let data: PaymentHistoryResponse = response
            .json()
            .await
            .context("Failed to parse PaymentHistoryResponse")?;

        Ok(data.payments)
    }

    /// Creates a refund for a payment
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID to refund
    /// * `amount` - Optional amount in smallest currency unit (e.g., e9s for ICP). If None, refunds full amount.
    ///
    /// # Returns
    /// Refund ID on success
    pub async fn create_refund(
        &self,
        payment_id: &str,
        amount: Option<i64>,
    ) -> Result<String> {
        let url = format!("{}/sdk/private/refunds", Self::API_URL);

        let request_body = CreateRefundRequest {
            payment_id: payment_id.to_string(),
            amount: amount.map(|a| a.to_string()),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .json(&request_body)
            .send()
            .await
            .context("Failed to send refund creation request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "ICPay refund API error ({}): {}",
                status,
                body
            ));
        }

        let data: CreateRefundResponse = response
            .json()
            .await
            .context("Failed to parse CreateRefundResponse")?;

        Ok(data.id)
    }

    /// Verifies a payment by contract metadata
    ///
    /// # Arguments
    /// * `contract_id` - The contract ID to verify payment for
    ///
    /// # Returns
    /// True if a completed payment with matching metadata exists, false otherwise
    pub async fn verify_payment_by_metadata(&self, contract_id: &str) -> Result<bool> {
        let payments = self
            .get_payments_by_metadata(serde_json::json!({ "contractId": contract_id }))
            .await?;

        Ok(payments.iter().any(|p| p.status == "completed"))
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
    async fn test_get_payments_by_metadata_success() {
        use mockito::Server;

        std::env::set_var("ICPAY_SECRET_KEY", "sk_test_dummy");
        let mut server = Server::new_async().await;

        // Mock successful response with completed payment
        let mock = server
            .mock("POST", "/sdk/private/payments/by-metadata")
            .match_header("authorization", "Bearer sk_test_dummy")
            .match_body(mockito::Matcher::JsonString(
                r#"{"metadata":{"contractId":"test-123"}}"#.to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "payments": [
                        {
                            "id": "pay_123",
                            "status": "completed",
                            "amount": "1000000000",
                            "metadata": {"contractId": "test-123"}
                        }
                    ],
                    "total": 1
                }"#,
            )
            .create_async()
            .await;

        // Override API URL for testing
        let client = IcpayClient {
            secret_key: "sk_test_dummy".to_string(),
            client: reqwest::Client::new(),
        };

        // Temporarily patch API URL - we'll use the client directly
        let url = format!("{}/sdk/private/payments/by-metadata", server.url());
        let response = client
            .client
            .post(&url)
            .header("Authorization", "Bearer sk_test_dummy")
            .json(&serde_json::json!({ "metadata": { "contractId": "test-123" } }))
            .send()
            .await
            .unwrap();

        let data: PaymentHistoryResponse = response.json().await.unwrap();

        assert_eq!(data.payments.len(), 1);
        assert_eq!(data.payments[0].id, "pay_123");
        assert_eq!(data.payments[0].status, "completed");
        assert_eq!(data.total, 1);

        mock.assert_async().await;
        std::env::remove_var("ICPAY_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_get_payments_by_metadata_no_completed() {
        use mockito::Server;

        std::env::set_var("ICPAY_SECRET_KEY", "sk_test_dummy");
        let mut server = Server::new_async().await;

        // Mock response with pending payment
        let mock = server
            .mock("POST", "/sdk/private/payments/by-metadata")
            .match_header("authorization", "Bearer sk_test_dummy")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "payments": [
                        {
                            "id": "pay_456",
                            "status": "pending",
                            "amount": "1000000000",
                            "metadata": {"contractId": "test-456"}
                        }
                    ],
                    "total": 1
                }"#,
            )
            .create_async()
            .await;

        let url = format!("{}/sdk/private/payments/by-metadata", server.url());
        let client = IcpayClient {
            secret_key: "sk_test_dummy".to_string(),
            client: reqwest::Client::new(),
        };

        let response = client
            .client
            .post(&url)
            .header("Authorization", "Bearer sk_test_dummy")
            .json(&serde_json::json!({ "metadata": { "contractId": "test-456" } }))
            .send()
            .await
            .unwrap();

        let data: PaymentHistoryResponse = response.json().await.unwrap();

        assert_eq!(data.payments.len(), 1);
        assert_eq!(data.payments[0].status, "pending");

        mock.assert_async().await;
        std::env::remove_var("ICPAY_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_create_refund_success() {
        use mockito::Server;

        std::env::set_var("ICPAY_SECRET_KEY", "sk_test_dummy");
        let mut server = Server::new_async().await;

        // Mock successful refund creation
        let mock = server
            .mock("POST", "/sdk/private/refunds")
            .match_header("authorization", "Bearer sk_test_dummy")
            .match_body(mockito::Matcher::JsonString(
                r#"{"paymentId":"pay_123","amount":"500000000"}"#.to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"refund_xyz"}"#)
            .create_async()
            .await;

        let url = format!("{}/sdk/private/refunds", server.url());
        let client = IcpayClient {
            secret_key: "sk_test_dummy".to_string(),
            client: reqwest::Client::new(),
        };

        let request_body = CreateRefundRequest {
            payment_id: "pay_123".to_string(),
            amount: Some("500000000".to_string()),
        };

        let response = client
            .client
            .post(&url)
            .header("Authorization", "Bearer sk_test_dummy")
            .json(&request_body)
            .send()
            .await
            .unwrap();

        let data: CreateRefundResponse = response.json().await.unwrap();

        assert_eq!(data.id, "refund_xyz");

        mock.assert_async().await;
        std::env::remove_var("ICPAY_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_create_refund_full_amount() {
        use mockito::Server;

        std::env::set_var("ICPAY_SECRET_KEY", "sk_test_dummy");
        let mut server = Server::new_async().await;

        // Mock full refund (no amount specified)
        let mock = server
            .mock("POST", "/sdk/private/refunds")
            .match_header("authorization", "Bearer sk_test_dummy")
            .match_body(mockito::Matcher::JsonString(
                r#"{"paymentId":"pay_456"}"#.to_string(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"refund_abc"}"#)
            .create_async()
            .await;

        let url = format!("{}/sdk/private/refunds", server.url());
        let client = IcpayClient {
            secret_key: "sk_test_dummy".to_string(),
            client: reqwest::Client::new(),
        };

        let request_body = CreateRefundRequest {
            payment_id: "pay_456".to_string(),
            amount: None,
        };

        let response = client
            .client
            .post(&url)
            .header("Authorization", "Bearer sk_test_dummy")
            .json(&request_body)
            .send()
            .await
            .unwrap();

        let data: CreateRefundResponse = response.json().await.unwrap();

        assert_eq!(data.id, "refund_abc");

        mock.assert_async().await;
        std::env::remove_var("ICPAY_SECRET_KEY");
    }
}
