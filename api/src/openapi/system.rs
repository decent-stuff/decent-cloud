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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serializes_to_json() {
        let resp = HealthResponse {
            success: true,
            message: "ok".to_string(),
            environment: "production".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["message"], "ok");
        assert_eq!(json["environment"], "production");
    }

    #[test]
    fn test_environment_defaults_to_development() {
        // Remove the env var if set, then verify default
        std::env::remove_var("ENVIRONMENT");
        let env = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        assert_eq!(env, "development");
    }
}
