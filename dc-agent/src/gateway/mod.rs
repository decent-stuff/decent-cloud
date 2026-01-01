//! Gateway module for DC-level reverse proxy (Traefik) management.
//!
//! This module handles:
//! - Port allocation tracking
//! - Traefik dynamic configuration file generation
//! - Cloudflare DNS record management
//! - Gateway slug generation

mod cloudflare;
mod port_allocator;
mod traefik;

pub use cloudflare::CloudflareClient;
pub use port_allocator::{PortAllocation, PortAllocator};
pub use traefik::TraefikConfigManager;

use crate::config::GatewayConfig;
use crate::provisioner::Instance;
use anyhow::{Context, Result};
use rand::Rng;

/// Gateway manager that coordinates port allocation, Traefik config, and DNS.
pub struct GatewayManager {
    config: GatewayConfig,
    port_allocator: PortAllocator,
    traefik_manager: TraefikConfigManager,
    cloudflare: CloudflareClient,
}

impl GatewayManager {
    /// Create a new gateway manager from config.
    pub fn new(config: GatewayConfig) -> Result<Self> {
        let port_allocator = PortAllocator::new(
            &config.port_allocations_path,
            config.port_range_start,
            config.port_range_end,
            config.ports_per_vm,
        )?;

        let traefik_manager = TraefikConfigManager::new(&config.traefik_dynamic_dir);

        let cloudflare =
            CloudflareClient::new(&config.cloudflare_api_token, &config.cloudflare_zone_id);

        Ok(Self {
            config,
            port_allocator,
            traefik_manager,
            cloudflare,
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

        // Get internal IP (required for Traefik routing)
        let internal_ip = instance
            .ip_address
            .as_ref()
            .context("Instance must have an IP address for gateway setup")?;

        // Write Traefik config
        self.traefik_manager
            .write_vm_config(&slug, &subdomain, internal_ip, &allocation, contract_id)
            .context("Failed to write Traefik config")?;

        // Create DNS record
        self.cloudflare
            .create_a_record(
                &format!("{}.{}", slug, self.config.datacenter),
                &self.config.public_ip,
            )
            .await
            .context("Failed to create DNS record")?;

        // Update instance with gateway info
        instance.gateway_slug = Some(slug);
        instance.gateway_subdomain = Some(subdomain);
        instance.gateway_ssh_port = Some(allocation.base);
        instance.gateway_port_range_start = Some(allocation.base);
        instance.gateway_port_range_end = Some(allocation.base + allocation.count - 1);

        tracing::info!(
            "Gateway setup complete: {} -> {}:{} (ports {}-{})",
            instance.gateway_subdomain.as_ref().unwrap(),
            internal_ip,
            instance.ssh_port,
            allocation.base,
            allocation.base + allocation.count - 1
        );

        Ok(instance)
    }

    /// Cleanup gateway for a terminated VM.
    pub async fn cleanup_gateway(&mut self, slug: &str) -> Result<()> {
        // Delete Traefik config
        self.traefik_manager
            .delete_vm_config(slug)
            .context("Failed to delete Traefik config")?;

        // Delete DNS record
        if let Err(e) = self
            .cloudflare
            .delete_a_record(&format!("{}.{}", slug, self.config.datacenter))
            .await
        {
            tracing::warn!("Failed to delete DNS record for {}: {}", slug, e);
        }

        // Free port allocation
        self.port_allocator
            .free(slug)
            .context("Failed to free port allocation")?;

        tracing::info!("Gateway cleanup complete for slug: {}", slug);

        Ok(())
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
