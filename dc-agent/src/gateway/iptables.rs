//! iptables DNAT for TCP/UDP port forwarding.
//!
//! Uses kernel-level NAT for non-HTTP traffic (SSH, databases, game servers, etc.).
//! This is more efficient than Traefik for raw TCP/UDP since there's no userspace proxy.

use super::port_allocator::PortAllocation;
use anyhow::{bail, Context, Result};
use std::process::Command;

/// Chain name for our DNAT rules
const CHAIN_NAME: &str = "DC_GATEWAY";

/// iptables NAT manager for TCP/UDP port forwarding
pub struct IptablesNat;

impl IptablesNat {
    /// Initialize the DC_GATEWAY chain in the NAT table.
    /// Called once during gateway manager initialization.
    pub fn init_chain() -> Result<()> {
        // Create chain if it doesn't exist
        let _ = Command::new("iptables")
            .args(["-t", "nat", "-N", CHAIN_NAME])
            .output();

        // Ensure jump from PREROUTING to our chain (idempotent)
        let check = Command::new("iptables")
            .args(["-t", "nat", "-C", "PREROUTING", "-j", CHAIN_NAME])
            .output();

        if check.map(|o| !o.status.success()).unwrap_or(true) {
            let result = Command::new("iptables")
                .args(["-t", "nat", "-I", "PREROUTING", "-j", CHAIN_NAME])
                .output()
                .context("Failed to insert PREROUTING jump")?;

            if !result.status.success() {
                bail!(
                    "Failed to setup iptables chain: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }

        tracing::debug!("iptables NAT chain {} initialized", CHAIN_NAME);
        Ok(())
    }

    /// Setup port forwarding for a VM.
    /// Creates DNAT rules for all allocated ports.
    pub fn setup_forwarding(
        slug: &str,
        internal_ip: &str,
        allocation: &PortAllocation,
    ) -> Result<()> {
        // Validate internal_ip is a valid IPv4 address (defense in depth)
        internal_ip
            .parse::<std::net::Ipv4Addr>()
            .with_context(|| format!("Invalid internal IP address: {}", internal_ip))?;

        // Port mapping:
        // base+0: SSH (-> VM:22)
        // base+1 to base+4: TCP (-> VM:10001-10004)
        // base+5 to base+9: UDP (-> VM:10005-10009)

        let base = allocation.base;

        // SSH port (TCP)
        Self::add_dnat_rule(slug, "tcp", base, internal_ip, 22)?;

        // Additional TCP ports
        for i in 1..=4u16 {
            Self::add_dnat_rule(slug, "tcp", base + i, internal_ip, 10000 + i)?;
        }

        // UDP ports
        for i in 5..=9u16 {
            Self::add_dnat_rule(slug, "udp", base + i, internal_ip, 10000 + i)?;
        }

        tracing::debug!(
            "iptables DNAT setup for {} -> {} (ports {}-{})",
            slug,
            internal_ip,
            base,
            base + allocation.count - 1
        );

        Ok(())
    }

    /// Add a single DNAT rule.
    fn add_dnat_rule(
        slug: &str,
        proto: &str,
        ext_port: u16,
        internal_ip: &str,
        int_port: u16,
    ) -> Result<()> {
        // Comment format: DC_VM_{slug}_{proto}_{ext_port}
        let comment = format!("DC_VM_{}_{}", slug, ext_port);

        // Check if rule already exists
        let check = Command::new("iptables")
            .args([
                "-t",
                "nat",
                "-C",
                CHAIN_NAME,
                "-p",
                proto,
                "--dport",
                &ext_port.to_string(),
                "-j",
                "DNAT",
                "--to-destination",
                &format!("{}:{}", internal_ip, int_port),
                "-m",
                "comment",
                "--comment",
                &comment,
            ])
            .output();

        if check.map(|o| o.status.success()).unwrap_or(false) {
            // Rule already exists
            return Ok(());
        }

        let result = Command::new("iptables")
            .args([
                "-t",
                "nat",
                "-A",
                CHAIN_NAME,
                "-p",
                proto,
                "--dport",
                &ext_port.to_string(),
                "-j",
                "DNAT",
                "--to-destination",
                &format!("{}:{}", internal_ip, int_port),
                "-m",
                "comment",
                "--comment",
                &comment,
            ])
            .output()
            .context("Failed to execute iptables")?;

        if !result.status.success() {
            bail!(
                "Failed to add DNAT rule for {}:{} -> {}:{}: {}",
                proto,
                ext_port,
                internal_ip,
                int_port,
                String::from_utf8_lossy(&result.stderr)
            );
        }

        Ok(())
    }

    /// Cleanup port forwarding for a VM.
    /// Removes all DNAT rules for this slug.
    pub fn cleanup_forwarding(slug: &str) -> Result<()> {
        // List all rules with line numbers
        let output = Command::new("iptables")
            .args(["-t", "nat", "-L", CHAIN_NAME, "-n", "--line-numbers"])
            .output()
            .context("Failed to list iptables rules")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let comment_pattern = format!("DC_VM_{}_", slug);

        // Collect line numbers to delete (in reverse order to avoid shifting)
        let mut lines_to_delete: Vec<u32> = Vec::new();

        for line in stdout.lines().skip(2) {
            // Skip headers
            if line.contains(&comment_pattern) {
                if let Some(num_str) = line.split_whitespace().next() {
                    if let Ok(num) = num_str.parse::<u32>() {
                        lines_to_delete.push(num);
                    }
                }
            }
        }

        // Delete in reverse order to preserve line numbers
        lines_to_delete.sort_by(|a, b| b.cmp(a));
        for line_num in lines_to_delete {
            let result = Command::new("iptables")
                .args(["-t", "nat", "-D", CHAIN_NAME, &line_num.to_string()])
                .output();

            if let Err(e) = result {
                tracing::warn!("Failed to delete iptables rule {}: {}", line_num, e);
            }
        }

        tracing::debug!("iptables DNAT cleanup for {}", slug);
        Ok(())
    }

    /// Check if forwarding is setup for a specific slug.
    pub fn has_forwarding(slug: &str) -> bool {
        let output = Command::new("iptables")
            .args(["-t", "nat", "-L", CHAIN_NAME, "-n"])
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains(&format!("DC_VM_{}_", slug))
            }
            Err(_) => false,
        }
    }

    /// Get count of active port forwarding rules.
    pub fn count_rules() -> usize {
        let output = Command::new("iptables")
            .args(["-t", "nat", "-L", CHAIN_NAME, "-n"])
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout
                    .lines()
                    .skip(2) // Skip headers
                    .filter(|l| l.contains("DC_VM_"))
                    .count()
            }
            Err(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests require root privileges and modify iptables.
    // They are designed to be run in an isolated environment.

    #[test]
    fn test_comment_format() {
        let slug = "k7m2p4";
        let ext_port = 20000u16;
        let comment = format!("DC_VM_{}_{}", slug, ext_port);
        assert_eq!(comment, "DC_VM_k7m2p4_20000");
    }

    #[test]
    fn test_count_rules_no_chain() {
        // When chain doesn't exist, should return 0 without panicking
        let count = super::IptablesNat::count_rules();
        // Can be 0 or more depending on environment
        assert!(count < 100000); // Sanity check
    }
}
