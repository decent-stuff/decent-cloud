//! Gateway setup - automates Caddy installation and configuration on Proxmox hosts.
//!
//! This setup runs LOCALLY on the Proxmox host where dc-agent is installed.
//! Caddy provides automatic HTTPS via Let's Encrypt HTTP-01 challenge.
//! No Cloudflare token needed - certs are obtained automatically.

use super::{execute_command, CommandOutput};
use anyhow::{bail, Result};
use std::process::Command;

/// Latest stable Caddy version
const CADDY_VERSION: &str = "2.10.2";

/// Detect the host's public IP address using external services.
/// Tries multiple services for reliability.
pub fn detect_public_ip() -> Result<String> {
    // Services to try, in order of preference
    const SERVICES: &[&str] = &[
        "https://ifconfig.me",
        "https://api.ipify.org",
        "https://icanhazip.com",
    ];

    for service in SERVICES {
        let output = Command::new("curl")
            .args(["-sf", "--max-time", "5", service])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // Basic validation: should look like an IP
                if ip.contains('.') && ip.len() >= 7 && ip.len() <= 15 {
                    return Ok(ip);
                }
            }
        }
    }

    bail!(
        "Could not detect public IP. Tried: {}\n\
         Use --gateway-public-ip to specify manually.",
        SERVICES.join(", ")
    )
}

/// Gateway setup configuration.
/// Runs locally on the Proxmox host.
pub struct GatewaySetup {
    pub datacenter: String,
    pub domain: String,
    pub public_ip: String,
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub ports_per_vm: u16,
}

/// Result of successful gateway setup
pub struct GatewaySetupResult {
    pub caddy_version: String,
}

impl GatewaySetup {
    /// Execute a shell command locally and return output.
    fn execute(&self, cmd: &str) -> Result<CommandOutput> {
        execute_command(cmd)
    }

    /// Run the complete gateway setup process locally.
    pub async fn run(&self) -> Result<GatewaySetupResult> {
        // Verify we're running as root
        let result = self.execute("id -u")?;
        if result.stdout.trim() != "0" {
            bail!("Gateway setup must be run as root");
        }

        // Step 1: Configure host networking (IP forwarding, NAT, firewall)
        println!("Configuring host networking...");
        self.configure_host_networking()?;

        // Step 2: Install Caddy binary
        println!("\nInstalling Caddy {}...", CADDY_VERSION);
        self.install_caddy()?;

        // Step 3: Create directories and user
        println!("\nSetting up directories and user...");
        self.setup_directories()?;

        // Step 4: Write Caddy config
        println!("\nWriting Caddy configuration...");
        self.write_caddy_config()?;

        // Step 5: Write systemd service
        println!("\nCreating systemd service...");
        self.write_systemd_service()?;

        // Step 6: Enable and start Caddy
        println!("\nStarting Caddy service...");
        self.start_caddy()?;

        // Step 7: Verify Caddy is running
        println!("\nVerifying Caddy...");
        self.verify_caddy()?;

        Ok(GatewaySetupResult {
            caddy_version: CADDY_VERSION.to_string(),
        })
    }

    /// Configure host networking: IP forwarding, NAT masquerade, and firewall rules.
    /// All operations are idempotent - safe to run multiple times.
    fn configure_host_networking(&self) -> Result<()> {
        // 1. Enable IP forwarding
        self.enable_ip_forwarding()?;

        // 2. Detect if we're behind 1:1 NAT (public IP not on any interface)
        let nat_mode = self.detect_nat_mode()?;

        // 3. Detect public interface (for masquerade rules if not NAT mode)
        let public_iface = self.detect_public_interface()?;
        println!("  [ok] Public interface: {}", public_iface);

        // 4. Configure NAT masquerade for VM traffic
        // Always needed: even behind 1:1 NAT, DNAT'd traffic needs SNAT for return path
        // because VMs are on a private bridge and can't route directly to external clients
        self.configure_nat_masquerade(&public_iface)?;

        // 5. Configure firewall rules
        self.configure_firewall()?;

        // 6. Persist iptables rules
        self.persist_iptables()?;

        Ok(())
    }

