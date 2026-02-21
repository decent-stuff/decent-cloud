use crate::price_cache::PriceCache;
use poem::web::Data;
use poem_openapi::{payload::Json, Object, OpenApi};
use serde::Serialize;
use std::sync::Arc;

use super::common::ApiTags;

#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct IcpPriceResponse {
    /// ICP/USD price, or null if the price feed is unavailable
    #[oai(skip_serializing_if_is_none)]
    pub price_usd: Option<f64>,
}

pub struct PricesApi;

#[OpenApi]
impl PricesApi {
    /// ICP/USD price
    ///
    /// Returns the current ICP/USD price from CoinGecko, cached for 5 minutes.
    /// Returns `{"priceUsd": null}` when the price feed is unavailable.
    #[oai(path = "/prices/icp", method = "get", tag = "ApiTags::System")]
    async fn get_icp_price(
        &self,
        price_cache: Data<&Arc<PriceCache>>,
    ) -> Json<IcpPriceResponse> {
        let price_usd = price_cache.get_icp_usd().await;
        Json(IcpPriceResponse { price_usd })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_serializes_with_price() {
        let resp = IcpPriceResponse {
            price_usd: Some(8.45),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["priceUsd"], 8.45_f64);
    }

    #[test]
    fn response_serializes_without_price() {
        let resp = IcpPriceResponse { price_usd: None };
        let json = serde_json::to_value(&resp).unwrap();
        // poem_openapi's Object derive serializes None as JSON null (not omitted)
        assert!(json["priceUsd"].is_null());
    }
}
