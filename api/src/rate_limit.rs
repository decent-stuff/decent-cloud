use poem::http::StatusCode;
use poem::{Endpoint, IntoResponse, Middleware, Request, Response};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const STRICT_MAX: u32 = 10;
const STRICT_WINDOW_SECS: u64 = 60;

const STANDARD_MAX: u32 = 60;
const STANDARD_WINDOW_SECS: u64 = 60;

const RELAXED_MAX: u32 = 120;
const RELAXED_WINDOW_SECS: u64 = 60;

const CLEANUP_INTERVAL: u32 = 500;
const MAX_ENTRIES: usize = 50_000;

#[derive(Clone, Copy)]
enum Tier {
    Strict,
    Standard,
    Relaxed,
}

impl Tier {
    fn limits(self) -> (u32, u64) {
        match self {
            Tier::Strict => (STRICT_MAX, STRICT_WINDOW_SECS),
            Tier::Standard => (STANDARD_MAX, STANDARD_WINDOW_SECS),
            Tier::Relaxed => (RELAXED_MAX, RELAXED_WINDOW_SECS),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Tier::Strict => "strict",
            Tier::Standard => "standard",
            Tier::Relaxed => "relaxed",
        }
    }
}

struct Entry {
    count: u32,
    window_start: Instant,
}

struct RateLimitState {
    entries: HashMap<String, Entry>,
    request_count: u32,
}

impl RateLimitState {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            request_count: 0,
        }
    }
}

pub struct RateLimiter {
    state: Arc<Mutex<RateLimitState>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimitState::new())),
        }
    }
}

impl<E: Endpoint> Middleware<E> for RateLimiter {
    type Output = RateLimitEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        RateLimitEndpoint {
            inner: ep,
            state: self.state.clone(),
        }
    }
}

pub struct RateLimitEndpoint<E> {
    inner: E,
    state: Arc<Mutex<RateLimitState>>,
}

impl<E: Endpoint> Endpoint for RateLimitEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        let path = req.uri().path().to_string();
        let method = req.method();

        if should_skip(&path) {
            return self.inner.call(req).await.map(|r| r.into_response());
        }

        let tier = classify(&path, method.as_ref());
        let client_ip = req
            .remote_addr()
            .as_socket_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let key = format!("{}:{}", client_ip, tier.name());
        let (max_requests, window_secs) = tier.limits();

        let allowed = {
            let mut state = self.state.lock().unwrap();
            let now = Instant::now();

            state.request_count += 1;
            if state.request_count >= CLEANUP_INTERVAL {
                state.request_count = 0;
                state.entries.retain(|_, e| now.duration_since(e.window_start).as_secs() < window_secs);
            }
            if state.entries.len() > MAX_ENTRIES {
                let oldest_keys: Vec<String> = state
                    .entries
                    .iter()
                    .filter(|(_, e)| now.duration_since(e.window_start).as_secs() >= window_secs)
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in oldest_keys {
                    state.entries.remove(&k);
                }
            }

            let entry = state.entries.entry(key).or_insert(Entry {
                count: 0,
                window_start: now,
            });

            if now.duration_since(entry.window_start).as_secs() >= window_secs {
                entry.count = 0;
                entry.window_start = now;
            }

            entry.count += 1;
            entry.count <= max_requests
        };

        if !allowed {
            tracing::warn!(
                path = %path,
                method = %method,
                client_ip = %client_ip,
                tier = tier.name(),
                "rate limit exceeded"
            );
            return Ok(poem::Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("Retry-After", window_secs.to_string())
                .header("X-RateLimit-Tier", tier.name())
                .body("Rate limit exceeded. Please try again later."));
        }

        self.inner.call(req).await.map(|r| r.into_response())
    }
}

fn should_skip(path: &str) -> bool {
    if path == "/" {
        return true;
    }
    if path.starts_with("/api/v1/swagger") || path.starts_with("/api/v1/openapi") {
        return true;
    }
    if path.starts_with("/api/v1/webhooks/") {
        return true;
    }
    if path.starts_with("/api/v1/acme-dns/") {
        return true;
    }
    if path == "/api/v1/health" {
        return true;
    }
    false
}

fn classify(path: &str, method: &str) -> Tier {
    if is_strict_path(path) && !matches!(method, "GET" | "HEAD" | "OPTIONS") {
        return Tier::Strict;
    }
    match method {
        "GET" | "HEAD" | "OPTIONS" => Tier::Relaxed,
        _ => Tier::Standard,
    }
}

