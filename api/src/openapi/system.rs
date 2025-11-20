use super::common::{ApiTags, HealthResponse};
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;

pub struct SystemApi;

#[OpenApi]
impl SystemApi {
    /// Health check endpoint
    ///
    /// Returns the health status of the API server
    #[oai(path = "/health", method = "get", tag = "ApiTags::System")]
    async fn health(&self) -> Json<HealthResponse> {
        let environment =
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        Json(HealthResponse {
            success: true,
            message: "Decent Cloud API is running".to_string(),
            environment,
        })
    }
}
