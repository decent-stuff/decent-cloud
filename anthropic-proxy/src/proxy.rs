//! The reverse-proxy handler (axum 0.7 / hyper 1.x).
//!
//! Flow for every request:
//!   1. resolve identity (single for beta; trait for future multi-tenant);
//!   2. build clean outbound headers — strip ALL client auth + hop-by-hop headers,
//!      inject the platform `x-api-key` and `anthropic-version`;
//!   3. forward `{upstream}{path?query}` verbatim (path-transparent);
//!   4. stream the upstream response back unchanged;
//!   5. capture token usage:
//!        - non-streaming JSON → parse top-level `usage`;
//!        - streaming SSE (`text/event-stream`) → tee the byte stream, parse the
//!          terminal `message_delta` usage, record after the stream ends;
//!   6. record usage via the `MeteringRecorder`. Recording is fire-and-forget on a
//!      detached task so a slow recorder never stalls the client response.
//!
//! Every error path goes through `redact_secret` so the configured key can never
//! leak via an error message or response body.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{extract::State, Router};
use bytes::Bytes;
use futures::{ready, Stream, StreamExt};
use http_body_util::BodyExt;

use crate::config::Config;
use crate::identity::{IdentityRef, SharedResolver};
use crate::metering::{parse_response_usage, SharedRecorder, StreamUsageCollector, Usage};
use crate::redact::{redact_secret, redacted_headers};

/// Hop-by-hop headers (RFC 7230 §6.1) plus headers reqwest recomputes and the auth
/// headers we MUST strip so a client can never spoof or shadow the platform key.
const STRIPPED_REQUEST_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "host",
    "content-length",
    // auth — the proxy's configured key wins; client headers are never trusted.
    "x-api-key",
    "authorization",
    "anthropic-version",
];

const HOP_BY_HOP_RESPONSE_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

#[derive(Clone)]
pub struct ProxyState {
    pub config: Arc<Config>,
    pub client: reqwest::Client,
    pub resolver: SharedResolver,
    pub recorder: SharedRecorder,
}

/// Build the proxy `Router` with the catch-all forwarding handler. Exposed for tests
/// and for `main`.
pub fn build_app(state: ProxyState) -> Router {
    Router::new().fallback(proxy_handler).with_state(state)
}

/// Catch-all handler: any method, any path — forwarded path-transparently to upstream.
pub async fn proxy_handler(State(state): State<ProxyState>, req: Request<Body>) -> Response {
    match forward(&state, req).await {
        Ok(resp) => resp,
        Err(err) => {
            // Redact the configured key out of any error context before surfacing.
            let msg = redact_secret(&format!("{err:#}"), &state.config.api_key);
            tracing::error!(error = %msg, "proxy request failed");
            (StatusCode::BAD_GATEWAY, msg).into_response()
        }
    }
}

async fn forward(state: &ProxyState, req: Request<Body>) -> Result<Response, anyhow::Error> {
    let identity = state.resolver.resolve().await?;
    let (parts, body) = req.into_parts();
    let path_query = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");

    // Outbound headers: drop everything we cannot trust, keep the rest verbatim.
    let mut out_headers = HeaderMap::with_capacity(parts.headers.len() + 2);
    for (name, value) in &parts.headers {
        if STRIPPED_REQUEST_HEADERS.contains(&name.as_str()) {
            continue;
        }
        out_headers.append(name, value.clone());
    }
    out_headers.insert("x-api-key", HeaderValue::from_str(&state.config.api_key)?);
    out_headers.insert(
        "anthropic-version",
        HeaderValue::from_str(&state.config.anthropic_version)?,
    );

    let url = format!("{}{}", state.config.upstream, path_query);
    tracing::debug!(
        identity = %identity.id,
        method = %parts.method,
        url = %redact_secret(&url, &state.config.api_key),
        headers = %redacted_headers(&out_headers, &state.config.api_key),
        "forwarding request to upstream"
    );

    // Buffer the (small, JSON) request body. True request-body streaming is YAGNI for
    // the Messages API and avoids holding the upstream connection during slow upload.
    let body_bytes = body.collect().await?.to_bytes();

    let upstream = state
        .client
        .request(parts.method, &url)
        .headers(out_headers)
        .body(body_bytes)
        .send()
        .await?;

    let status = upstream.status();
    let resp_headers = upstream.headers().clone();
    let content_type = resp_headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if content_type.contains("text/event-stream") {
        Ok(stream_response(upstream.bytes_stream(), status, resp_headers, state, identity))
    } else {
        Ok(buffered_response(upstream, status, resp_headers, state, &identity).await?)
    }
}

fn stream_response(
    upstream_stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
    status: StatusCode,
    mut resp_headers: HeaderMap,
    state: &ProxyState,
    identity: IdentityRef,
) -> Response {
    strip_response_headers(&mut resp_headers);

    // Map to a boxed stream of Result<Bytes, BoxErr> (BoxStream is Unpin) so the tee
    // can own it and poll it without pin-fighting at the call site.
    let upstream_stream = upstream_stream
        .map(|res| res.map_err(|e| -> BoxErr { Box::new(e) }))
        .boxed();
    let tee = UsageTee::new(
        upstream_stream,
        state.recorder.clone(),
        identity,
        state.config.api_key.clone(),
    );
    let body = Body::from_stream(tee);

    let mut resp = Response::builder().status(status);
    if let Some(h) = resp.headers_mut() {
        *h = resp_headers;
    }
    resp.body(body).expect("status + headers always build a valid Response")
}

