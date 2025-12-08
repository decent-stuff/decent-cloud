use super::common::ApiTags;
use poem_openapi::{payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};

pub struct VatApi;

/// VAT ID validation request
#[derive(Debug, Deserialize, Object)]
pub struct ValidateVatRequest {
    /// Two-letter country code (e.g., "DE", "FR", "ES")
    pub country_code: String,
    /// VAT number without country prefix
    pub vat_number: String,
}

/// VAT ID validation response
#[derive(Debug, Serialize, Object)]
pub struct ValidateVatResponse {
    /// Whether the VAT ID is valid
    pub valid: bool,
    /// Company name (if available)
    pub name: Option<String>,
    /// Company address (if available)
    pub address: Option<String>,
    /// Error message (if validation failed)
    pub error: Option<String>,
}

#[OpenApi]
impl VatApi {
    /// Validate EU VAT ID
    ///
    /// Validates a VAT identification number using the EU VIES service.
    /// This is a public endpoint that does not require authentication.
    ///
    /// # VIES Service
    /// The VIES (VAT Information Exchange System) is provided by the European Commission
    /// and may be slow or temporarily unavailable. This endpoint will return an error
    /// if the VIES service is unreachable.
    #[oai(path = "/api/v1/vat/validate", method = "post", tag = "ApiTags::System")]
    async fn validate_vat(&self, request: Json<ValidateVatRequest>) -> Json<ValidateVatResponse> {
        let result = crate::vies::validate_vat_id(&request.country_code, &request.vat_number).await;

        match result {
            Ok(vies_response) => Json(ValidateVatResponse {
                valid: vies_response.valid,
                name: vies_response.name,
                address: vies_response.address,
                error: None,
            }),
            Err(e) => Json(ValidateVatResponse {
                valid: false,
                name: None,
                address: None,
                error: Some(format!("VIES validation failed: {}", e)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_vat_request_deserialization() {
        let json = r#"{"country_code": "DE", "vat_number": "123456789"}"#;
        let req: ValidateVatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.country_code, "DE");
        assert_eq!(req.vat_number, "123456789");
    }

    #[test]
    fn test_validate_vat_response_serialization() {
        let response = ValidateVatResponse {
            valid: true,
            name: Some("Example GmbH".to_string()),
            address: Some("Musterstrasse 1".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("Example GmbH"));
        assert!(json.contains("Musterstrasse 1"));
    }

    #[test]
    fn test_validate_vat_error_response() {
        let response = ValidateVatResponse {
            valid: false,
            name: None,
            address: None,
            error: Some("VIES service unavailable".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("VIES service unavailable"));
    }
}
