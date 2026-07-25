//! Integration tests for anthropic-proxy.
//!
//! A local axum "mock upstream" captures the request the proxy forwards (method,
//! path+query, headers, body) and returns a canned Anthropic-shaped response. The
//! tests NEVER hit z.ai / Anthropic — the PoC already proved real integration.
//!
//! Coverage (each asserts a distinct, non-overlapping behavior):
//!   * key_injection_and_stripping        — platform key injected, client auth dropped
//!   * path_transparency                  — incoming path+query forwarded verbatim
//!   * usage_recorded_non_streaming       — JSON usage → InMemoryRecorder counts
//!   * streaming_passthrough_and_metering — SSE body returned intact + terminal usage recorded
//!   * metering_failure_is_loud           — a failing recorder surfaces a warning, no silent swallow
//!   * redaction_in_error_body            — proxy-generated error body has no key
//!   * redaction_in_tracing               — captured proxy logs have no key (sensitive headers masked)

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anthropic_proxy::config::Config;
use anthropic_proxy::identity::{IdentityRef, SingleIdentityResolver};
use anthropic_proxy::metering::{InMemoryRecorder, MeteringRecorder, Usage};
use anthropic_proxy::proxy::{build_app, ProxyState};
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::response::Response;
use axum::{extract::State, Router};
use http_body_util::BodyExt;
use tokio::net::TcpListener;
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::*;

// ---------- mock upstream --------------------------------------------------

const PLATFORM_KEY: &str = "sk-platform-secret-4b4ae2c1-AAAA";

const NON_STREAM_BODY: &str = r#"{
  "id":"msg_test","type":"message","role":"assistant","model":"glm-4.5",
  "content":[{"type":"text","text":"Hi!"}],
  "stop_reason":"end_turn","stop_sequence":null,
  "usage":{"input_tokens":42,"output_tokens":7,"cache_read_input_tokens":3,"service_tier":"standard"}
}"#;

const STREAM_BODY: &str = "\
event: message_start
data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"usage\":{\"input_tokens\":0,\"output_tokens\":0}}}

event: content_block_delta
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}

event: message_delta
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":51,\"output_tokens\":9,\"cache_read_input_tokens\":0}}

event: message_stop
data: {\"type\":\"message_stop\"}
";

#[derive(Default, Clone)]
struct MockState {
    received: Arc<Mutex<Vec<ReceivedRequest>>>,
    body: Arc<String>,
    status: Arc<StatusCode>,
    content_type: Arc<String>,
}

#[derive(Default)]
struct ReceivedRequest {
    method: String,
    path_with_query: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct MockUpstream {
    base_url: String,
    received: Arc<Mutex<Vec<ReceivedRequest>>>,
}

async fn spawn_mock_upstream(body: &str, status: StatusCode, content_type: &str) -> MockUpstream {
    let received: Arc<Mutex<Vec<ReceivedRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
    let addr = listener.local_addr().expect("local_addr");
    let state = MockState {
        received: received.clone(),
        body: Arc::new(body.to_string()),
        status: Arc::new(status),
        content_type: Arc::new(content_type.to_string()),
    };
    let app = Router::new().fallback(mock_handler).with_state(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock upstream serve");
    });
    MockUpstream {
        base_url: format!("http://{addr}"),
        received,
    }
}

async fn mock_handler(State(st): State<MockState>, req: Request<Body>) -> Response {
    let (parts, body) = req.into_parts();
    let pwq = parts
        .uri
        .path_and_query()
        .map(|p| p.to_string())
        .unwrap_or_default();
    let mut hdrs: HashMap<String, String> = HashMap::new();
    for (name, value) in parts.headers.iter() {
        if let Ok(s) = value.to_str() {
            hdrs.insert(name.as_str().to_string(), s.to_string());
        }
    }
    let body_bytes = body
        .collect()
        .await
        .map(|b| b.to_bytes().to_vec())
        .unwrap_or_default();
    st.received
        .lock()
        .expect("mock mutex")
        .push(ReceivedRequest {
            method: parts.method.as_str().to_string(),
            path_with_query: pwq,
            headers: hdrs,
            body: body_bytes,
        });

    let mut resp = Response::builder().status(*st.status);
    if let Some(h) = resp.headers_mut() {
        h.insert(
            "content-type",
            HeaderValue::from_str(&st.content_type).expect("ct header"),
        );
    }
    resp.body(Body::from((*st.body).clone()))
        .expect("mock response body")
}

// ---------- helpers --------------------------------------------------------

fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .build()
        .expect("test reqwest client")
}

fn proxy_state(
    upstream: impl Into<String>,
    recorder: Arc<dyn MeteringRecorder>,
) -> ProxyState {
    let config = Config::new(
        "127.0.0.1:0",
        upstream,
        PLATFORM_KEY,
        "2023-06-01",
        "identity-test-1",
    )
    .expect("valid test config");
    ProxyState {
        config: Arc::new(config),
        client: test_client(),
        resolver: Arc::new(SingleIdentityResolver::new(IdentityRef::new("identity-test-1"))),
        recorder,
    }
}

