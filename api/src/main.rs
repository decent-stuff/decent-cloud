use poem::{
    handler, listener::TcpListener, middleware::Cors, post, web::Json, EndpointExt, IntoResponse,
    Response, Route, Server,
};
use serde::{Deserialize, Serialize};
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    success: bool,
    message: String,
    environment: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CanisterRequest {
    args: Vec<serde_json::Value>,
}

#[handler]
async fn health() -> Json<HealthResponse> {
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    Json(HealthResponse {
        success: true,
        message: "Decent Cloud API is running".to_string(),
        environment,
    })
}

#[handler]
async fn canister_proxy(
    method: poem::web::Path<String>,
    Json(req): Json<CanisterRequest>,
) -> Response {
    tracing::info!("Proxying canister method: {}", method.0);
    tracing::debug!("Request args: {:?}", req.args);

    // TODO: Implement actual ICP canister call
    // For now, return a placeholder response
    let response: ApiResponse<String> = ApiResponse {
        success: false,
        data: None,
        error: Some(format!("Canister proxy not yet implemented for method: {}", method.0)),
    };

    Response::builder()
        .content_type("application/json")
        .body(serde_json::to_string(&response).unwrap())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Starting Decent Cloud API server on {}", addr);

    let app = Route::new()
        .at("/api/v1/health", poem::get(health))
        .at("/api/v1/canister/:method", post(canister_proxy))
        .with(Cors::new());

    Server::new(TcpListener::bind(&addr))
        .run(app)
        .await
}
