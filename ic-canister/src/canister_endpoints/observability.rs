use crate::canister_backend::http_types::{HttpRequest, HttpResponse};
use crate::canister_backend::observability::{
    _get_logs_debug, _get_logs_error, _get_logs_info, _get_logs_warn, _http_request,
};

/// Regular HTTP requests will be proxied to this endpoint, by icx-proxy locally, or by boundary nodes on the mainnet.
/// See [../README.md] for details.
#[ic_cdk::query(hidden = true)]
fn http_request(request: HttpRequest) -> HttpResponse {
    _http_request(request)
}

#[ic_cdk::query]
fn get_logs_debug() -> Result<String, String> {
    _get_logs_debug()
}

#[ic_cdk::query]
fn get_logs_info() -> Result<String, String> {
    _get_logs_info()
}

#[ic_cdk::query]
fn get_logs_warn() -> Result<String, String> {
    _get_logs_warn()
}

#[ic_cdk::query]
fn get_logs_error() -> Result<String, String> {
    _get_logs_error()
}
