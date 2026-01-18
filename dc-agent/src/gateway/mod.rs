//! Gateway module for DC-level reverse proxy management.
//!
//! This module handles:
//! - Port allocation tracking
//! - Caddy for HTTP/HTTPS routing (TLS termination with automatic HTTP-01 certs)
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

        Ok(Self {
            config,
            port_allocator,
            caddy_manager,
            api_client,
        })
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

    /// Build the full subdomain from a slug.
    pub fn build_subdomain(&self, slug: &str) -> String {
        format!("{}.{}.{}", slug, self.config.datacenter, self.config.domain)
    }

    /// Setup gateway for a newly provisioned VM.
    /// Returns the updated instance with gateway fields populated.
    pub async fn setup_gateway(
        &mut self,
        mut instance: Instance,
        contract_id: &str,
    ) -> Result<Instance> {
        // Generate slug
        let slug = Self::generate_slug();
        let subdomain = self.build_subdomain(&slug);

        // Allocate ports
        let allocation = self
            .port_allocator
            .allocate(&slug, contract_id)
            .context("Failed to allocate ports for gateway")?;

        // Get internal IP (required for Caddy routing)
        let internal_ip = instance
            .ip_address
            .as_ref()
            .context("Instance must have an IP address for gateway setup")?;

        // Create DNS record FIRST via central API
        // Must exist before Caddy config so HTTP-01 challenge can succeed
        self.api_client
            .create_dns_record(&slug, &self.config.datacenter, &self.config.public_ip)
            .await
            .context("Failed to create DNS record")?;

        // Setup iptables DNAT for TCP/UDP port forwarding
        IptablesNat::setup_forwarding(&slug, internal_ip, &allocation)
            .context("Failed to setup iptables port forwarding")?;

        // Write Caddy config AFTER DNS exists
        // Caddy will reload and obtain Let's Encrypt cert via HTTP-01
        self.caddy_manager
            .write_vm_config(&slug, &subdomain, internal_ip, &allocation, contract_id)
            .context("Failed to write Caddy config")?;

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
            .delete_dns_record(slug, &self.config.datacenter)
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
