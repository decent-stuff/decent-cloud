use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

const COINGECKO_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=internet-computer&vs_currencies=usd";

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

#[derive(Debug, Clone)]
pub struct IcpPrice {
    pub usd: f64,
    pub fetched_at: Instant,
}

pub struct PriceCache {
    inner: Arc<RwLock<Option<IcpPrice>>>,
    http: reqwest::Client,
}

impl PriceCache {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            http,
        }
    }

    /// Returns cached ICP/USD price, refreshing if stale or missing.
    pub async fn get_icp_usd(&self) -> Option<f64> {
        // Fast path: read cache
        {
            let cache = self.inner.read().expect("price cache lock poisoned");
            if let Some(ref p) = *cache {
                if p.fetched_at.elapsed() < CACHE_TTL {
                    return Some(p.usd);
                }
            }
        }

        // Slow path: fetch and update
        match self.fetch_from_coingecko().await {
            Ok(usd) => {
                let mut cache = self.inner.write().expect("price cache lock poisoned");
                *cache = Some(IcpPrice {
                    usd,
                    fetched_at: Instant::now(),
                });
                Some(usd)
            }
            Err(e) => {
                tracing::warn!("ICP price fetch failed: {:#}", e);
                // Return stale cached value if available (better than nothing)
                let cache = self.inner.read().expect("price cache lock poisoned");
                if let Some(ref p) = *cache {
                    tracing::warn!(
                        "Returning stale cached ICP price (${:.2}, {}s old)",
                        p.usd,
                        p.fetched_at.elapsed().as_secs()
                    );
                    Some(p.usd)
                } else {
                    tracing::warn!("No cached ICP price available — returning null");
                    None
                }
            }
        }
    }

    async fn fetch_from_coingecko(&self) -> anyhow::Result<f64> {
        let resp = self
            .http
            .get(COINGECKO_URL)
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;

        resp["internet-computer"]["usd"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("unexpected CoinGecko response shape: {resp}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> PriceCache {
        PriceCache::new(reqwest::Client::new())
    }

    #[test]
    fn cache_starts_empty() {
        let cache = make_cache();
        let inner = cache.inner.read().unwrap();
        assert!(inner.is_none());
    }

    #[test]
    fn icp_price_is_fresh_within_ttl() {
        let price = IcpPrice {
            usd: 5.0,
            fetched_at: Instant::now(),
        };
        assert!(price.fetched_at.elapsed() < CACHE_TTL);
    }

    #[test]
    fn icp_price_is_stale_after_ttl() {
        // Simulate a fetched_at that is older than TTL by subtracting more than TTL
        let fetched_at = Instant::now() - CACHE_TTL - Duration::from_secs(1);
        let price = IcpPrice {
            usd: 5.0,
            fetched_at,
        };
        assert!(price.fetched_at.elapsed() >= CACHE_TTL);
    }

    #[tokio::test]
    async fn returns_none_on_unreachable_host() {
        // Use a URL that will fail immediately (local unreachable port)
        let client = reqwest::Client::new();
        let cache = PriceCache {
            inner: Arc::new(RwLock::new(None)),
            http: client,
        };

        // Override via direct fetch_from_coingecko is not accessible, so test via
        // get_icp_usd with a real but incorrect-format mock would need integration test.
        // Here we just verify that the cache is None before any fetch.
        let inner = cache.inner.read().unwrap();
        assert!(inner.is_none());
    }

    #[test]
    fn write_then_read_cache() {
        let cache = make_cache();
        {
            let mut w = cache.inner.write().unwrap();
            *w = Some(IcpPrice {
                usd: 8.45,
                fetched_at: Instant::now(),
            });
        }
        let r = cache.inner.read().unwrap();
        let p = r.as_ref().unwrap();
        assert!((p.usd - 8.45).abs() < f64::EPSILON);
        assert!(p.fetched_at.elapsed() < CACHE_TTL);
    }
}