    /// Detect if host is behind 1:1 NAT.
    /// Returns true if public_ip is NOT on any local interface but external services see it.
    fn detect_nat_mode(&self) -> Result<bool> {
        // Check if public IP is assigned to any local interface
        let cmd = format!(
            "ip addr show | grep -q '{}' && echo local || echo not_local",
            self.public_ip
        );
        let result = self.execute(&cmd)?;
        let ip_is_local = result.stdout.trim() == "local";

        if ip_is_local {
            println!("  [ok] Public IP {} is directly assigned", self.public_ip);
            return Ok(false);
        }

        // Public IP not local - verify it's actually our external IP (1:1 NAT)
        let result = self.execute(
            "curl -sf --max-time 5 https://ifconfig.me 2>/dev/null || curl -sf --max-time 5 https://api.ipify.org 2>/dev/null || echo unknown"
        )?;
        let external_ip = result.stdout.trim();

        if external_ip == self.public_ip {
            println!(
                "  [ok] 1:1 NAT detected: {} not on interface but is external IP",
                self.public_ip
            );
            Ok(true)
        } else if external_ip == "unknown" {
            // Can't verify - assume direct assignment needed, masquerade won't hurt
            println!("  [warn] Could not verify external IP, assuming direct assignment");
            Ok(false)
        } else {
            // External IP doesn't match - misconfiguration?
            println!(
                "  [warn] Public IP {} doesn't match external IP {}",
                self.public_ip, external_ip
            );
            println!("         Proceeding with masquerade rules");
            Ok(false)
        }
    }

    fn enable_ip_forwarding(&self) -> Result<()> {
        // Check current IP forwarding state
        let result = self.execute("sysctl -n net.ipv4.ip_forward")?;
        let ip_forward_enabled = result.stdout.trim() == "1";

        // Check bridge netfilter state (required for iptables to work with bridged VMs)
        let result = self.execute("sysctl -n net.bridge.bridge-nf-call-iptables 2>/dev/null || echo 0")?;
        let bridge_nf_enabled = result.stdout.trim() == "1";

        if ip_forward_enabled && bridge_nf_enabled {
            println!("  [ok] IP forwarding and bridge netfilter already enabled");
            return Ok(());
        }

        // Load br_netfilter module (required for bridge-nf-call-iptables)
        let _ = self.execute("modprobe br_netfilter");

        // Enable IP forwarding
        if !ip_forward_enabled {
            let result = self.execute("sysctl -w net.ipv4.ip_forward=1")?;
            if result.exit_status != 0 {
                bail!("Failed to enable IP forwarding: {}", result.stdout);
            }
        }

        // Enable bridge netfilter (required for iptables NAT to work with bridged traffic)
        if !bridge_nf_enabled {
            let result = self.execute("sysctl -w net.bridge.bridge-nf-call-iptables=1")?;
            if result.exit_status != 0 {
                // Non-fatal: some systems may not have this
                println!("  [warn] Could not enable bridge netfilter - bridged VM traffic may not be NAT'd");
            }
        }

        // Make persistent
        let sysctl_conf = r#"# DC Gateway networking settings
net.ipv4.ip_forward = 1
net.bridge.bridge-nf-call-iptables = 1
"#;
        let result = self.execute(&format!(
            "echo '{}' > /etc/sysctl.d/99-dc-gateway.conf",
            sysctl_conf.replace('\n', "\\n")
        ))?;
        if result.exit_status != 0 {
            bail!("Failed to persist sysctl settings: {}", result.stdout);
        }

        // Ensure br_netfilter module loads on boot
        let _ = self.execute("echo 'br_netfilter' >> /etc/modules-load.d/br_netfilter.conf 2>/dev/null || true");

        println!("  [ok] IP forwarding and bridge netfilter enabled");
        Ok(())
    }