async fn drain_body(resp: Response<Body>) -> Vec<u8> {
    resp.into_body()
        .collect()
        .await
        .expect("collect response body")
        .to_bytes()
        .to_vec()
}

/// A recorder that always fails, to assert the proxy surfaces metering failures.
struct AlwaysFailRecorder;
#[async_trait]
impl MeteringRecorder for AlwaysFailRecorder {
    async fn record_usage(
        &self,
        _identity: &IdentityRef,
        _usage: Usage,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("simulated recorder outage"))
    }
}

// ---------- tracing capture (for redaction-in-tracing test) ----------------

#[derive(Clone)]
struct CaptureMaker {
    buf: Arc<Mutex<Vec<u8>>>,
}

struct CaptureWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Write for CaptureWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buf
            .lock()
            .expect("capture mutex")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CaptureMaker {
    type Writer = CaptureWriter;
    fn make_writer(&'a self) -> Self::Writer {
        CaptureWriter {
            buf: self.buf.clone(),
        }
    }
}

/// Install a thread-local tracing subscriber that captures all log output into a
/// shared buffer. Returns the buffer and a guard that must be held for the duration.
fn install_capture(
) -> (
    Arc<Mutex<Vec<u8>>>,
    tracing::dispatcher::DefaultGuard,
) {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let maker = CaptureMaker { buf: buf.clone() };
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::DEBUG)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(maker)
                .with_ansi(false),
        );
    let guard = tracing::dispatcher::set_default(&tracing::Dispatch::new(subscriber));
    (buf, guard)
}

// ---------- tests ----------------------------------------------------------

