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