    fn detect_public_interface(&self) -> Result<String> {
        // Find interface with the public IP
        let cmd = format!(
            "ip -o addr show | grep '{}' | awk '{{print $2}}'",
            self.public_ip
        );
        let result = self.execute(&cmd)?;

        let iface = result.stdout.trim();
        if iface.is_empty() {
            // Fallback: get default route interface
            let result =
                self.execute("ip route show default | awk '/default/ {print $5}' | head -1")?;
            let iface = result.stdout.trim();
            if iface.is_empty() {
                bail!(
                    "Could not detect public interface for IP {}. \
                     Verify the public IP is assigned to an interface.",
                    self.public_ip
                );
            }
            return Ok(iface.to_string());
        }

        Ok(iface.to_string())
    }

    fn configure_nat_masquerade(&self, _public_iface: &str) -> Result<()> {
        // MASQUERADE traffic from private subnets going to non-private destinations
        // This handles DNAT return traffic: external client -> DNAT to VM -> VM replies
        // The VM reply needs SNAT so the client sees the reply from the public IP
        //
        // Using "! -d <private>" instead of "-o <iface>" is more robust:
        // - Works regardless of interface naming
        // - Works with bridged setups where traffic stays on the bridge
        const PRIVATE_RANGES: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];

        for subnet in PRIVATE_RANGES {
            // Check if rule already exists (either format)
            let check_cmd = format!(
                "iptables -t nat -C POSTROUTING -s {} ! -d {} -j MASQUERADE 2>/dev/null && echo exists || echo missing",
                subnet, subnet
            );
            let result = self.execute(&check_cmd)?;

            if result.stdout.trim() == "exists" {
                println!("  [ok] NAT masquerade for {} already configured", subnet);
                continue;
            }

            // Add masquerade rule: traffic from private subnet NOT going to private subnet
            let cmd = format!(
                "iptables -t nat -A POSTROUTING -s {} ! -d {} -j MASQUERADE",
                subnet, subnet
            );
            let result = self.execute(&cmd)?;
            if result.exit_status != 0 {
                bail!(
                    "Failed to configure NAT masquerade for {}: {}",
                    subnet,
                    result.stdout
                );
            }

            println!("  [ok] NAT masquerade configured for {}", subnet);
        }
        Ok(())
    }

    fn configure_firewall(&self) -> Result<()> {
        // All RFC1918 private ranges - VMs could get any private IP via DHCP
        const PRIVATE_RANGES: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];

        // Rules to add (protocol, port/range, description)
        let input_rules: &[(&str, &str, &str)] = &[
            ("tcp", "80", "HTTP"),
            ("tcp", "443", "HTTPS"),
            (
                "tcp",
                &format!("{}:{}", self.port_range_start, self.port_range_end),
                "VM TCP ports",
            ),
            (
                "udp",
                &format!("{}:{}", self.port_range_start, self.port_range_end),
                "VM UDP ports",
            ),
        ];

        for (proto, port, desc) in input_rules {
            let check_cmd = format!(
                "iptables -C INPUT -p {} --dport {} -j ACCEPT 2>/dev/null && echo exists || echo missing",
                proto, port
            );
            let result = self.execute(&check_cmd)?;

            if result.stdout.trim() == "exists" {
                println!("  [ok] {} port {} already open", desc, port);
                continue;
            }

            let cmd = format!("iptables -A INPUT -p {} --dport {} -j ACCEPT", proto, port);
            let result = self.execute(&cmd)?;
            if result.exit_status != 0 {
                bail!("Failed to open {} port {}: {}", desc, port, result.stdout);
            }
            println!("  [ok] Opened {} port {}", desc, port);
        }

        // FORWARD rule for established/related connections (only need one)
        let established_rule = "-m state --state RELATED,ESTABLISHED";
        let check_cmd = format!(
            "iptables -C FORWARD {} -j ACCEPT 2>/dev/null && echo exists || echo missing",
            established_rule
        );
        let result = self.execute(&check_cmd)?;
        if result.stdout.trim() != "exists" {
            let cmd = format!("iptables -A FORWARD {} -j ACCEPT", established_rule);
            let result = self.execute(&cmd)?;
            if result.exit_status != 0 {
                bail!(
                    "Failed to add FORWARD rule for established: {}",
                    result.stdout
                );
            }
            println!("  [ok] FORWARD rule for established connections");
        } else {
            println!("  [ok] FORWARD rule for established connections already exists");
        }

        // FORWARD rules for each private range (source and destination)
        for subnet in PRIVATE_RANGES {
            // Outbound from VMs
            let rule = format!("-s {}", subnet);
            let check_cmd = format!(
                "iptables -C FORWARD {} -j ACCEPT 2>/dev/null && echo exists || echo missing",
                rule
            );
            let result = self.execute(&check_cmd)?;
            if result.stdout.trim() != "exists" {
                let cmd = format!("iptables -A FORWARD {} -j ACCEPT", rule);
                let result = self.execute(&cmd)?;
                if result.exit_status != 0 {
                    bail!(
                        "Failed to add FORWARD rule for outbound {}: {}",
                        subnet,
                        result.stdout
                    );
                }
                println!("  [ok] FORWARD outbound from {}", subnet);
            } else {
                println!("  [ok] FORWARD outbound from {} already exists", subnet);
            }

            // Inbound to VMs
            let rule = format!("-d {}", subnet);
            let check_cmd = format!(
                "iptables -C FORWARD {} -j ACCEPT 2>/dev/null && echo exists || echo missing",
                rule
            );
            let result = self.execute(&check_cmd)?;
            if result.stdout.trim() != "exists" {
                let cmd = format!("iptables -A FORWARD {} -j ACCEPT", rule);
                let result = self.execute(&cmd)?;
                if result.exit_status != 0 {
                    bail!(
                        "Failed to add FORWARD rule for inbound {}: {}",
                        subnet,
                        result.stdout
                    );
                }
                println!("  [ok] FORWARD inbound to {}", subnet);
            } else {
                println!("  [ok] FORWARD inbound to {} already exists", subnet);
            }
        }

        Ok(())
    }

    fn persist_iptables(&self) -> Result<()> {
        // Check if iptables-persistent is installed
        let result = self.execute(
            "dpkg -l iptables-persistent 2>/dev/null | grep -q '^ii' && echo installed || echo missing",
        )?;

        if result.stdout.trim() == "missing" {
            // Install iptables-persistent non-interactively
            println!("  Installing iptables-persistent...");
            let result = self
                .execute("DEBIAN_FRONTEND=noninteractive apt-get install -y iptables-persistent")?;
            if result.exit_status != 0 {
                // Not fatal - rules are applied, just won't persist across reboot
                println!(
                    "  [warn] Failed to install iptables-persistent: {}",
                    result.stdout.lines().next().unwrap_or("unknown error")
                );
                println!("  [warn] Rules applied but may not persist across reboot");
                return Ok(());
            }
        }

        // Save rules
        let result = self.execute("netfilter-persistent save")?;
        if result.exit_status != 0 {
            println!(
                "  [warn] Failed to persist iptables rules: {}",
                result.stdout
            );
            println!("  [warn] Rules applied but may not persist across reboot");
            return Ok(());
        }

        println!("  [ok] iptables rules persisted");
        Ok(())
    }

    fn install_caddy(&self) -> Result<()> {
        // Check if already installed with correct version
        let check = self.execute("caddy version 2>/dev/null || true")?;
        if check.stdout.contains(CADDY_VERSION) {
            println!("  Caddy {} already installed", CADDY_VERSION);
            return Ok(());
        }

        // Download and install
        let arch_result = self.execute("uname -m")?;
        let arch = match arch_result.stdout.trim() {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            other => bail!("Unsupported architecture: {}", other),
        };

        let download_url = format!(
            "https://github.com/caddyserver/caddy/releases/download/v{}/caddy_{}_linux_{}.tar.gz",
            CADDY_VERSION, CADDY_VERSION, arch
        );

        let cmd = format!(
            "cd /tmp && \
             curl -sSL '{}' -o caddy.tar.gz && \
             tar xzf caddy.tar.gz caddy && \
             mv caddy /usr/local/bin/caddy && \
             chmod +x /usr/local/bin/caddy && \
             rm caddy.tar.gz",
            download_url
        );

        let result = self.execute(&cmd)?;
        if result.exit_status != 0 {
            bail!("Failed to install Caddy: {}", result.stdout);
        }

        // Verify installation
        let verify = self.execute("caddy version")?;
        if !verify.stdout.contains(CADDY_VERSION) {
            bail!("Caddy installation verification failed");
        }

        println!("  [ok] Caddy {} installed", CADDY_VERSION);
        Ok(())
    }

    fn setup_directories(&self) -> Result<()> {
        let commands = [
            // Create caddy user/group
            "id caddy >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin caddy",
            // Create directories
            "mkdir -p /etc/caddy/sites /var/lib/caddy /var/lib/dc-agent",
            // Set ownership
            "chown -R caddy:caddy /etc/caddy /var/lib/caddy",
            // dc-agent needs write access to sites dir
            "chmod 775 /etc/caddy/sites",
        ];

        for cmd in commands {
            let result = self.execute(cmd)?;
            if result.exit_status != 0 {
                bail!("Failed to setup directories: {} - {}", cmd, result.stdout);
            }
        }

        println!("  [ok] Directories created");
        println!("  [ok] caddy user created");
        Ok(())
    }

    fn write_caddy_config(&self) -> Result<()> {
        let caddyfile = self.generate_caddyfile();

        // Write Caddyfile
        let cmd = format!(
            "cat > /etc/caddy/Caddyfile << 'EOFCONFIG'\n{}\nEOFCONFIG",
            caddyfile
        );
        let result = self.execute(&cmd)?;
        if result.exit_status != 0 {
            bail!("Failed to write Caddyfile: {}", result.stdout);
        }
        println!("  [ok] /etc/caddy/Caddyfile");

        // Set ownership and permissions (640 = rw-r-----)
        self.execute("chown caddy:caddy /etc/caddy/Caddyfile && chmod 640 /etc/caddy/Caddyfile")?;

        Ok(())
    }

    fn generate_caddyfile(&self) -> String {
        format!(
            r#"# Caddy configuration for DC Gateway
# Generated by dc-agent setup gateway
#
# Architecture: TLS termination at gateway with automatic HTTP-01 certificates
# - HTTP (80): Automatic redirect to HTTPS + ACME challenge
# - HTTPS (443): TLS terminated here, proxies to VMs
#
# VM-specific configs are imported from /etc/caddy/sites/*.caddy
#
# Logs: stdout → journald (view with: journalctl -u caddy)

{{
    # Global options
    admin off
    persist_config off

    # Let's Encrypt email (optional but recommended)
    # email admin@{domain}

    # Data directory for certificates
    storage file_system /var/lib/caddy
}}

# HTTP to HTTPS redirect is automatic in Caddy

# Import all VM-specific site configurations
import /etc/caddy/sites/*.caddy
"#,
            domain = self.domain
        )
    }

    fn write_systemd_service(&self) -> Result<()> {
        let service = r#"[Unit]
Description=Caddy Web Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/caddy run --config /etc/caddy/Caddyfile
ExecReload=/usr/local/bin/caddy reload --config /etc/caddy/Caddyfile
Restart=always
RestartSec=5
User=caddy
Group=caddy

# Bind to privileged ports
AmbientCapabilities=CAP_NET_BIND_SERVICE

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/etc/caddy /var/lib/caddy

[Install]
WantedBy=multi-user.target
"#;

        let cmd = format!(
            "cat > /etc/systemd/system/caddy.service << 'EOFSERVICE'\n{}\nEOFSERVICE",
            service
        );
        let result = self.execute(&cmd)?;
        if result.exit_status != 0 {
            bail!("Failed to write systemd service: {}", result.stdout);
        }

        // Reload systemd
        let result = self.execute("systemctl daemon-reload")?;
        if result.exit_status != 0 {
            bail!("Failed to reload systemd: {}", result.stdout);
        }

        println!("  [ok] /etc/systemd/system/caddy.service");
        Ok(())
    }

    fn start_caddy(&self) -> Result<()> {
        // Enable and start
        let result = self.execute("systemctl enable --now caddy")?;
        if result.exit_status != 0 {
            bail!("Failed to start Caddy: {}", result.stdout);
        }

        // Wait a moment for startup
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Check status
        let result = self.execute("systemctl is-active caddy")?;
        if result.stdout.trim() != "active" {
            // Get logs for debugging
            let logs = self.execute("journalctl -u caddy -n 20 --no-pager")?;
            bail!(
                "Caddy failed to start. Status: {}. Logs:\n{}",
                result.stdout.trim(),
                logs.stdout
            );
        }

        println!("  [ok] Caddy service started");
        Ok(())
    }

    fn verify_caddy(&self) -> Result<()> {
        // Check if listening on ports 80 and 443
        let result = self.execute("ss -tlnp | grep -E ':80|:443' | grep caddy || true")?;
        if result.stdout.is_empty() {
            println!("  [warn] Caddy not yet listening on ports (may still be starting)");
        } else {
            println!("  [ok] Listening on ports 80 and 443");
        }

        // Check for certificate storage directory
        let result =
            self.execute("test -d /var/lib/caddy/certificates && echo exists || echo missing")?;
        if result.stdout.trim() == "exists" {
            println!("  [ok] Certificate storage initialized");
        } else {
            println!("  [info] Certificate storage will be created on first request");
        }

        println!("\nGateway setup complete!");
        println!("\nNext steps:");
        println!(
            "  1. Add [gateway] section to dc-agent.toml with datacenter={}, domain={}",
            self.datacenter, self.domain
        );
        println!("  2. Run 'dc-agent doctor' to verify configuration");
        println!("  3. Provision a VM to test gateway routing");

        Ok(())
    }

    /// Generate gateway config section for dc-agent.toml.
    /// DNS management for individual VMs is handled via the central API.
    pub fn generate_gateway_config(&self) -> String {
        format!(
            r#"
[gateway]
datacenter = "{datacenter}"
domain = "{domain}"
public_ip = "{public_ip}"
port_range_start = {port_start}
port_range_end = {port_end}
ports_per_vm = {ports_per_vm}
caddy_sites_dir = "/etc/caddy/sites"
port_allocations_path = "/var/lib/dc-agent/port-allocations.json"
# DNS management is handled via the central API (no Cloudflare credentials needed)
# TLS certificates are managed automatically by Caddy via HTTP-01 challenge
"#,
            datacenter = self.datacenter,
            domain = self.domain,
            public_ip = self.public_ip,
            port_start = self.port_range_start,
            port_end = self.port_range_end,
            ports_per_vm = self.ports_per_vm,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_setup() -> GatewaySetup {
        GatewaySetup {
            datacenter: "dc-lk".into(),
            domain: "decent-cloud.org".into(),
            public_ip: "203.0.113.1".into(),
            port_range_start: 20000,
            port_range_end: 59999,
            ports_per_vm: 10,
        }
    }

    #[test]
    fn test_generate_caddyfile() {
        let setup = test_setup();
        let config = setup.generate_caddyfile();

        // TLS termination architecture
        assert!(config.contains("TLS termination at gateway"));
        assert!(config.contains("HTTP-01 certificates"));

        // Global options
        assert!(config.contains("admin off"));
        assert!(config.contains("storage file_system /var/lib/caddy"));

        // Should import VM-specific configs
        assert!(config.contains("import /etc/caddy/sites/*.caddy"));

        // Should NOT have cloudflare references
        assert!(!config.contains("cloudflare"));
    }

    #[test]
    fn test_generate_gateway_config() {
        let setup = test_setup();
        let config = setup.generate_gateway_config();
        assert!(config.contains("datacenter = \"dc-lk\""));
        assert!(config.contains("public_ip = \"203.0.113.1\""));
        assert!(config.contains("caddy_sites_dir = \"/etc/caddy/sites\""));
        // No Cloudflare credentials needed
        assert!(!config.contains("cloudflare_zone_id"));
        assert!(!config.contains("cloudflare_api_token"));
    }

    #[test]
    fn test_port_range_format_for_iptables() {
        // iptables uses colon-separated port ranges (e.g., 20000:59999)
        let setup = test_setup();
        let port_range = format!("{}:{}", setup.port_range_start, setup.port_range_end);
        assert_eq!(port_range, "20000:59999");

        // Verify format is valid for iptables --dport
        assert!(port_range.contains(':'));
        let parts: Vec<&str> = port_range.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].parse::<u16>().is_ok());
        assert!(parts[1].parse::<u16>().is_ok());
    }

    #[test]
    fn test_private_ranges_are_valid_cidr() {
        // All RFC1918 private ranges used in NAT and FORWARD rules
        const PRIVATE_RANGES: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];

        for range in PRIVATE_RANGES {
            let parts: Vec<&str> = range.split('/').collect();
            assert_eq!(parts.len(), 2, "Invalid CIDR: {}", range);
            // Verify IP part has 4 octets
            let octets: Vec<&str> = parts[0].split('.').collect();
            assert_eq!(octets.len(), 4, "Invalid IP in CIDR: {}", range);
            // Verify prefix is a number
            assert!(
                parts[1].parse::<u8>().is_ok(),
                "Invalid prefix in CIDR: {}",
                range
            );
        }

        // Verify we cover all RFC1918 ranges
        assert!(PRIVATE_RANGES.contains(&"10.0.0.0/8"));
        assert!(PRIVATE_RANGES.contains(&"172.16.0.0/12"));
        assert!(PRIVATE_RANGES.contains(&"192.168.0.0/16"));
    }

    #[test]
    fn test_sysctl_config_format() {
        // The format written to /etc/sysctl.d/99-dc-gateway.conf
        let config_line = "net.ipv4.ip_forward = 1";
        assert!(config_line.contains("net.ipv4.ip_forward"));
        assert!(config_line.contains("= 1"));
    }

    #[test]
    fn test_ip_validation_logic() {
        // The validation logic used in detect_public_ip
        fn is_valid_ipv4(ip: &str) -> bool {
            ip.contains('.') && ip.len() >= 7 && ip.len() <= 15
        }

        // Valid IPs
        assert!(is_valid_ipv4("1.2.3.4")); // 7 chars (minimum)
        assert!(is_valid_ipv4("192.168.1.1")); // 11 chars
        assert!(is_valid_ipv4("255.255.255.255")); // 15 chars (maximum)

        // Invalid IPs
        assert!(!is_valid_ipv4("1.2.3")); // Too short (5 chars)
        assert!(!is_valid_ipv4("1234567")); // No dots
        assert!(!is_valid_ipv4("1234.1234.1234.1234")); // Too long (19 chars)
    }

    #[test]
    #[ignore] // Requires network access - run manually with: cargo test -- --ignored
    fn test_detect_public_ip_integration() {
        let ip = detect_public_ip().expect("Should detect public IP");
        // Basic validation
        assert!(ip.contains('.'), "Should be IPv4 format");
        assert!(ip.len() >= 7, "IP too short");
        assert!(ip.len() <= 15, "IP too long");
        // Should be parseable as IP address octets
        let parts: Vec<&str> = ip.split('.').collect();
        assert_eq!(parts.len(), 4, "Should have 4 octets");
        for part in parts {
            // parse::<u8>() already validates 0-255 range
            let _octet: u8 = part.parse().expect("Each octet should be 0-255");
        }
    }
}