async fn buffered_response(
    upstream: reqwest::Response,
    status: StatusCode,
    mut resp_headers: HeaderMap,
    state: &ProxyState,
    identity: &IdentityRef,
) -> Result<Response, anyhow::Error> {
    let body = upstream.bytes().await?;
    strip_response_headers(&mut resp_headers);

    // Parse + record usage for non-streaming responses. Failure to meter is logged
    // loudly but never breaks the response (the client still gets the body).
    if let Some(usage) = parse_response_usage(&String::from_utf8_lossy(&body)) {
        record_usage(state, identity.clone(), usage, "non-stream").await;
    } else {
        tracing::debug!(
            identity = %identity.id,
            status = %status.as_u16(),
            "non-streaming response had no parseable usage (likely an error body)"
        );
    }

    let mut resp = Response::builder().status(status);
    if let Some(h) = resp.headers_mut() {
        *h = resp_headers;
    }
    Ok(resp.body(Body::from(body))?)
}

/// Hand usage off to the recorder. The recorder owns its own success side-effect
/// (InMemoryRecorder stores; LoggingRecorder logs; the future DB recorder persists).
/// We only surface a failure here — per "never silently ignore failures".
async fn record_usage(state: &ProxyState, identity: IdentityRef, usage: Usage, kind: &str) {
    if let Err(e) = state.recorder.record_usage(&identity, usage).await {
        let msg = redact_secret(&format!("{e:#}"), &state.config.api_key);
        tracing::warn!(
            error = %msg,
            identity = %identity.id,
            kind,
            "metering record_usage failed — usage for this response was NOT recorded"
        );
    }
}

fn strip_response_headers(headers: &mut HeaderMap) {
    for h in HOP_BY_HOP_RESPONSE_HEADERS {
        headers.remove(*h);
    }
}

type BoxErr = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Wraps the upstream byte stream: passes every chunk through to the client unchanged
/// while feeding bytes into a `StreamUsageCollector`. When the stream ends, records
/// the captured usage on a detached task.
struct UsageTee<S> {
    inner: S,
    collector: StreamUsageCollector,
    recorder: SharedRecorder,
    identity: IdentityRef,
    api_key: String,
}

impl<S> UsageTee<S>
where
    S: Stream<Item = Result<Bytes, BoxErr>> + Unpin,
{
    fn new(inner: S, recorder: SharedRecorder, identity: IdentityRef, api_key: String) -> Self {
        Self {
            inner,
            collector: StreamUsageCollector::new(),
            recorder,
            identity,
            api_key,
        }
    }
}

impl<S> Stream for UsageTee<S>
where
    S: Stream<Item = Result<Bytes, BoxErr>> + Unpin,
{
    type Item = Result<Bytes, BoxErr>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // UsageTee<S> is Unpin whenever S: Unpin (all other fields are Unpin), and the
        // impl bound guarantees S: Unpin, so self.get_mut() is sound here.
        let this = self.get_mut();
        match ready!(Pin::new(&mut this.inner).poll_next(cx)) {
            None => {
                let usage = this.collector.finish();
                if let Some(u) = usage {
                    let recorder = this.recorder.clone();
                    let identity = this.identity.clone();
                    let api_key = this.api_key.clone();
                    // Detached: never block the client response on the recorder.
                    tokio::spawn(async move {
                        record_usage_spawned(recorder, identity, u, api_key).await;
                    });
                } else {
                    tracing::debug!(
                        identity = %this.identity.id,
                        "stream ended with no message_delta usage"
                    );
                }
                Poll::Ready(None)
            }
            Some(Ok(chunk)) => {
                this.collector.push_bytes(&chunk);
                Poll::Ready(Some(Ok(chunk)))
            }
            Some(Err(e)) => {
                let redacted = redact_secret(&format!("{e}"), &this.api_key);
                tracing::warn!(
                    identity = %this.identity.id,
                    error = %redacted,
                    "upstream stream error"
                );
                Poll::Ready(Some(Err(e)))
            }
        }
    }
}

async fn record_usage_spawned(
    recorder: SharedRecorder,
    identity: IdentityRef,
    usage: Usage,
    api_key: String,
) {
    if let Err(e) = recorder.record_usage(&identity, usage).await {
        let msg = redact_secret(&format!("{e:#}"), &api_key);
        tracing::warn!(
            error = %msg,
            identity = %identity.id,
            "metering record_usage failed (stream) — usage for this response was NOT recorded"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripped_header_list_covers_auth_and_hop_by_hop() {
        for h in [
            "x-api-key",
            "authorization",
            "anthropic-version",
            "host",
            "content-length",
            "transfer-encoding",
            "connection",
        ] {
            assert!(STRIPPED_REQUEST_HEADERS.contains(&h), "missing {h}");
        }
    }

    #[test]
    fn hop_by_hop_response_list_is_non_empty() {
        assert!(HOP_BY_HOP_RESPONSE_HEADERS.contains(&"transfer-encoding"));
        assert!(HOP_BY_HOP_RESPONSE_HEADERS.contains(&"connection"));
    }
}
