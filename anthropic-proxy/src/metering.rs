//! Token-usage metering (spec section G — handoff to #415/#416).
//!
//! Parses the Anthropic Messages-API `usage` object from both non-streaming JSON
//! responses and streaming SSE responses, and records it per identity via a trait.
//! The DB-backed recorder that writes `agent_runs.claude_{input,output}_tokens` is
//! #415/#416's job; we ship an in-memory recorder (tests + dev) and a logging recorder.

use std::sync::{Arc, Mutex};

use serde::Deserialize;

use crate::identity::IdentityRef;

/// Token usage parsed from an Anthropic Messages-API response `usage` object.
///
/// Unknown fields in the upstream `usage` object are ignored (z.ai adds
/// `server_tool_use` and `service_tier`; Anthropic may add others). Missing token
/// fields default to `0` so both upstream shapes parse uniformly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

impl Usage {
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Records token usage for an identity. The proxy calls this after parsing each
/// response's usage. Implementations MUST be cheap to clone (wrapped behind `Arc`).
#[async_trait::async_trait]
pub trait MeteringRecorder: Send + Sync {
    async fn record_usage(&self, identity: &IdentityRef, usage: Usage) -> anyhow::Result<()>;
}

pub type SharedRecorder = Arc<dyn MeteringRecorder>;

/// Records usage in memory (tests + dev). NOT durable — never used in production.
#[derive(Debug, Default)]
pub struct InMemoryRecorder {
    records: Mutex<Vec<(String, Usage)>>,
}

impl InMemoryRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// All recorded `(identity_id, usage)` pairs, in insertion order.
    pub fn snapshots(&self) -> Vec<(String, Usage)> {
        self.records
            .lock()
            .expect("InMemoryRecorder mutex poisoned")
            .iter()
            .map(|(id, u)| (id.clone(), *u))
            .collect()
    }

    /// Aggregated usage for one identity across all recorded calls.
    pub fn total_for(&self, id: &str) -> Usage {
        self.records
            .lock()
            .expect("InMemoryRecorder mutex poisoned")
            .iter()
            .filter(|(rid, _)| rid == id)
            .fold(Usage::default(), |mut acc, (_, u)| {
                acc.input_tokens += u.input_tokens;
                acc.output_tokens += u.output_tokens;
                acc.cache_creation_input_tokens += u.cache_creation_input_tokens;
                acc.cache_read_input_tokens += u.cache_read_input_tokens;
                acc
            })
    }
}

#[async_trait::async_trait]
impl MeteringRecorder for InMemoryRecorder {
    async fn record_usage(&self, identity: &IdentityRef, usage: Usage) -> anyhow::Result<()> {
        let mut guard = self
            .records
            .lock()
            .map_err(|e| anyhow::anyhow!("InMemoryRecorder mutex poisoned: {e}"))?;
        guard.push((identity.id.clone(), usage));
        Ok(())
    }
}

/// Records usage by emitting a structured log line. For dev/staging inspection
/// where a DB recorder (#415) is not yet wired.
#[derive(Debug, Clone)]
pub struct LoggingRecorder;

#[async_trait::async_trait]
impl MeteringRecorder for LoggingRecorder {
    async fn record_usage(&self, identity: &IdentityRef, usage: Usage) -> anyhow::Result<()> {
        tracing::info!(
            identity = %identity.id,
            input_tokens = usage.input_tokens,
            output_tokens = usage.output_tokens,
            cache_creation_input_tokens = usage.cache_creation_input_tokens,
            cache_read_input_tokens = usage.cache_read_input_tokens,
            "[meter] recorded usage"
        );
        Ok(())
    }
}

// ----- Usage parsing -------------------------------------------------------
//
// Two shapes:
//   * non-streaming JSON: top-level `usage` object in the response body.
//   * streaming SSE: terminal `event: message_delta` whose `data.usage` carries the
//     final counts (verified empirically against z.ai's Anthropic-compatible API).

#[derive(Deserialize)]
struct MessageResponse {
    #[serde(default)]
    usage: Option<Usage>,
}

/// Parse the top-level `usage` from a non-streaming Messages JSON response body.
/// Returns `None` when the body is not valid JSON or carries no `usage` field.
pub fn parse_response_usage(body: &str) -> Option<Usage> {
    serde_json::from_str::<MessageResponse>(body).ok().and_then(|r| r.usage)
}

/// Parse the terminal `message_delta` usage from a COMPLETE Anthropic SSE body.
/// Returns the last `message_delta` usage seen (Anthropic emits exactly one).
pub fn parse_stream_usage(sse: &str) -> Option<Usage> {
    let mut last: Option<Usage> = None;
    for line in sse.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(u) = parse_data_line_usage(line.as_bytes()) {
            last = Some(u);
        }
    }
    last
}

#[derive(Deserialize)]
struct StreamEvent {
    #[serde(default, rename = "type")]
    ty: String,
    #[serde(default)]
    usage: Option<Usage>,
}

/// Parse a single SSE `data:` line for a `message_delta` usage object.
/// Operates on raw bytes so it is robust to multi-byte UTF-8 split across stream
/// chunks (only the ASCII structure/keys we care about are inspected).
fn parse_data_line_usage(line: &[u8]) -> Option<Usage> {
    let trimmed = trim_ascii_leading(line);
    let payload = trimmed.strip_prefix(b"data:")?;
    let payload = trim_ascii_leading(payload);
    if payload.is_empty() || payload == b"[DONE]" {
        return None;
    }
    let ev: StreamEvent = serde_json::from_slice(payload).ok()?;
    if ev.ty == "message_delta" {
        ev.usage
    } else {
        None
    }
}

