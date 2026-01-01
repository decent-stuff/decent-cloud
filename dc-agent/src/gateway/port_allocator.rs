//! Port allocation tracking for gateway TCP/UDP routing.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Port allocation for a single VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAllocation {
    /// Base port (e.g., 20000)
    pub base: u16,
    /// Number of ports allocated (e.g., 10)
    pub count: u16,
    /// Contract ID for tracking
    pub contract_id: String,
}

/// Persistent port allocations state.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PortAllocations {
    /// Next base port hint (legacy, kept for backward compat with saved state)
    #[serde(default)]
    #[allow(dead_code)]
    pub next_base: u16,
    /// Allocated port ranges by gateway slug
    pub allocations: HashMap<String, PortAllocation>,
}

/// Port allocator for managing VM port ranges.
pub struct PortAllocator {
    path: PathBuf,
    range_start: u16,
    range_end: u16,
    ports_per_vm: u16,
    allocations: PortAllocations,
}

impl PortAllocator {
    /// Create a new port allocator, loading state from disk if exists.
    pub fn new(path: &str, range_start: u16, range_end: u16, ports_per_vm: u16) -> Result<Self> {
        let path = PathBuf::from(path);

        let allocations = if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read port allocations from {:?}", path))?;
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse port allocations from {:?}", path))?
        } else {
            PortAllocations::default()
        };

        Ok(Self {
            path,
            range_start,
            range_end,
            ports_per_vm,
            allocations,
        })
    }

    /// Save allocations to disk.
    fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(&self.allocations)
            .context("Failed to serialize port allocations")?;
        std::fs::write(&self.path, content)
            .with_context(|| format!("Failed to write port allocations to {:?}", self.path))?;

        Ok(())
    }

    /// Allocate a port range for a VM.
    pub fn allocate(&mut self, slug: &str, contract_id: &str) -> Result<PortAllocation> {
        // Check if already allocated (idempotent)
        if let Some(existing) = self.allocations.allocations.get(slug) {
            return Ok(existing.clone());
        }

        // Find next available base
        let base = self.find_next_available_base()?;

        let allocation = PortAllocation {
            base,
            count: self.ports_per_vm,
            contract_id: contract_id.to_string(),
        };

        self.allocations
            .allocations
            .insert(slug.to_string(), allocation.clone());
        self.save()?;

        Ok(allocation)
    }

    /// Find the next available base port.
    fn find_next_available_base(&self) -> Result<u16> {
        // Collect all allocated bases (sorted)
        let mut allocated_bases: Vec<u16> = self
            .allocations
            .allocations
            .values()
            .map(|a| a.base)
            .collect();
        allocated_bases.sort();

        // Scan from range_start to find first available slot
        let mut candidate = self.range_start;
        for base in &allocated_bases {
            if candidate + self.ports_per_vm <= *base {
                // Found gap before this allocation
                return Ok(candidate);
            }
            // Move candidate past this allocation
            candidate = base + self.ports_per_vm;
        }

        // Check if there's space after all allocations
        if candidate + self.ports_per_vm <= self.range_end {
            return Ok(candidate);
        }

        bail!(
            "Port range exhausted: {} allocations at {} ports each in range {}-{}",
            self.allocations.allocations.len(),
            self.ports_per_vm,
            self.range_start,
            self.range_end
        );
    }

    /// Free a port allocation.
    pub fn free(&mut self, slug: &str) -> Result<()> {
        self.allocations.allocations.remove(slug);
        self.save()?;
        Ok(())
    }

    /// Get current allocations for diagnostics.
    pub fn allocations(&self) -> &PortAllocations {
        &self.allocations
    }

    /// Get allocation for a specific slug.
    pub fn get(&self, slug: &str) -> Option<&PortAllocation> {
        self.allocations.allocations.get(slug)
    }

    /// Find slug by contract_id (for cleanup during termination).
    pub fn find_slug_by_contract(&self, contract_id: &str) -> Option<String> {
        self.allocations
            .allocations
            .iter()
            .find(|(_, alloc)| alloc.contract_id == contract_id)
            .map(|(slug, _)| slug.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_allocator(temp_dir: &TempDir) -> PortAllocator {
        let path = temp_dir.path().join("port-allocations.json");
        PortAllocator::new(path.to_str().unwrap(), 20000, 20100, 10).unwrap()
    }

    #[test]
    fn test_allocate_first() {
        let temp_dir = TempDir::new().unwrap();
        let mut allocator = create_allocator(&temp_dir);

        let alloc = allocator.allocate("abc123", "contract-1").unwrap();
        assert_eq!(alloc.base, 20000);
        assert_eq!(alloc.count, 10);
        assert_eq!(alloc.contract_id, "contract-1");
    }

    #[test]
    fn test_allocate_sequential() {
        let temp_dir = TempDir::new().unwrap();
        let mut allocator = create_allocator(&temp_dir);

        let alloc1 = allocator.allocate("slug1", "contract-1").unwrap();
        let alloc2 = allocator.allocate("slug2", "contract-2").unwrap();
        let alloc3 = allocator.allocate("slug3", "contract-3").unwrap();

        assert_eq!(alloc1.base, 20000);
        assert_eq!(alloc2.base, 20010);
        assert_eq!(alloc3.base, 20020);
    }

    #[test]
    fn test_allocate_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let mut allocator = create_allocator(&temp_dir);

        let alloc1 = allocator.allocate("slug1", "contract-1").unwrap();
        let alloc2 = allocator.allocate("slug1", "contract-1").unwrap();

        assert_eq!(alloc1.base, alloc2.base);
        assert_eq!(alloc1.count, alloc2.count);
    }

    #[test]
    fn test_free_and_reallocate() {
        let temp_dir = TempDir::new().unwrap();
        let mut allocator = create_allocator(&temp_dir);

        let alloc1 = allocator.allocate("slug1", "contract-1").unwrap();
        allocator.allocate("slug2", "contract-2").unwrap();

        allocator.free("slug1").unwrap();

        // New allocation should reuse freed slot
        let alloc3 = allocator.allocate("slug3", "contract-3").unwrap();
        assert_eq!(alloc3.base, alloc1.base);
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("port-allocations.json");

        {
            let mut allocator =
                PortAllocator::new(path.to_str().unwrap(), 20000, 20100, 10).unwrap();
            allocator.allocate("slug1", "contract-1").unwrap();
            allocator.allocate("slug2", "contract-2").unwrap();
        }

        // Reload and verify
        let allocator = PortAllocator::new(path.to_str().unwrap(), 20000, 20100, 10).unwrap();
        assert!(allocator.get("slug1").is_some());
        assert!(allocator.get("slug2").is_some());
        assert_eq!(allocator.get("slug1").unwrap().base, 20000);
        assert_eq!(allocator.get("slug2").unwrap().base, 20010);
    }

    #[test]
    fn test_exhaustion() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("port-allocations.json");
        // Only room for 2 allocations (20 ports, 10 per VM)
        let mut allocator = PortAllocator::new(path.to_str().unwrap(), 20000, 20020, 10).unwrap();

        allocator.allocate("slug1", "contract-1").unwrap();
        allocator.allocate("slug2", "contract-2").unwrap();

        let result = allocator.allocate("slug3", "contract-3");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exhausted"));
    }
}
