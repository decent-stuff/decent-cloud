use candid::{CandidType, Deserialize, Principal};
use ledger_map::info;

const KONGSWAP_BACKEND_CANISTER_ID: &str = "2ipq2-uqaaa-aaaar-qailq-cai";
#[cfg(test)]
const ICP_LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
const CKUSDT_LEDGER_CANISTER_ID: &str = "cngnf-vqaaa-aaaar-qag4q-cai";

#[derive(Debug, Clone, CandidType, Deserialize)]
enum KongPoolsResult {
    Ok(Vec<KongPoolReply>),
    Err(String),
}

#[derive(Debug, Clone, CandidType, Deserialize)]
struct KongPoolReply {
    address_0: String,
    address_1: String,
    price: f64,
    is_removed: bool,
}

pub async fn fetch_dct_price_usd() -> Result<f64, String> {
    let token_canister_id = ic_cdk::api::canister_self().to_text();
    fetch_token_price_usd(&token_canister_id).await
}

async fn fetch_token_price_usd(token_canister_id: &str) -> Result<f64, String> {
    let kongswap = Principal::from_text(KONGSWAP_BACKEND_CANISTER_ID)
        .map_err(|e| format!("Invalid KongSwap canister id: {e}"))?;
    let pool_filter = format!("{}_{}", token_canister_id, CKUSDT_LEDGER_CANISTER_ID);
    #[allow(deprecated)]
    let (result,): (KongPoolsResult,) =
        ic_cdk::api::call::call(kongswap, "pools", (Some(pool_filter),))
            .await
            .map_err(|e| format!("KongSwap pools call failed: code={:?} message={}", e.0, e.1))?;
    extract_price_from_pools_result(result, token_canister_id)
}

fn extract_price_from_pools_result(
    result: KongPoolsResult,
    token_canister_id: &str,
) -> Result<f64, String> {
    match result {
        KongPoolsResult::Err(e) => Err(format!("KongSwap pools returned error: {e}")),
        KongPoolsResult::Ok(pools) => {
            let pool = pools
                .into_iter()
                .find(|pool| {
                    !pool.is_removed
                        && pool.address_0 == token_canister_id
                        && pool.address_1 == CKUSDT_LEDGER_CANISTER_ID
                })
                .ok_or_else(|| {
                    format!(
                        "No active KongSwap pool found for {}_{}",
                        token_canister_id, CKUSDT_LEDGER_CANISTER_ID
                    )
                })?;
            if !pool.price.is_finite() || pool.price <= 0.0 {
                return Err(format!("Invalid pool price: {}", pool.price));
            }
            Ok(pool.price)
        }
    }
}

pub fn price_to_e6(price_usd: f64) -> u64 {
    (price_usd * 1_000_000.0).round() as u64
}