fn trim_ascii_leading(b: &[u8]) -> &[u8] {
    let i = b
        .iter()
        .position(|&c| c != b' ' && c != b'\t')
        .unwrap_or(b.len());
    &b[i..]
}

/// Incremental collector for live SSE streams: feed `&[u8]` chunks as they arrive;
/// the buffer holds a partial trailing line across chunk boundaries, and only
/// complete `\n`-terminated lines are parsed. Call `finish` at stream end.
#[derive(Default)]
pub struct StreamUsageCollector {
    buf: Vec<u8>,
    last: Option<Usage>,
}

impl StreamUsageCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        while let Some(idx) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=idx).collect();
            // strip a trailing CR if present
            let line = if line.last() == Some(&b'\r') {
                &line[..line.len() - 1]
            } else {
                &line[..]
            };
            if let Some(u) = parse_data_line_usage(line) {
                self.last = Some(u);
            }
        }
    }

    /// Returns the latest captured `message_delta` usage, or `None` if none was seen.
    /// The collector is reset, so it can be reused for another stream if desired.
    pub fn finish(&mut self) -> Option<Usage> {
        self.buf.clear();
        self.last.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nonstreaming_usage_full_fixture() {
        let body = r#"{
            "id":"msg_1","type":"message","role":"assistant",
            "content":[{"type":"text","text":"Hi!"}],
            "usage":{"input_tokens":17,"output_tokens":4,"cache_read_input_tokens":0}
        }"#;
        let u = parse_response_usage(body).expect("usage present");
        assert_eq!(u, Usage { input_tokens: 17, output_tokens: 4, cache_creation_input_tokens: 0, cache_read_input_tokens: 0 });
    }

    #[test]
    fn parse_nonstreaming_usage_ignores_unknown_fields() {
        // z.ai adds server_tool_use + service_tier; parser must tolerate them.
        let body = r#"{"usage":{"input_tokens":7,"output_tokens":16,
            "cache_read_input_tokens":0,
            "server_tool_use":{"web_search_requests":0},"service_tier":"standard"}}"#;
        let u = parse_response_usage(body).expect("usage present");
        assert_eq!(u.input_tokens, 7);
        assert_eq!(u.output_tokens, 16);
        assert_eq!(u.cache_creation_input_tokens, 0);
    }

    #[test]
    fn parse_nonstreaming_usage_missing_field_defaults_to_zero() {
        let body = r#"{"usage":{"input_tokens":5}}"#;
        let u = parse_response_usage(body).expect("usage present");
        assert_eq!(u.input_tokens, 5);
        assert_eq!(u.output_tokens, 0);
        assert_eq!(u.cache_creation_input_tokens, 0);
        assert_eq!(u.cache_read_input_tokens, 0);
    }

    #[test]
    fn parse_nonstreaming_usage_none_for_invalid_json() {
        assert!(parse_response_usage("not json").is_none());
        assert!(parse_response_usage(r#"{"foo":"bar"}"#).is_none());
    }

    const STREAM_FIXTURE: &str = "\
event: message_start
data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":0,\"output_tokens\":0}}}

event: content_block_delta
data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hi\"}}

event: message_delta
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"input_tokens\":10,\"output_tokens\":12,\"cache_read_input_tokens\":0,\"service_tier\":\"standard\"}}

event: message_stop
data: {\"type\":\"message_stop\"}
";

    #[test]
    fn parse_stream_usage_returns_terminal_message_delta() {
        let u = parse_stream_usage(STREAM_FIXTURE).expect("usage present");
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 12);
    }

    #[test]
    fn parse_stream_usage_none_without_message_delta() {
        let sse = "\
event: message_start
data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":0,\"output_tokens\":0}}}

event: message_stop
data: {\"type\":\"message_stop\"}
";
        assert!(parse_stream_usage(sse).is_none());
    }

    #[test]
    fn collector_handles_chunks_split_across_line_boundary() {
        let mut c = StreamUsageCollector::new();
        // Split the message_delta data line across three pushes, mid-JSON.
        c.push_bytes(b"event: message_delta\ndata: {\"type\":\"mes");
        c.push_bytes(b"sage_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"");
        c.push_bytes(b"usage\":{\"input_tokens\":99,\"output_tokens\":7}}\n\nevent: message_stop\n");
        let u = c.finish().expect("usage collected");
        assert_eq!(u.input_tokens, 99);
        assert_eq!(u.output_tokens, 7);
    }

    #[test]
    fn collector_skips_done_marker_and_non_delta_lines() {
        let mut c = StreamUsageCollector::new();
        c.push_bytes(b"data: [DONE]\n");
        c.push_bytes(b"event: ping\ndata: {\"type\":\"ping\"}\n");
        assert!(c.finish().is_none());
    }

    #[test]
    fn in_memory_records_and_aggregates() {
        let r = InMemoryRecorder::new();
        let id = IdentityRef::new("cust-1");
        // Block on a tiny runtime via tokio.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            r.record_usage(&id, Usage { input_tokens: 10, output_tokens: 5, ..Default::default() })
                .await
                .unwrap();
            r.record_usage(&id, Usage { input_tokens: 3, output_tokens: 2, ..Default::default() })
                .await
                .unwrap();
        });
        let total = r.total_for("cust-1");
        assert_eq!(total.input_tokens, 13);
        assert_eq!(total.output_tokens, 7);
        assert_eq!(r.snapshots().len(), 2);
        assert_eq!(r.total_for("other"), Usage::default());
    }
}
