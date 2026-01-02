//! Bandwidth monitoring via iptables accounting chains.
//!
//! Creates per-VM iptables chains to track bytes in/out.
//! Machine-parsable output for UI display.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

/// Bandwidth statistics for a VM
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BandwidthStats {
    /// Bytes received by the VM
    pub bytes_in: u64,
    /// Bytes sent by the VM
    pub bytes_out: u64,
    /// Packets received
    pub packets_in: u64,
    /// Packets sent
    pub packets_out: u64,
}

/// Bandwidth monitor using iptables accounting
pub struct BandwidthMonitor;

impl BandwidthMonitor {
    /// Chain name prefix for VM accounting
    const CHAIN_PREFIX: &'static str = "DC_VM_";

    /// Setup iptables accounting chains for a VM.
    /// Called during VM provisioning.
    pub fn setup_accounting(slug: &str, internal_ip: &str) -> Result<()> {
        let chain_name = format!("{}{}", Self::CHAIN_PREFIX, slug);

        // Create chain if it doesn't exist
        let _ = Command::new("iptables")
            .args(["-N", &chain_name])
            .output();

        // Flush any existing rules
        Command::new("iptables")
            .args(["-F", &chain_name])
            .output()
            .context("Failed to flush iptables chain")?;

        // Add rules to count traffic to/from this VM
        // Incoming traffic (to VM)
        Command::new("iptables")
            .args([
                "-A", &chain_name,
                "-d", internal_ip,
                "-j", "RETURN",
            ])
            .output()
            .context("Failed to add incoming traffic rule")?;

        // Outgoing traffic (from VM)
        Command::new("iptables")
            .args([
                "-A", &chain_name,
                "-s", internal_ip,
                "-j", "RETURN",
            ])
            .output()
            .context("Failed to add outgoing traffic rule")?;

        // Insert jump to our chain from FORWARD chain (if not already present)
        let check = Command::new("iptables")
            .args(["-C", "FORWARD", "-j", &chain_name])
            .output();

        if check.map(|o| !o.status.success()).unwrap_or(true) {
            Command::new("iptables")
                .args(["-I", "FORWARD", "-j", &chain_name])
                .output()
                .context("Failed to insert jump rule")?;
        }

        tracing::debug!("Setup iptables accounting for {} ({})", slug, internal_ip);
        Ok(())
    }

    /// Remove iptables accounting chains for a VM.
    /// Called during VM termination.
    pub fn cleanup_accounting(slug: &str) -> Result<()> {
        let chain_name = format!("{}{}", Self::CHAIN_PREFIX, slug);

        // Remove jump from FORWARD chain
        let _ = Command::new("iptables")
            .args(["-D", "FORWARD", "-j", &chain_name])
            .output();

        // Flush and delete chain
        let _ = Command::new("iptables")
            .args(["-F", &chain_name])
            .output();

        let _ = Command::new("iptables")
            .args(["-X", &chain_name])
            .output();

        tracing::debug!("Cleaned up iptables accounting for {}", slug);
        Ok(())
    }

    /// Get bandwidth statistics for a specific VM.
    pub fn get_stats(slug: &str) -> Result<BandwidthStats> {
        let chain_name = format!("{}{}", Self::CHAIN_PREFIX, slug);

        // Get stats with exact byte counts: iptables -L <chain> -v -n -x
        let output = Command::new("iptables")
            .args(["-L", &chain_name, "-v", "-n", "-x"])
            .output()
            .context("Failed to query iptables")?;

        if !output.status.success() {
            return Ok(BandwidthStats::default());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_iptables_output(&stdout)
    }

    /// Get bandwidth statistics for all VMs.
    pub fn get_all_stats() -> Result<HashMap<String, BandwidthStats>> {
        let mut stats = HashMap::new();

        // List all chains
        let output = Command::new("iptables")
            .args(["-L", "-n"])
            .output()
            .context("Failed to list iptables chains")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Find all DC_VM_* chains
        for line in stdout.lines() {
            if line.starts_with("Chain DC_VM_") {
                if let Some(slug) = line
                    .strip_prefix("Chain DC_VM_")
                    .and_then(|s| s.split_whitespace().next())
                {
                    if let Ok(vm_stats) = Self::get_stats(slug) {
                        stats.insert(slug.to_string(), vm_stats);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Parse iptables -L -v -n -x output
    fn parse_iptables_output(output: &str) -> Result<BandwidthStats> {
        let mut stats = BandwidthStats::default();

        for line in output.lines().skip(2) {
            // Skip header lines
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                continue;
            }

            // Format: pkts bytes target prot opt in out source destination
            let packets: u64 = parts[0].parse().unwrap_or(0);
            let bytes: u64 = parts[1].parse().unwrap_or(0);
            let source = parts.get(7).unwrap_or(&"");
            let dest = parts.get(8).unwrap_or(&"");

            // If source is specific IP (not 0.0.0.0/0), it's outgoing
            // If dest is specific IP (not 0.0.0.0/0), it's incoming
            if !source.contains("0.0.0.0") && *source != "anywhere" {
                stats.bytes_out = bytes;
                stats.packets_out = packets;
            } else if !dest.contains("0.0.0.0") && *dest != "anywhere" {
                stats.bytes_in = bytes;
                stats.packets_in = packets;
            }
        }

        Ok(stats)
    }

    /// Reset counters for a VM (useful for billing periods).
    pub fn reset_counters(slug: &str) -> Result<()> {
        let chain_name = format!("{}{}", Self::CHAIN_PREFIX, slug);

        Command::new("iptables")
            .args(["-Z", &chain_name])
            .output()
            .context("Failed to reset iptables counters")?;

        tracing::debug!("Reset bandwidth counters for {}", slug);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iptables_output() {
        let output = r#"Chain DC_VM_k7m2p4 (1 references)
 pkts bytes target     prot opt in     out     source               destination
  100  50000 RETURN     all  --  *      *       0.0.0.0/0            10.0.1.5
  200 100000 RETURN     all  --  *      *       10.0.1.5             0.0.0.0/0
"#;

        let stats = BandwidthMonitor::parse_iptables_output(output).unwrap();
        assert_eq!(stats.bytes_in, 50000);
        assert_eq!(stats.packets_in, 100);
        assert_eq!(stats.bytes_out, 100000);
        assert_eq!(stats.packets_out, 200);
    }

    #[test]
    fn test_parse_empty_output() {
        let output = r#"Chain DC_VM_test (0 references)
 pkts bytes target     prot opt in     out     source               destination
"#;

        let stats = BandwidthMonitor::parse_iptables_output(output).unwrap();
        assert_eq!(stats.bytes_in, 0);
        assert_eq!(stats.bytes_out, 0);
    }
}
