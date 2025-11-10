use crate::ledger_client::{LedgerClient, MetadataValue};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct CachedMetadata {
    pub data: HashMap<String, MetadataValue>,
    pub last_updated: SystemTime,
}

impl CachedMetadata {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            last_updated: SystemTime::UNIX_EPOCH,
        }
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.data.get(key).and_then(|v| match v {
            MetadataValue::Nat(n) => Some(*n),
            MetadataValue::Int(i) if *i >= 0 => Some(*i as u64),
            _ => None,
        })
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.data.get(key).and_then(|v| match v {
            MetadataValue::Nat(n) => Some(*n as i64),
            MetadataValue::Int(i) => Some(*i),
            _ => None,
        })
    }
}

pub struct MetadataCache {
    cache: Arc<RwLock<CachedMetadata>>,
    ledger_client: Arc<LedgerClient>,
    refresh_interval: Duration,
}

impl MetadataCache {
    pub fn new(ledger_client: Arc<LedgerClient>, refresh_interval_secs: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(CachedMetadata::new())),
            ledger_client,
            refresh_interval: Duration::from_secs(refresh_interval_secs),
        }
    }

    pub async fn run(&self) {
        let mut interval = tokio::time::interval(self.refresh_interval);

        // Run initial fetch immediately on startup
        if let Err(e) = self.refresh().await {
            tracing::error!("Initial metadata fetch failed: {}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = self.refresh().await {
                tracing::error!("Metadata refresh failed: {}", e);
            }
        }
    }

    async fn refresh(&self) -> Result<()> {
        let metadata = self
            .ledger_client
            .fetch_metadata()
            .await
            .context("Failed to fetch metadata from canister")?;

        let mut cache = self.cache.write().map_err(|_| {
            anyhow::anyhow!("Failed to acquire cache lock - possible poisoning")
        })?;

        cache.data.clear();
        for (key, value) in metadata {
            cache.data.insert(key, value);
        }
        cache.last_updated = SystemTime::now();

        tracing::debug!("Metadata cache refreshed with {} entries", cache.data.len());
        Ok(())
    }

    pub fn get(&self) -> Result<CachedMetadata> {
        self.cache
            .read()
            .map(|c| c.clone())
            .map_err(|_| anyhow::anyhow!("Failed to acquire cache lock - possible poisoning"))
    }
}
