use anyhow::{Context, Result};
use dcc_common::DccIdentity;
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};

use super::Identity;

/// HTTP client that automatically signs requests using Ed25519ph
#[allow(dead_code)]
pub struct SignedClient {
    identity: DccIdentity,
    public_key_hex: String,
    base_url: String,
    http: Client,
}

/// Standard API response wrapper
#[derive(Debug, serde::Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    /// Extract data or return error
    pub fn into_result(self) -> Result<T> {
        if self.success {
            self.data.context("API returned success but no data")
        } else {
            anyhow::bail!(
                "API error: {}",
                self.error.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }
}

#[allow(dead_code)]
impl SignedClient {
    /// Create a new signed client from an identity
    pub fn new(identity: &Identity, base_url: &str) -> Result<Self> {
        let dcc_identity = identity.to_dcc_identity()?;
        Ok(SignedClient {
            identity: dcc_identity,
            public_key_hex: identity.public_key_hex.clone(),
            base_url: base_url.trim_end_matches('/').to_string(),
            http: api::http_util::http_client(),
        })
    }

    /// Create a new signed client from a DccIdentity directly
    pub fn from_dcc_identity(identity: DccIdentity, base_url: &str) -> Result<Self> {
        let public_key_hex = hex::encode(identity.to_bytes_verifying());
        Ok(SignedClient {
            identity,
            public_key_hex,
            base_url: base_url.trim_end_matches('/').to_string(),
            http: api::http_util::http_client(),
        })
    }

    /// Get the public key hex string
    pub fn public_key_hex(&self) -> &str {
        &self.public_key_hex
    }

    /// Sign a message using Ed25519ph with "decent-cloud" context
    /// Message format: timestamp + nonce + method + path + body
    fn sign_request(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<(String, String, String)> {
        // Delegates to the shared dcc_common signer so the api-cli and the
        // api-server verifier use one message-layout implementation.
        let signed = dcc_common::api_auth::sign_request(&self.identity, method, path, body)
            .map_err(|e| anyhow::anyhow!("Failed to sign request: {}", e))?;
        Ok((signed.timestamp, signed.nonce, signed.signature_hex))
    }

    /// Make a GET request
    pub async fn get<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let full_path = format!("/api/v1{}", path);
        let url = format!("{}{}", self.base_url, full_path);

        let (timestamp, nonce, signature) = self.sign_request("GET", &full_path, &[])?;

        let response = self
            .http
            .get(&url)
            .header(
                dcc_common::api_auth::HEADER_PUBLIC_KEY,
                &self.public_key_hex,
            )
            .header(dcc_common::api_auth::HEADER_SIGNATURE, &signature)
            .header(dcc_common::api_auth::HEADER_TIMESTAMP, &timestamp)
            .header(dcc_common::api_auth::HEADER_NONCE, &nonce)
            .send()
            .await
            .with_context(|| format!("Failed to send GET request to {}", url))?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("HTTP {} from {}: {}", status, url, text);
        }

        serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse response from {}: {}", url, text))
    }

    /// Make a GET request and unwrap ApiResponse
    pub async fn get_api<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let response: ApiResponse<R> = self.get(path).await?;
        response.into_result()
    }

    /// Make a POST request with JSON body
    pub async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let full_path = format!("/api/v1{}", path);
        let url = format!("{}{}", self.base_url, full_path);
        let body_bytes = serde_json::to_vec(body)?;

        let (timestamp, nonce, signature) = self.sign_request("POST", &full_path, &body_bytes)?;

        let response = self
            .http
            .post(&url)
            .header(
                dcc_common::api_auth::HEADER_PUBLIC_KEY,
                &self.public_key_hex,
            )
            .header(dcc_common::api_auth::HEADER_SIGNATURE, &signature)
            .header(dcc_common::api_auth::HEADER_TIMESTAMP, &timestamp)
            .header(dcc_common::api_auth::HEADER_NONCE, &nonce)
            .header("Content-Type", "application/json")
            .body(body_bytes)
            .send()
            .await
            .with_context(|| format!("Failed to send POST request to {}", url))?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("HTTP {} from {}: {}", status, url, text);
        }

        serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse response from {}: {}", url, text))
    }

    /// Make a POST request and unwrap ApiResponse
    pub async fn post_api<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let response: ApiResponse<R> = self.post(path, body).await?;
        response.into_result()
    }

    /// Make a PUT request with JSON body
    pub async fn put<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let full_path = format!("/api/v1{}", path);
        let url = format!("{}{}", self.base_url, full_path);
        let body_bytes = serde_json::to_vec(body)?;

        let (timestamp, nonce, signature) = self.sign_request("PUT", &full_path, &body_bytes)?;

        let response = self
            .http
            .put(&url)
            .header(
                dcc_common::api_auth::HEADER_PUBLIC_KEY,
                &self.public_key_hex,
            )
            .header(dcc_common::api_auth::HEADER_SIGNATURE, &signature)
            .header(dcc_common::api_auth::HEADER_TIMESTAMP, &timestamp)
            .header(dcc_common::api_auth::HEADER_NONCE, &nonce)
            .header("Content-Type", "application/json")
            .body(body_bytes)
            .send()
            .await
            .with_context(|| format!("Failed to send PUT request to {}", url))?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("HTTP {} from {}: {}", status, url, text);
        }

        serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse response from {}: {}", url, text))
    }

    /// Make a PUT request and unwrap ApiResponse
    pub async fn put_api<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let response: ApiResponse<R> = self.put(path, body).await?;
        response.into_result()
    }

    /// Make a DELETE request
    pub async fn delete<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let full_path = format!("/api/v1{}", path);
        let url = format!("{}{}", self.base_url, full_path);

        let (timestamp, nonce, signature) = self.sign_request("DELETE", &full_path, &[])?;

        let response = self
            .http
            .delete(&url)
            .header(
                dcc_common::api_auth::HEADER_PUBLIC_KEY,
                &self.public_key_hex,
            )
            .header(dcc_common::api_auth::HEADER_SIGNATURE, &signature)
            .header(dcc_common::api_auth::HEADER_TIMESTAMP, &timestamp)
            .header(dcc_common::api_auth::HEADER_NONCE, &nonce)
            .send()
            .await
            .with_context(|| format!("Failed to send DELETE request to {}", url))?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("HTTP {} from {}: {}", status, url, text);
        }

        serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse response from {}: {}", url, text))
    }

    /// Make a DELETE request and unwrap ApiResponse
    pub async fn delete_api<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let response: ApiResponse<R> = self.delete(path).await?;
        response.into_result()
    }

    /// Make an unauthenticated GET request (for public endpoints)
    pub async fn get_public<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = format!("{}/api/v1{}", self.base_url, path);

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to send GET request to {}", url))?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("HTTP {} from {}: {}", status, url, text);
        }

        serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse response from {}: {}", url, text))
    }

    /// Make an unauthenticated GET request and unwrap ApiResponse
    pub async fn get_public_api<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let response: ApiResponse<R> = self.get_public(path).await?;
        response.into_result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response: ApiResponse<String> = ApiResponse {
            success: true,
            data: Some("test".to_string()),
            error: None,
        };
        assert_eq!(response.into_result().unwrap(), "test");
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<String> = ApiResponse {
            success: false,
            data: None,
            error: Some("test error".to_string()),
        };
        let err = response.into_result().unwrap_err();
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_api_response_success_no_data() {
        let response: ApiResponse<String> = ApiResponse {
            success: true,
            data: None,
            error: None,
        };
        let err = response.into_result().unwrap_err();
        assert!(err.to_string().contains("no data"));
    }

    /// Guard against drift back to hand-typed header-name literals: the real
    /// `get()` path must send all four canonical signed-request headers under
    /// the exact names from `dcc_common::api_auth::HEADER_*`. If any is
    /// renamed, removed, or typo'd (the historical `X-DC-*` bug), mockito's
    /// header matchers fail to match and `mock.assert_async()` fails.
    #[tokio::test]
    async fn test_signed_request_sends_canonical_auth_headers() {
        let mut server = mockito::Server::new_async().await;

        let identity = dcc_common::DccIdentity::new_signing_from_bytes(&[9u8; 32])
            .expect("valid test signing key");
        let client =
            SignedClient::from_dcc_identity(identity, &server.url()).expect("client from identity");
        // The mock ONLY matches when all four canonical auth headers are
        // present; X-Public-Key must additionally carry this client's pubkey.
        let expected_pubkey = client.public_key_hex().to_string();
        let mock = server
            .mock("GET", "/api/v1/health")
            .match_header(
                dcc_common::api_auth::HEADER_PUBLIC_KEY,
                mockito::Matcher::Exact(expected_pubkey),
            )
            .match_header(
                dcc_common::api_auth::HEADER_SIGNATURE,
                mockito::Matcher::Any,
            )
            .match_header(
                dcc_common::api_auth::HEADER_TIMESTAMP,
                mockito::Matcher::Any,
            )
            .match_header(dcc_common::api_auth::HEADER_NONCE, mockito::Matcher::Any)
            .with_status(200)
            .with_body(r#"{"success":true,"data":{"ok":true}}"#)
            .create_async()
            .await;

        let result: ApiResponse<serde_json::Value> = client.get("/health").await.unwrap();
        assert!(result.success);
        // Fails if any canonical header was missing or carried the wrong name.
        mock.assert_async().await;
    }
}
