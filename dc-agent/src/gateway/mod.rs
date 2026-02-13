//! Gateway module for DC-level reverse proxy management.
//!
//! This module handles:
//! - Port allocation tracking
//! - Caddy for HTTP/HTTPS routing (per-provider wildcard TLS via DNS-01 with acme-dns)
//! - iptables DNAT for TCP/UDP port forwarding (SSH, databases, game servers)
//! - DNS record management (via central API)
//! - Gateway slug generation
//! - Bandwidth monitoring via iptables

mod bandwidth;
mod caddy;
mod iptables;
mod port_allocator;

pub use bandwidth::{BandwidthMonitor, BandwidthStats};
pub use caddy::CaddyConfigManager;
pub use iptables::IptablesNat;
pub use port_allocator::{PortAllocation, PortAllocator};

use crate::api_client::ApiClient;
use crate::config::GatewayConfig;
use crate::provisioner::Instance;
use anyhow::{Context, Result};
use rand::Rng;
use std::sync::Arc;

/// Gateway manager that coordinates port allocation, Caddy config, iptables, and DNS.
pub struct GatewayManager {
    config: GatewayConfig,
    port_allocator: PortAllocator,
    caddy_manager: CaddyConfigManager,
    api_client: Arc<ApiClient>,
}

impl GatewayManager {
    /// Create a new gateway manager from config.
    /// Requires an API client for DNS management via the central API.
    pub fn new(config: GatewayConfig, api_client: Arc<ApiClient>) -> Result<Self> {
        let port_allocator = PortAllocator::new(
            &config.port_allocations_path,
            config.port_range_start,
            config.port_range_end,
            config.ports_per_vm,
        )?;

        let caddy_manager = CaddyConfigManager::new(&config.caddy_sites_dir);

        // Initialize iptables NAT chain for port forwarding
        if let Err(e) = IptablesNat::init_chain() {
            tracing::warn!(
                "Failed to initialize iptables NAT chain: {} - TCP/UDP forwarding may not work",
                e
            );
        }

        let mut manager = Self {
            config,
            port_allocator,
            caddy_manager,
            api_client,
        };

        // Restore iptables rules from persisted allocations (for reboot recovery)
        manager.restore_iptables_rules();

        Ok(manager)
    }

    /// Generate a 6-character alphanumeric slug for subdomain.
    pub fn generate_slug() -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Setup gateway for a newly provisioned VM.
    /// Returns the updated instance with gateway fields populated.
    pub async fn setup_gateway(
        &mut self,
        instance: Instance,
        contract_id: &str,
    ) -> Result<Instance> {
        self.setup_gateway_internal(instance, contract_id, false)
            .await
    }

    /// Setup gateway for testing (skips DNS record creation).
    /// Useful for local testing when DNS API is not configured.
    pub async fn setup_gateway_local(
        &mut self,
        instance: Instance,
        contract_id: &str,
    ) -> Result<Instance> {
        self.setup_gateway_internal(instance, contract_id, true)
            .await
    }

    /// Internal gateway setup with optional DNS skip.
    async fn setup_gateway_internal(
        &mut self,
        mut instance: Instance,
        contract_id: &str,
        skip_dns: bool,
    ) -> Result<Instance> {
        // Generate slug
        let slug = Self::generate_slug();

        // Get internal IP (required for Caddy routing and iptables)
        let internal_ip = instance
            .ip_address
            .as_ref()
            .context("Instance must have an IP address for gateway setup")?;

        // Allocate ports (stores internal_ip for restore after reboot)
        let allocation = self
            .port_allocator
            .allocate(&slug, contract_id, internal_ip)
            .context("Failed to allocate ports for gateway")?;

        // Create DNS record FIRST via central API (unless skipped for testing)
        // Must exist before traffic routing can work
        // The API returns the full subdomain (e.g., "k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org")
        let subdomain = if skip_dns {
            tracing::info!("Skipping DNS record creation (--skip-dns specified)");
            format!("{}.{}.local", slug, self.config.dc_id)
        } else {
            self.api_client
                .create_dns_record(&slug, &self.config.dc_id, &self.config.public_ip)
                .await
                .context("Failed to create DNS record")?
        };

        // Setup iptables DNAT for TCP/UDP port forwarding
        IptablesNat::setup_forwarding(&slug, internal_ip, &allocation)
            .context("Failed to setup iptables port forwarding")?;

        // Write Caddy config (will work for HTTP but HTTPS needs DNS for cert)
        if !skip_dns {
            self.caddy_manager
                .write_vm_config(&slug, &subdomain, internal_ip, &allocation, contract_id)
                .context("Failed to write Caddy config")?;
        } else {
            tracing::info!("Skipping Caddy config (no DNS means no HTTPS cert)");
        }

        // Setup bandwidth monitoring
        if let Err(e) = BandwidthMonitor::setup_accounting(&slug, internal_ip) {
            tracing::warn!("Failed to setup bandwidth monitoring for {}: {}", slug, e);
            // Non-fatal: continue even if bandwidth monitoring fails
        }

        // Update instance with gateway info
        instance.gateway_slug = Some(slug);
        instance.gateway_subdomain = Some(subdomain.clone());
        instance.gateway_ssh_port = Some(allocation.base);
        instance.gateway_port_range_start = Some(allocation.base);
        instance.gateway_port_range_end = Some(allocation.base + allocation.count - 1);

        tracing::info!(
            "Gateway setup complete: {} -> {}:{} (ports {}-{})",
            subdomain,
            internal_ip,
            instance.ssh_port,
            allocation.base,
            allocation.base + allocation.count - 1
        );

        Ok(instance)
    }

