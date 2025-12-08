use crate::ledger_client::LedgerClient;
use anyhow::Result;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};
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

    /// Convert all metadata to JSON format
    pub fn to_json_map(&self) -> BTreeMap<String, JsonValue> {
        self.data
            .iter()
            .map(|(k, v)| (k.clone(), metadata_value_to_json(v)))
            .collect()
    }
}

/// Convert MetadataValue to JSON Value
fn metadata_value_to_json(value: &MetadataValue) -> JsonValue {
    match value {
        MetadataValue::Nat(n) => {
            // Try to parse as u64, fall back to string for large numbers
            if n > &candid::Nat::from(u64::MAX) {
                return JsonValue::String(n.to_string());
            }
            n.0.to_u64_digits()
                .first()
                .cloned()
                .unwrap_or_default()
                .into()
        }
        MetadataValue::Int(i) => {
            // Try to parse as i64, fall back to string for large numbers
            if i > &candid::Int::from(i64::MAX) {
                return JsonValue::String(i.to_string());
            }
            let (sign, n) = i.0.to_u64_digits();
            if sign == num_bigint::Sign::Minus {
                JsonValue::from(-(*n.first().unwrap_or(&0) as i64))
            } else {
                JsonValue::from(*n.first().unwrap_or(&0) as i64)
            }
        }
        MetadataValue::Text(t) => JsonValue::String(t.clone()),
        MetadataValue::Blob(b) => JsonValue::String(hex::encode(b)),
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

        loop {
            interval.tick().await;
            if let Err(e) = self.refresh().await {
                tracing::error!("Metadata refresh failed: {:#}", e);
            }
        }
    }

    async fn refresh(&self) -> Result<()> {
        let metadata = self.ledger_client.fetch_metadata().await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to fetch metadata from canister after retries: {}",
                e
            )
        })?;

        let mut cache = self
            .cache
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to acquire cache lock - possible poisoning"))?;

        cache.data.clear();
        for (key, value) in metadata {
            cache.data.insert(key, value);
        }
        cache.last_updated = SystemTime::now();

        tracing::info!("Metadata cache refreshed with {} entries", cache.data.len());
        Ok(())
    }

    pub fn get(&self) -> Result<CachedMetadata> {
        self.cache
            .read()
            .map(|c| c.clone())
            .map_err(|_| anyhow::anyhow!("Failed to acquire cache lock - possible poisoning"))
    }
}