fn is_strict_path(path: &str) -> bool {
    let p = path.trim_start_matches("/api/v1");
    if p.is_empty() {
        return false;
    }
    if p == "/accounts" || p.starts_with("/accounts/recovery") {
        return true;
    }
    if p.starts_with("/accounts/verify-email") || p.starts_with("/accounts/resend-verification") {
        return true;
    }
    if p == "/oauth/register" {
        return true;
    }
    if p.starts_with("/subscriptions/checkout") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_internal_paths() {
        assert!(should_skip("/"));
        assert!(should_skip("/api/v1/swagger/ui"));
        assert!(should_skip("/api/v1/openapi.json"));
        assert!(should_skip("/api/v1/webhooks/stripe"));
        assert!(should_skip("/api/v1/webhooks/icpay"));
        assert!(should_skip("/api/v1/acme-dns/update"));
        assert!(should_skip("/api/v1/health"));
    }

    #[test]
    fn test_should_not_skip_api_paths() {
        assert!(!should_skip("/api/v1/accounts"));
        assert!(!should_skip("/api/v1/providers"));
        assert!(!should_skip("/api/v1/offerings"));
        assert!(!should_skip("/api/v1/oauth/register"));
    }

    #[test]
    fn test_classify_strict_tier() {
        assert!(matches!(
            classify("/api/v1/accounts", "POST"),
            Tier::Strict
        ));
        assert!(matches!(
            classify("/api/v1/accounts/recovery/request", "POST"),
            Tier::Strict
        ));
        assert!(matches!(
            classify("/api/v1/accounts/recovery/complete", "POST"),
            Tier::Strict
        ));
        assert!(matches!(
            classify("/api/v1/accounts/verify-email", "POST"),
            Tier::Strict
        ));
        assert!(matches!(
            classify("/api/v1/accounts/resend-verification", "POST"),
            Tier::Strict
        ));
        assert!(matches!(
            classify("/api/v1/oauth/register", "POST"),
            Tier::Strict
        ));
        assert!(matches!(
            classify("/api/v1/subscriptions/checkout", "POST"),
            Tier::Strict
        ));
    }

    #[test]
    fn test_classify_relaxed_tier() {
        assert!(matches!(
            classify("/api/v1/providers", "GET"),
            Tier::Relaxed
        ));
        assert!(matches!(
            classify("/api/v1/offerings", "GET"),
            Tier::Relaxed
        ));
        assert!(matches!(
            classify("/api/v1/stats", "GET"),
            Tier::Relaxed
        ));
        assert!(matches!(
            classify("/api/v1/accounts/username/profile", "GET"),
            Tier::Relaxed
        ));
    }

    #[test]
    fn test_classify_standard_tier() {
        assert!(matches!(
            classify("/api/v1/offerings/123", "PUT"),
            Tier::Standard
        ));
        assert!(matches!(
            classify("/api/v1/contracts", "POST"),
            Tier::Standard
        ));
        assert!(matches!(
            classify("/api/v1/contracts/1", "DELETE"),
            Tier::Standard
        ));
    }

    #[test]
    fn test_tier_limits() {
        let (max, window) = Tier::Strict.limits();
        assert_eq!(max, 10);
        assert_eq!(window, 60);

        let (max, window) = Tier::Standard.limits();
        assert_eq!(max, 60);
        assert_eq!(window, 60);

        let (max, window) = Tier::Relaxed.limits();
        assert_eq!(max, 120);
        assert_eq!(window, 60);
    }

    #[test]
    fn test_accounts_get_is_relaxed_not_strict() {
        assert!(matches!(
            classify("/api/v1/accounts", "GET"),
            Tier::Relaxed
        ));
        assert!(matches!(
            classify("/api/v1/accounts/someuser/profile", "GET"),
            Tier::Relaxed
        ));
        assert!(matches!(
            classify("/api/v1/accounts/recovery/request", "GET"),
            Tier::Relaxed
        ));
    }

    #[test]
    fn test_rate_limit_allows_up_to_max() {
        let state = Arc::new(Mutex::new(RateLimitState::new()));
        let key = "127.0.0.1:strict".to_string();
        let (max, _) = Tier::Strict.limits();

        for _ in 0..max {
            let mut s = state.lock().unwrap();
            let now = Instant::now();
            let entry = s.entries.entry(key.clone()).or_insert(Entry {
                count: 0,
                window_start: now,
            });
            if now.duration_since(entry.window_start).as_secs() >= 60 {
                entry.count = 0;
                entry.window_start = now;
            }
            entry.count += 1;
            assert!(entry.count <= max);
        }

        let s = state.lock().unwrap();
        let entry = s.entries.get(&key).unwrap();
        assert_eq!(entry.count, max);
    }

    #[test]
    fn test_rate_limit_blocks_over_max() {
        let state = Arc::new(Mutex::new(RateLimitState::new()));
        let key = "127.0.0.1:strict".to_string();
        let (max, _) = Tier::Strict.limits();

        {
            let mut s = state.lock().unwrap();
            let now = Instant::now();
            s.entries.insert(key.clone(), Entry {
                count: max,
                window_start: now,
            });
        }

        let mut s = state.lock().unwrap();
        let entry = s.entries.entry(key.clone()).or_insert(Entry {
            count: 0,
            window_start: Instant::now(),
        });
        entry.count += 1;
        assert!(entry.count > max);
    }
}
