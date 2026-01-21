//! Persistent orphan VM tracking for automatic pruning.
//!
//! Tracks when orphan VMs (VMs without valid contract IDs) are first detected.
//! Orphans exceeding the grace period are automatically pruned.
//! State persists to disk so orphans can't escape cleanup by agent restarts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Persistent orphan tracker state.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OrphanTracker {
    /// Map of external_id -> unix timestamp when first detected
    pub first_seen: HashMap<String, u64>,
    /// Path to persistence file (not serialized)
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl OrphanTracker {
    /// Load orphan tracker state from disk, or create empty if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self> {
        let mut tracker = if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read orphan tracker from {:?}", path))?;
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse orphan tracker from {:?}", path))?
        } else {
            Self::default()
        };
        tracker.path = Some(path.to_path_buf());
        Ok(tracker)
    }

    /// Save tracker state to disk.
    pub fn save(&self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("OrphanTracker has no persistence path configured")?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize orphan tracker")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write orphan tracker to {:?}", path))?;

        Ok(())
    }

    /// Record an orphan, returning the timestamp when it was first seen.
    /// If already tracked, returns existing first_seen timestamp.
    /// If new, records current time and returns it.
    pub fn record_orphan(&mut self, external_id: &str, now: u64) -> u64 {
        *self
            .first_seen
            .entry(external_id.to_string())
            .or_insert(now)
    }

    /// Remove an orphan from tracking (after successful pruning).
    pub fn remove(&mut self, external_id: &str) {
        self.first_seen.remove(external_id);
    }

    /// Retain only orphans that are still present. Removes any that were resolved.
    /// Returns the list of external_ids that were removed (resolved orphans).
    pub fn retain_present(&mut self, present: &HashSet<String>) -> Vec<String> {
        let mut removed = Vec::new();
        self.first_seen.retain(|external_id, _| {
            if present.contains(external_id) {
                true
            } else {
                removed.push(external_id.clone());
                false
            }
        });
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_missing_file_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("orphans.json");

        let tracker = OrphanTracker::load(&path).unwrap();
        assert!(tracker.first_seen.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("orphans.json");

        // Save some data
        {
            let mut tracker = OrphanTracker::load(&path).unwrap();
            tracker.record_orphan("vm-100", 1000);
            tracker.record_orphan("vm-200", 2000);
            tracker.save().unwrap();
        }

        // Load and verify
        let tracker = OrphanTracker::load(&path).unwrap();
        assert_eq!(tracker.first_seen.len(), 2);
        assert_eq!(tracker.first_seen.get("vm-100"), Some(&1000));
        assert_eq!(tracker.first_seen.get("vm-200"), Some(&2000));
    }

    #[test]
    fn test_record_orphan_returns_first_seen() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("orphans.json");
        let mut tracker = OrphanTracker::load(&path).unwrap();

        // First time seeing this orphan
        let first = tracker.record_orphan("vm-100", 1000);
        assert_eq!(first, 1000);

        // Second time - should return original timestamp, not new one
        let second = tracker.record_orphan("vm-100", 2000);
        assert_eq!(second, 1000);
    }

    #[test]
    fn test_remove() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("orphans.json");
        let mut tracker = OrphanTracker::load(&path).unwrap();

        tracker.record_orphan("vm-100", 1000);
        tracker.record_orphan("vm-200", 2000);
        assert_eq!(tracker.first_seen.len(), 2);

        tracker.remove("vm-100");
        assert_eq!(tracker.first_seen.len(), 1);
        assert!(!tracker.first_seen.contains_key("vm-100"));
        assert!(tracker.first_seen.contains_key("vm-200"));
    }

    #[test]
    fn test_retain_present() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("orphans.json");
        let mut tracker = OrphanTracker::load(&path).unwrap();

        tracker.record_orphan("vm-100", 1000);
        tracker.record_orphan("vm-200", 2000);
        tracker.record_orphan("vm-300", 3000);

        // Only vm-200 is still present
        let present: HashSet<String> = ["vm-200".to_string()].into_iter().collect();
        let removed = tracker.retain_present(&present);

        assert_eq!(tracker.first_seen.len(), 1);
        assert!(tracker.first_seen.contains_key("vm-200"));
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&"vm-100".to_string()));
        assert!(removed.contains(&"vm-300".to_string()));
    }

    #[test]
    fn test_persistence_survives_restart() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("orphans.json");

        // Simulate agent run 1: detect orphan
        {
            let mut tracker = OrphanTracker::load(&path).unwrap();
            tracker.record_orphan("vm-100", 1000);
            tracker.save().unwrap();
        }

        // Simulate agent run 2: should retain original first_seen
        {
            let mut tracker = OrphanTracker::load(&path).unwrap();
            let first_seen = tracker.record_orphan("vm-100", 5000);
            // Should still be 1000, not 5000
            assert_eq!(first_seen, 1000);
        }
    }

    #[test]
    fn test_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nested/deep/orphans.json");

        let mut tracker = OrphanTracker::load(&path).unwrap();
        tracker.record_orphan("vm-100", 1000);
        tracker.save().unwrap();

        // Verify file was created
        assert!(path.exists());
    }
}
