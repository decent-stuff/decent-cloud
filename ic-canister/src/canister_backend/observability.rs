use crate::canister_backend::http_types::{HttpRequest, HttpResponse};
use dcc_common::cache_transactions::RecentCache;
use ic_cdk::api::time;
use ic_metrics_encoder::MetricsEncoder;
use ledger_map::{export_debug, export_error, export_info, export_warn};
use serde_bytes::ByteBuf;

fn encode_metrics(w: &mut MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    w.encode_gauge(
        "recent_transaction_cache_entries_count",
        RecentCache::get_num_entries() as f64,
        "Number of entries in the cache of recent transactions",
    )?;
    Ok(())
}

/// Returns metrics as an ascii-encoded string, in the Prometheus exposition format.
pub fn metrics() -> Result<Vec<u8>, std::io::Error> {
    let mut writer = MetricsEncoder::new(vec![], time() as i64 / 1_000_000);
    encode_metrics(&mut writer)?;
    Ok(writer.into_inner())
}

fn serve_metrics() -> HttpResponse {
    match metrics() {
        Ok(body) => {
            let headers = vec![
                (
                    "Content-Type".to_string(),
                    "text/plain; version=0.0.4".to_string(),
                ),
                ("Content-Length".to_string(), body.len().to_string()),
            ];
            HttpResponse {
                status_code: 200,
                headers,
                body: ByteBuf::from(body),
                upgrade: None,
            }
        }
        Err(err) => HttpResponse {
            status_code: 500,
            headers: vec![],
            body: ByteBuf::from(format!("Failed to encode metrics: {err}")),
            upgrade: None,
        },
    }
}

/// Regular HTTP requests will be proxied to this endpoint, by icx-proxy locally, or by boundary nodes on the mainnet.
/// See [../README.md] for details.
pub fn _http_request(request: HttpRequest) -> HttpResponse {
    let path = match request.url.find('?') {
        None => &request.url[..],
        Some(index) => &request.url[..index],
    };

    match path {
        "/metrics" => serve_metrics(),
        "/logs" => {
            let body = match serde_json::to_string_pretty(&export_info()) {
                Ok(json) => json,
                Err(e) => {
                    return HttpResponse {
                        status_code: 500,
                        headers: vec![],
                        body: ByteBuf::from(format!("Failed to serialize logs: {:?}", e)),
                        upgrade: None,
                    }
                }
            };
            HttpResponse {
                status_code: 200,
                body: ByteBuf::from(body),
                ..Default::default()
            }
        }
        _ => HttpResponse {
            status_code: 404u16,
            body: ByteBuf::from("not_found"),
            ..Default::default()
        },
    }
}

pub fn _get_logs_debug() -> Result<String, String> {
    serde_json::to_string_pretty(&export_debug()).map_err(|e| format!("{:?}", e))
}

pub fn _get_logs_info() -> Result<String, String> {
    serde_json::to_string_pretty(&export_info()).map_err(|e| format!("{:?}", e))
}

pub fn _get_logs_warn() -> Result<String, String> {
    serde_json::to_string_pretty(&export_warn()).map_err(|e| format!("{:?}", e))
}

pub fn _get_logs_error() -> Result<String, String> {
    serde_json::to_string_pretty(&export_error()).map_err(|e| format!("{:?}", e))
}