#[tokio::test]
async fn key_injection_and_stripping() {
    let mock = spawn_mock_upstream(NON_STREAM_BODY, StatusCode::OK, "application/json").await;
    let recorder = Arc::new(InMemoryRecorder::new());
    let app = build_app(proxy_state(mock.base_url.clone(), recorder));

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("x-api-key", "evil-client-key-1234")
        .header("authorization", "Bearer evil-bearer-xyz")
        .header("anthropic-version", "2099-99-99")
        .body(Body::from(r#"{"model":"glm-4.5","max_tokens":1,"messages":[]}"#))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    drop(drain_body(resp).await);

    let received = mock.received.lock().expect("mock mutex");
    let last = received.last().expect("mock captured a request");
    // Platform key injected.
    assert_eq!(
        last.headers.get("x-api-key"),
        Some(&PLATFORM_KEY.to_string()),
        "platform key must be injected"
    );
    // Client auth stripped (never trusted).
    assert!(
        !last.headers.contains_key("authorization"),
        "client Authorization header must be stripped"
    );
    assert!(
        !last.headers.values().any(|v| v == "evil-client-key-1234"),
        "client x-api-key value must not appear anywhere in forwarded headers"
    );
    // anthropic-version injected == configured (NOT the client's 2099-99-99).
    assert_eq!(
        last.headers.get("anthropic-version"),
        Some(&"2023-06-01".to_string()),
        "anthropic-version must be the configured one, not the client's"
    );
}

#[tokio::test]
async fn path_transparency() {
    let mock = spawn_mock_upstream(NON_STREAM_BODY, StatusCode::OK, "application/json").await;
    let app = build_app(proxy_state(mock.base_url.clone(), Arc::new(InMemoryRecorder::new())));

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages?beta=true&stream=false")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"model":"glm-4.5","messages":[]}"#))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    drop(drain_body(resp).await);

    let received = mock.received.lock().expect("mock mutex");
    let last = received.last().expect("captured");
    assert_eq!(last.method, "POST");
    assert_eq!(
        last.path_with_query, "/v1/messages?beta=true&stream=false",
        "incoming path + query must be forwarded verbatim"
    );
    assert_eq!(
        last.body,
        br#"{"model":"glm-4.5","messages":[]}"#,
        "request body must be forwarded to upstream unchanged"
    );
}

#[tokio::test]
async fn usage_recorded_non_streaming() {
    let mock = spawn_mock_upstream(NON_STREAM_BODY, StatusCode::OK, "application/json").await;
    let recorder = Arc::new(InMemoryRecorder::new());
    let app = build_app(proxy_state(mock.base_url.clone(), recorder.clone()));

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = drain_body(resp).await;
    // Response body is returned to the caller unchanged.
    assert!(String::from_utf8_lossy(&body).contains(r#""id":"msg_test""#));

    let total = recorder.total_for("identity-test-1");
    assert_eq!(total.input_tokens, 42);
    assert_eq!(total.output_tokens, 7);
    assert_eq!(total.cache_read_input_tokens, 3);
    assert_eq!(total.cache_creation_input_tokens, 0);
    assert_eq!(recorder.snapshots().len(), 1, "exactly one record");
}

#[tokio::test]
async fn streaming_passthrough_and_metering() {
    let mock = spawn_mock_upstream(STREAM_BODY, StatusCode::OK, "text/event-stream").await;
    let recorder = Arc::new(InMemoryRecorder::new());
    let app = build_app(proxy_state(mock.base_url.clone(), recorder.clone()));

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = String::from_utf8(drain_body(resp).await).expect("utf8");
    // SSE body returned to the caller intact, including the terminal usage event.
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: message_delta"));
    assert!(body.contains("\"output_tokens\":9"));

    // The recorder runs on a detached task when the stream ends; give it time.
    wait_for(|| recorder.snapshots().len() == 1, Duration::from_millis(500)).await;

    let total = recorder.total_for("identity-test-1");
    assert_eq!(
        total,
        Usage {
            input_tokens: 51,
            output_tokens: 9,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        },
        "terminal message_delta usage must be recorded"
    );
}

#[tokio::test]
async fn streaming_without_message_delta_records_nothing() {
    // An SSE stream that never emits message_delta must not produce a usage record
    // (and must not crash); the response body is still returned to the caller.
    let sse_no_delta = "\
event: message_start
data: {\"type\":\"message_start\"}

event: message_stop
data: {\"type\":\"message_stop\"}
";
    let mock = spawn_mock_upstream(sse_no_delta, StatusCode::OK, "text/event-stream").await;
    let recorder = Arc::new(InMemoryRecorder::new());
    let app = build_app(proxy_state(mock.base_url.clone(), recorder.clone()));

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    drop(drain_body(resp).await);

    // Allow any detached work to settle, then assert no usage was recorded.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(recorder.snapshots().is_empty());
}

#[tokio::test]
async fn metering_failure_is_loud_no_silent_swallow() {
    let mock = spawn_mock_upstream(NON_STREAM_BODY, StatusCode::OK, "application/json").await;
    let recorder: Arc<dyn MeteringRecorder> = Arc::new(AlwaysFailRecorder);
    let app = build_app(proxy_state(mock.base_url.clone(), recorder));

    let (buf, _guard) = install_capture();

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    // Metering failure must NOT break the client response.
    assert_eq!(resp.status(), StatusCode::OK);
    drop(drain_body(resp).await);

    let captured = String::from_utf8_lossy(&buf.lock().expect("capture").clone()).to_string();
    assert!(
        captured.contains("metering record_usage failed"),
        "expected a loud metering-failure warning, got: {captured}"
    );
    assert!(
        captured.contains("simulated recorder outage"),
        "the underlying failure detail must be logged, not swallowed"
    );
}

#[tokio::test]
async fn redaction_in_error_body() {
    // Point the proxy at an unreachable upstream so it generates its OWN error.
    let app = build_app(proxy_state(
        "http://127.0.0.1:1", // port 1 is closed → connection refused, fast
        Arc::new(InMemoryRecorder::new()),
    ));

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("x-api-key", PLATFORM_KEY) // same as platform here; should not appear in error body
        .body(Body::from("{}"))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_GATEWAY,
        "proxy-generated forwarding failure must surface as 502"
    );
    let body = String::from_utf8(drain_body(resp).await).expect("utf8");
    assert!(
        !body.contains(PLATFORM_KEY),
        "platform key MUST NOT appear in the proxy error body: {body}"
    );
    assert!(
        !body.contains("4b4ae2c1") && !body.contains("AAAA"),
        "no substring of the key may appear in the error body: {body}"
    );
}

#[tokio::test]
async fn redaction_in_tracing() {
    let mock = spawn_mock_upstream(NON_STREAM_BODY, StatusCode::OK, "application/json").await;
    let app = build_app(proxy_state(
        mock.base_url.clone(),
        Arc::new(InMemoryRecorder::new()),
    ));

    let (buf, _guard) = install_capture();

    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("x-api-key", PLATFORM_KEY)
        .header("authorization", "Bearer evil")
        .body(Body::from("{}"))
        .expect("build request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    drop(drain_body(resp).await);

    let captured = String::from_utf8_lossy(&buf.lock().expect("capture").clone()).to_string();
    // The proxy logs the outgoing headers at DEBUG; the key + client auth must be masked.
    assert!(
        !captured.contains(PLATFORM_KEY),
        "platform key leaked into tracing: {captured}"
    );
    assert!(
        !captured.contains("Bearer evil"),
        "client auth leaked into tracing: {captured}"
    );
    // Sensitive header names appear, but only with masked values.
    assert!(
        captured.contains("x-api-key: [REDACTED]"),
        "x-api-key should be masked in tracing: {captured}"
    );
}

// ---------- tiny async wait helper -----------------------------------------

async fn wait_for<F>(mut cond: F, timeout: Duration)
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if cond() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
