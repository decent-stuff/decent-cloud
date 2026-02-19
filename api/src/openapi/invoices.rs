use super::common::{ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use crate::invoices::{get_invoice_metadata, get_invoice_pdf, Invoice};
use poem::web::Data;
use poem_openapi::{
    param::Path, payload::Binary, payload::Json, payload::Response as PoemResponse, OpenApi,
};
use std::sync::Arc;

pub struct InvoicesApi;

#[OpenApi]
impl InvoicesApi {
    /// Get invoice PDF
    ///
    /// Returns the PDF invoice for a contract. Generates on-demand if not cached.
    #[oai(
        path = "/contracts/:id/invoice",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_invoice(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> PoemResponse<InvoicePdfResponse> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return PoemResponse::new(InvoicePdfResponse::BadRequest(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })))
            }
        };

        // Get contract to verify auth
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return PoemResponse::new(InvoicePdfResponse::NotFound(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
                })))
            }
            Err(e) => {
                return PoemResponse::new(InvoicePdfResponse::InternalError(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })))
            }
        };

        // Verify auth: requester or provider
        let requester_pk = match hex::decode(&contract.requester_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                tracing::warn!("Malformed hex in contract.requester_pubkey: {:#}", e);
                return PoemResponse::new(InvoicePdfResponse::InternalError(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format in database".to_string()),
                })));
            }
        };
        let provider_pk = match hex::decode(&contract.provider_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                tracing::warn!("Malformed hex in contract.provider_pubkey: {:#}", e);
                return PoemResponse::new(InvoicePdfResponse::InternalError(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format in database".to_string()),
                })));
            }
        };

        if auth.pubkey != requester_pk && auth.pubkey != provider_pk {
            return PoemResponse::new(InvoicePdfResponse::Forbidden(Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Not authorized to view this invoice".to_string()),
            })));
        }

        // Get or generate PDF
        match get_invoice_pdf(&db, &contract_id).await {
            Ok(pdf_bytes) => PoemResponse::new(InvoicePdfResponse::Ok(Binary(pdf_bytes))),
            Err(e) => {
                tracing::error!("Failed to generate invoice PDF: {:#}", e);
                PoemResponse::new(InvoicePdfResponse::InternalError(Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to generate invoice: {}", e)),
                })))
            }
        }
    }

    /// Get invoice metadata
    ///
    /// Returns invoice metadata (without PDF binary) for a contract
    #[oai(
        path = "/contracts/:id/invoice/metadata",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_invoice_metadata(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<Invoice>> {
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

        // Get contract to verify auth
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
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        // Verify auth: requester or provider
        let requester_pk = match hex::decode(&contract.requester_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                tracing::warn!("Malformed hex in contract.requester_pubkey: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format in database".to_string()),
                });
            }
        };
        let provider_pk = match hex::decode(&contract.provider_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                tracing::warn!("Malformed hex in contract.provider_pubkey: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format in database".to_string()),
                });
            }
        };

        if auth.pubkey != requester_pk && auth.pubkey != provider_pk {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Not authorized to view this invoice".to_string()),
            });
        }

        // Get invoice metadata
        match get_invoice_metadata(&db, &contract_id).await {
            Ok(invoice) => Json(ApiResponse {
                success: true,
                data: Some(invoice),
                error: None,
            }),
            Err(e) => {
                tracing::error!("Failed to get invoice metadata: {:#}", e);
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get invoice metadata: {}", e)),
                })
            }
        }
    }
}

#[derive(poem_openapi::ApiResponse)]
enum InvoicePdfResponse {
    #[oai(status = 200, content_type = "application/pdf")]
    Ok(Binary<Vec<u8>>),
    #[oai(status = 400)]
    BadRequest(Json<ApiResponse<String>>),
    #[oai(status = 403)]
    Forbidden(Json<ApiResponse<String>>),
    #[oai(status = 404)]
    NotFound(Json<ApiResponse<String>>),
    #[oai(status = 500)]
    InternalError(Json<ApiResponse<String>>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_serialization_skips_contract_id() {
        let invoice = Invoice {
            id: 1,
            contract_id: vec![0xab; 32],
            invoice_number: "INV-2025-000001".to_string(),
            invoice_date_ns: 1_700_000_000_000_000_000,
            seller_name: "Decent Cloud Ltd".to_string(),
            seller_address: "Address".to_string(),
            seller_vat_id: None,
            buyer_name: Some("Alice".to_string()),
            buyer_address: Some("123 Main St".to_string()),
            buyer_vat_id: None,
            subtotal_e9s: 100_000_000_000,
            vat_rate_percent: 19,
            vat_amount_e9s: 19_000_000_000,
            total_e9s: 119_000_000_000,
            currency: "EUR".to_string(),
            pdf_generated_at_ns: None,
            created_at_ns: 1_700_000_000_000_000_000,
        };
        let json = serde_json::to_value(&invoice).unwrap();
        assert!(
            json.get("contractId").is_none(),
            "contract_id must be skipped via #[serde(skip)]"
        );
        assert_eq!(json["invoiceNumber"], "INV-2025-000001");
    }

    #[test]
    fn test_invoice_serialization_camel_case_field_names() {
        let invoice = Invoice {
            id: 1,
            contract_id: vec![],
            invoice_number: "INV-2025-000001".to_string(),
            invoice_date_ns: 0,
            seller_name: "Seller".to_string(),
            seller_address: "Addr".to_string(),
            seller_vat_id: Some("DE123456".to_string()),
            buyer_name: None,
            buyer_address: None,
            buyer_vat_id: None,
            subtotal_e9s: 0,
            vat_rate_percent: 0,
            vat_amount_e9s: 0,
            total_e9s: 0,
            currency: "USD".to_string(),
            pdf_generated_at_ns: Some(999),
            created_at_ns: 0,
        };
        let json = serde_json::to_value(&invoice).unwrap();
        // Verify camelCase keys from #[serde(rename_all = "camelCase")]
        assert!(json.get("invoiceDateNs").is_some());
        assert!(json.get("sellerVatId").is_some());
        assert!(json.get("pdfGeneratedAtNs").is_some());
        assert!(json.get("subtotalE9s").is_some());
    }

    #[test]
    fn test_hex_decode_valid_contract_id() {
        let valid_hex = "ab".repeat(32);
        let decoded = hex::decode(&valid_hex).unwrap();
        assert_eq!(decoded.len(), 32);
        assert!(decoded.iter().all(|&b| b == 0xab));
    }

    #[test]
    fn test_hex_decode_invalid_contract_id() {
        let result = hex::decode("not-valid-hex!");
        assert!(result.is_err());
    }

    #[test]
    fn test_api_response_with_invoice_success() {
        let invoice = Invoice {
            id: 42,
            contract_id: vec![],
            invoice_number: "INV-2025-000042".to_string(),
            invoice_date_ns: 0,
            seller_name: "Seller".to_string(),
            seller_address: "Addr".to_string(),
            seller_vat_id: None,
            buyer_name: None,
            buyer_address: None,
            buyer_vat_id: None,
            subtotal_e9s: 50_000_000_000,
            vat_rate_percent: 0,
            vat_amount_e9s: 0,
            total_e9s: 50_000_000_000,
            currency: "USD".to_string(),
            pdf_generated_at_ns: None,
            created_at_ns: 0,
        };
        let resp = ApiResponse {
            success: true,
            data: Some(invoice),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["totalE9s"], 50_000_000_000i64);
        assert_eq!(json["data"]["invoiceNumber"], "INV-2025-000042");
    }
}
