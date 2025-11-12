use poem::{Endpoint, IntoResponse, Middleware, Request, Response};
use std::time::Instant;

/// Middleware that logs HTTP requests with method, path, status, duration, and client IP
pub struct RequestLogging;

impl<E: Endpoint> Middleware<E> for RequestLogging {
    type Output = RequestLoggingEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        RequestLoggingEndpoint { inner: ep }
    }
}

pub struct RequestLoggingEndpoint<E> {
    inner: E,
}

impl<E: Endpoint> Endpoint for RequestLoggingEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let client_ip = req
            .remote_addr()
            .as_socket_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let response = self.inner.call(req).await;

        let duration = start.elapsed();
        let duration_ms = duration.as_millis();

        match response {
            Ok(resp) => {
                let resp = resp.into_response();
                let status = resp.status();

                if status.is_success() {
                    tracing::info!(
                        method = %method,
                        path = %path,
                        status = %status.as_u16(),
                        duration_ms = %duration_ms,
                        client_ip = %client_ip,
                        "request completed"
                    );
                } else if status.is_client_error() || status.is_server_error() {
                    tracing::warn!(
                        method = %method,
                        path = %path,
                        status = %status.as_u16(),
                        duration_ms = %duration_ms,
                        client_ip = %client_ip,
                        "request failed"
                    );
                } else {
                    tracing::debug!(
                        method = %method,
                        path = %path,
                        status = %status.as_u16(),
                        duration_ms = %duration_ms,
                        client_ip = %client_ip,
                        "request completed"
                    );
                }

                Ok(resp)
            }
            Err(err) => {
                let status = err.status();
                tracing::error!(
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration_ms = %duration_ms,
                    client_ip = %client_ip,
                    error = %err,
                    "request error"
                );
                Err(err)
            }
        }
    }
}