    /// Cleanup gateway for a terminated VM.
    pub async fn cleanup_gateway(&mut self, slug: &str) -> Result<()> {
        // Delete Caddy config
        self.caddy_manager
            .delete_vm_config(slug)
            .context("Failed to delete Caddy config")?;

        // Cleanup iptables DNAT rules
        if let Err(e) = IptablesNat::cleanup_forwarding(slug) {
            tracing::warn!("Failed to cleanup iptables for {}: {}", slug, e);
        }

        // Delete DNS record via central API
        if let Err(e) = self
            .api_client
            .delete_dns_record(slug, &self.config.dc_id)
            .await
        {
            tracing::warn!("Failed to delete DNS record for {}: {}", slug, e);
        }

        // Cleanup bandwidth monitoring
        if let Err(e) = BandwidthMonitor::cleanup_accounting(slug) {
            tracing::warn!("Failed to cleanup bandwidth monitoring for {}: {}", slug, e);
        }

        // Free port allocation
        self.port_allocator
            .free(slug)
            .context("Failed to free port allocation")?;

        tracing::info!("Gateway cleanup complete for slug: {}", slug);

        Ok(())
    }

    /// Get bandwidth statistics for all VMs.
    pub fn get_bandwidth_stats(&self) -> std::collections::HashMap<String, BandwidthStats> {
        BandwidthMonitor::get_all_stats().unwrap_or_default()
    }

    /// Get bandwidth statistics for a specific VM by slug.
    pub fn get_vm_bandwidth(&self, slug: &str) -> Option<BandwidthStats> {
        BandwidthMonitor::get_stats(slug).ok()
    }

    /// Get current port allocations for diagnostics.
    pub fn port_allocations(&self) -> &port_allocator::PortAllocations {
        self.port_allocator.allocations()
    }

    /// Find gateway slug by contract_id (for cleanup during termination).
    pub fn find_slug_by_contract(&self, contract_id: &str) -> Option<String> {
        self.port_allocator.find_slug_by_contract(contract_id)
    }

    /// Restore iptables rules from persisted port allocations.
    /// Called on startup to recover gateway state after reboot.
    fn restore_iptables_rules(&mut self) {
        let allocations = self.port_allocator.allocations().allocations.clone();
        if allocations.is_empty() {
            return;
        }

        tracing::info!(
            "Restoring iptables rules for {} allocations",
            allocations.len()
        );

        let mut restored = 0;
        let mut skipped = 0;

        for (slug, allocation) in allocations {
            let Some(internal_ip) = &allocation.internal_ip else {
                tracing::warn!(
                    "Skipping iptables restore for {}: no internal_ip stored (old allocation format)",
                    slug
                );
                skipped += 1;
                continue;
            };

            match IptablesNat::setup_forwarding(&slug, internal_ip, &allocation) {
                Ok(()) => {
                    tracing::debug!("Restored iptables rules for {} -> {}", slug, internal_ip);
                    restored += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to restore iptables for {}: {}", slug, e);
                    skipped += 1;
                }
            }
        }

        tracing::info!(
            "iptables restore complete: {} restored, {} skipped",
            restored,
            skipped
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug() {
        let slug = GatewayManager::generate_slug();
        assert_eq!(slug.len(), 6);
        // Slug contains only lowercase letters and digits
        assert!(slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_slug_uniqueness() {
        let slugs: Vec<String> = (0..100).map(|_| GatewayManager::generate_slug()).collect();
        let unique: std::collections::HashSet<_> = slugs.iter().collect();
        // With 36^6 = 2.1B combinations, 100 slugs should be unique
        assert_eq!(slugs.len(), unique.len());
    }
}