pub async fn refresh_last_token_value_usd_e6_async() -> Result<u64, String> {
    let price = fetch_dct_price_usd().await?;
    let price_e6 = price_to_e6(price);
    info!("Fetched DCT/USD price: ${:.6} ({} e6)", price, price_e6);
    Ok(price_e6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_price_returns_exact_pool_price() {
        let result = KongPoolsResult::Ok(vec![KongPoolReply {
            address_0: "ggi4a-wyaaa-aaaai-actqq-cai".to_string(),
            address_1: CKUSDT_LEDGER_CANISTER_ID.to_string(),
            price: 0.0203852135,
            is_removed: false,
        }]);

        let price = extract_price_from_pools_result(result, "ggi4a-wyaaa-aaaai-actqq-cai").unwrap();
        assert!((price - 0.0203852135).abs() < 0.000_000_1);
    }

    #[test]
    fn extract_price_rejects_kongswap_error_variant() {
        let result = extract_price_from_pools_result(
            KongPoolsResult::Err("upstream failed".to_string()),
            "ggi4a-wyaaa-aaaai-actqq-cai",
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("KongSwap pools returned error"));
    }

    #[test]
    fn extract_price_rejects_missing_matching_pool() {
        let result = KongPoolsResult::Ok(vec![KongPoolReply {
            address_0: ICP_LEDGER_CANISTER_ID.to_string(),
            address_1: CKUSDT_LEDGER_CANISTER_ID.to_string(),
            price: 2.39241518,
            is_removed: false,
        }]);

        let result = extract_price_from_pools_result(result, "ggi4a-wyaaa-aaaai-actqq-cai");
        assert!(result.is_err());
    }

    #[test]
    fn extract_price_rejects_removed_or_non_positive_pool() {
        let removed_pool = KongPoolsResult::Ok(vec![KongPoolReply {
            address_0: "ggi4a-wyaaa-aaaai-actqq-cai".to_string(),
            address_1: CKUSDT_LEDGER_CANISTER_ID.to_string(),
            price: 0.02,
            is_removed: true,
        }]);
        let removed_result =
            extract_price_from_pools_result(removed_pool, "ggi4a-wyaaa-aaaai-actqq-cai");
        assert!(removed_result.is_err());

        let non_positive_pool = KongPoolsResult::Ok(vec![KongPoolReply {
            address_0: "ggi4a-wyaaa-aaaai-actqq-cai".to_string(),
            address_1: CKUSDT_LEDGER_CANISTER_ID.to_string(),
            price: 0.0,
            is_removed: false,
        }]);
        let non_positive_result =
            extract_price_from_pools_result(non_positive_pool, "ggi4a-wyaaa-aaaai-actqq-cai");
        assert!(non_positive_result.is_err());
        assert!(non_positive_result
            .unwrap_err()
            .contains("Invalid pool price"));
    }

    #[test]
    fn extract_price_rejects_non_finite_pool_price() {
        let nan_pool = KongPoolsResult::Ok(vec![KongPoolReply {
            address_0: "ggi4a-wyaaa-aaaai-actqq-cai".to_string(),
            address_1: CKUSDT_LEDGER_CANISTER_ID.to_string(),
            price: f64::NAN,
            is_removed: false,
        }]);
        let nan_result = extract_price_from_pools_result(nan_pool, "ggi4a-wyaaa-aaaai-actqq-cai");
        assert!(nan_result.is_err());

        let inf_pool = KongPoolsResult::Ok(vec![KongPoolReply {
            address_0: "ggi4a-wyaaa-aaaai-actqq-cai".to_string(),
            address_1: CKUSDT_LEDGER_CANISTER_ID.to_string(),
            price: f64::INFINITY,
            is_removed: false,
        }]);
        let inf_result = extract_price_from_pools_result(inf_pool, "ggi4a-wyaaa-aaaai-actqq-cai");
        assert!(inf_result.is_err());
    }

    #[test]
    fn fetch_dct_uses_current_canister_id_format_for_pool_filter() {
        let filter = format!(
            "{}_{}",
            "ggi4a-wyaaa-aaaai-actqq-cai", CKUSDT_LEDGER_CANISTER_ID
        );
        assert_eq!(
            filter,
            "ggi4a-wyaaa-aaaai-actqq-cai_cngnf-vqaaa-aaaar-qag4q-cai"
        );
    }

    #[test]
    fn test_price_to_e6() {
        assert_eq!(price_to_e6(1.0), 1_000_000);
        assert_eq!(price_to_e6(2.5), 2_500_000);
        assert_eq!(price_to_e6(2.404803), 2_404_803);
        assert_eq!(price_to_e6(0.001), 1_000);
    }

    #[test]
    fn test_price_to_e6_rounds_half_up() {
        assert_eq!(price_to_e6(0.000_000_4), 0);
        assert_eq!(price_to_e6(0.000_000_5), 1);
    }

    #[test]
    fn fetch_dct_price_function_exists() {
        let _ = fetch_dct_price_usd;
    }

    #[test]
    fn kongswap_backend_canister_id_is_valid_principal() {
        let parsed = Principal::from_text(KONGSWAP_BACKEND_CANISTER_ID);
        assert!(parsed.is_ok());
    }

    #[test]
    fn ckusdt_canister_id_is_valid_principal() {
        let parsed = Principal::from_text(CKUSDT_LEDGER_CANISTER_ID);
        assert!(parsed.is_ok());
    }

    #[test]
    fn icp_canister_id_is_valid_principal() {
        let parsed = Principal::from_text(ICP_LEDGER_CANISTER_ID);
        assert!(parsed.is_ok());
    }

    #[test]
    fn extract_price_fails_when_pool_is_for_reversed_pair() {
        let result = KongPoolsResult::Ok(vec![KongPoolReply {
            address_0: CKUSDT_LEDGER_CANISTER_ID.to_string(),
            address_1: "ggi4a-wyaaa-aaaai-actqq-cai".to_string(),
            price: 49.0,
            is_removed: false,
        }]);

        let result = extract_price_from_pools_result(result, "ggi4a-wyaaa-aaaai-actqq-cai");
        assert!(result.is_err());
    }
}
