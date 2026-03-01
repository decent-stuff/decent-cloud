use candid::Func;
use ic_cdk::management_canister::{
    http_request, HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult, TransformArgs,
    TransformContext, TransformFunc,
};
use ledger_map::{error, info};
use serde::Deserialize;

const KONGSWAP_API_URL: &str = "https://api.kongswap.io/api/tokens/by_canister";
const ICP_LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

#[derive(Debug, Clone, Deserialize)]
struct KongSwapMetrics {
    price: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct KongSwapToken {
    metrics: KongSwapMetrics,
}

#[derive(Debug, Clone, Deserialize)]
struct KongSwapResponse {
    items: Vec<KongSwapToken>,
}

pub async fn fetch_icp_price_usd() -> Result<f64, String> {
    let request_body = serde_json::json!({
        "canister_ids": [ICP_LEDGER_CANISTER_ID],
        "page": 1,
        "limit": 1
    });

    let request = HttpRequestArgs {
        url: KONGSWAP_API_URL.to_string(),
        method: HttpMethod::POST,
        headers: vec![HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        }],
        body: Some(request_body.to_string().into_bytes()),
        max_response_bytes: Some(4096),
        transform: Some(TransformContext {
            function: TransformFunc(Func {
                method: "transform_kongswap_response".to_string(),
                principal: ic_cdk::api::canister_self(),
            }),
            context: vec![],
        }),
    };

    match http_request(&request).await {
        Ok(HttpRequestResult { status, body, .. }) => {
            if status != 200u16 {
                return Err(format!("HTTP status {}", status));
            }
            parse_kongswap_response(&body)
        }
        Err(e) => Err(format!("HTTP request failed: {:?}", e)),
    }
}

fn parse_kongswap_response(body: &[u8]) -> Result<f64, String> {
    let response: KongSwapResponse =
        serde_json::from_slice(body).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let token = response
        .items
        .first()
        .ok_or_else(|| "No tokens in response".to_string())?;

    if token.metrics.price <= 0.0 {
        return Err(format!("Invalid price: {}", token.metrics.price));
    }

    Ok(token.metrics.price)
}

pub fn transform_kongswap_response(args: TransformArgs) -> HttpRequestResult {
    HttpRequestResult {
        status: args.response.status,
        headers: vec![],
        body: args.response.body,
    }
}

pub fn price_to_e6(price_usd: f64) -> u64 {
    (price_usd * 1_000_000.0).round() as u64
}

pub async fn refresh_last_token_value_usd_e6_async() -> u64 {
    match fetch_icp_price_usd().await {
        Ok(price) => {
            let price_e6 = price_to_e6(price);
            info!("Fetched ICP/USD price: ${:.6} ({} e6)", price, price_e6);
            price_e6
        }
        Err(e) => {
            error!("Failed to fetch ICP/USD price: {}", e);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kongswap_response() {
        let json = r#"{
            "items": [{
                "token_id": 2,
                "name": "Internet Computer",
                "symbol": "ICP",
                "metrics": {
                    "token_id": 2,
                    "price": 2.40480345239503,
                    "market_cap": 1297568603.7792306
                }
            }],
            "total_pages": 1,
            "total_count": 1,
            "page": 1,
            "limit": 100
        }"#;

        let result = parse_kongswap_response(json.as_bytes()).unwrap();
        assert!((result - 2.40480345239503).abs() < 0.0001);
    }

    #[test]
    fn test_parse_kongswap_response_empty() {
        let json = r#"{"items": [], "total_pages": 0, "total_count": 0}"#;
        let result = parse_kongswap_response(json.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_kongswap_response_invalid_price() {
        let json = r#"{"items": [{"metrics": {"price": -1.0}}]}"#;
        let result = parse_kongswap_response(json.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_price_to_e6() {
        assert_eq!(price_to_e6(1.0), 1_000_000);
        assert_eq!(price_to_e6(2.5), 2_500_000);
        assert_eq!(price_to_e6(2.404803), 2_404_803);
        assert_eq!(price_to_e6(0.001), 1_000);
    }
}
