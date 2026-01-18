//! Gateway setup - automates Caddy installation and configuration on Proxmox hosts.
//!
//! Caddy provides automatic HTTPS via Let's Encrypt HTTP-01 challenge.
//! No Cloudflare token needed - certs are obtained automatically.

use anyhow::{bail, Context, Result};
use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};

/// Latest stable Caddy version
const CADDY_VERSION: &str = "2.10.2";

/// Gateway setup configuration
pub struct GatewaySetup {
    pub host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub ssh_password: String,
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
    /// Run the complete gateway setup process.
    pub async fn run(&self) -> Result<GatewaySetupResult> {
        println!("Connecting to host via SSH...");
        let ssh = self.connect_ssh().await?;
        println!("  Connected to {}@{}", self.ssh_user, self.host);

        // Step 1: Configure host networking (IP forwarding, NAT, firewall)
        println!("\nConfiguring host networking...");
        self.configure_host_networking(&ssh).await?;

        // Step 2: Install Caddy binary
        println!("\nInstalling Caddy {}...", CADDY_VERSION);
        self.install_caddy(&ssh).await?;

        // Step 3: Create directories and user
        println!("\nSetting up directories and user...");
        self.setup_directories(&ssh).await?;

        // Step 4: Write Caddy config
        println!("\nWriting Caddy configuration...");
        self.write_caddy_config(&ssh).await?;

        // Step 5: Write systemd service
        println!("\nCreating systemd service...");
        self.write_systemd_service(&ssh).await?;

        // Step 6: Enable and start Caddy
        println!("\nStarting Caddy service...");
        self.start_caddy(&ssh).await?;

        // Step 7: Verify Caddy is running
        println!("\nVerifying Caddy...");
        self.verify_caddy(&ssh).await?;

        Ok(GatewaySetupResult {
            caddy_version: CADDY_VERSION.to_string(),
        })
    }

    async fn connect_ssh(&self) -> Result<Client> {
        let auth = AuthMethod::with_password(&self.ssh_password);
        let client = Client::connect(
            (self.host.as_str(), self.ssh_port),
            &self.ssh_user,
            auth,
            ServerCheckMethod::NoCheck,
        )
        .await
        .context("Failed to connect via SSH")?;
        Ok(client)
    }

    /// Configure host networking: IP forwarding, NAT masquerade, and firewall rules.
    /// All operations are idempotent - safe to run multiple times.
    async fn configure_host_networking(&self, ssh: &Client) -> Result<()> {
        // 1. Enable IP forwarding
        self.enable_ip_forwarding(ssh).await?;

        // 2. Detect if we're behind 1:1 NAT (public IP not on any interface)
        let nat_mode = self.detect_nat_mode(ssh).await?;

        // 3. Detect public interface (for masquerade rules if not NAT mode)
        let public_iface = self.detect_public_interface(ssh).await?;
        println!("  [ok] Public interface: {}", public_iface);

        // 4. Configure NAT masquerade for VM traffic (skip if provider handles 1:1 NAT)
        if nat_mode {
            println!("  [ok] 1:1 NAT detected - skipping masquerade rules (provider handles NAT)");
        } else {
            self.configure_nat_masquerade(ssh, &public_iface).await?;
        }

        // 5. Configure firewall rules
        self.configure_firewall(ssh).await?;

        // 6. Persist iptables rules
        self.persist_iptables(ssh).await?;

        Ok(())
    }

    /// Detect if host is behind 1:1 NAT.
    /// Returns true if public_ip is NOT on any local interface but external services see it.
    async fn detect_nat_mode(&self, ssh: &Client) -> Result<bool> {
        // Check if public IP is assigned to any local interface
        let cmd = format!(
            "ip addr show | grep -q '{}' && echo local || echo not_local",
            self.public_ip
        );
        let result = ssh.execute(&cmd).await?;
        let ip_is_local = result.stdout.trim() == "local";

        if ip_is_local {
            println!("  [ok] Public IP {} is directly assigned", self.public_ip);
            return Ok(false);
        }

        // Public IP not local - verify it's actually our external IP (1:1 NAT)
        let result = ssh
            .execute("curl -sf --max-time 5 https://ifconfig.me 2>/dev/null || curl -sf --max-time 5 https://api.ipify.org 2>/dev/null || echo unknown")
            .await?;
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

    async fn enable_ip_forwarding(&self, ssh: &Client) -> Result<()> {
        // Check current state
        let result = ssh.execute("sysctl -n net.ipv4.ip_forward").await?;
        if result.stdout.trim() == "1" {
            println!("  [ok] IP forwarding already enabled");
            return Ok(());
        }

        // Enable immediately
        let result = ssh.execute("sysctl -w net.ipv4.ip_forward=1").await?;
        if result.exit_status != 0 {
            bail!("Failed to enable IP forwarding: {}", result.stdout);
        }

        // Make persistent
        let cmd = r#"echo "net.ipv4.ip_forward = 1" > /etc/sysctl.d/99-dc-gateway.conf"#;
        let result = ssh.execute(cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to persist IP forwarding: {}", result.stdout);
        }

        println!("  [ok] IP forwarding enabled");
        Ok(())
    }

    async fn detect_public_interface(&self, ssh: &Client) -> Result<String> {
        // Find interface with the public IP
        let cmd = format!(
            "ip -o addr show | grep '{}' | awk '{{print $2}}'",
            self.public_ip
        );
        let result = ssh.execute(&cmd).await?;

        let iface = result.stdout.trim();
        if iface.is_empty() {
            // Fallback: get default route interface
            let result = ssh
                .execute("ip route show default | awk '/default/ {print $5}' | head -1")
                .await?;
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

    async fn configure_nat_masquerade(&self, ssh: &Client, public_iface: &str) -> Result<()> {
        // All RFC1918 private ranges - VMs could get any private IP via DHCP
        const PRIVATE_RANGES: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];

        for subnet in PRIVATE_RANGES {
            // Check if rule already exists
            let check_cmd = format!(
                "iptables -t nat -C POSTROUTING -s {} -o {} -j MASQUERADE 2>/dev/null && echo exists || echo missing",
                subnet, public_iface
            );
            let result = ssh.execute(&check_cmd).await?;

            if result.stdout.trim() == "exists" {
                println!("  [ok] NAT masquerade for {} already configured", subnet);
                continue;
            }

            // Add masquerade rule
            let cmd = format!(
                "iptables -t nat -A POSTROUTING -s {} -o {} -j MASQUERADE",
                subnet, public_iface
            );
            let result = ssh.execute(&cmd).await?;
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

    async fn configure_firewall(&self, ssh: &Client) -> Result<()> {
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
            let result = ssh.execute(&check_cmd).await?;

            if result.stdout.trim() == "exists" {
                println!("  [ok] {} port {} already open", desc, port);
                continue;
            }

            let cmd = format!("iptables -A INPUT -p {} --dport {} -j ACCEPT", proto, port);
            let result = ssh.execute(&cmd).await?;
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
        let result = ssh.execute(&check_cmd).await?;
        if result.stdout.trim() != "exists" {
            let cmd = format!("iptables -A FORWARD {} -j ACCEPT", established_rule);
            let result = ssh.execute(&cmd).await?;
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
            let result = ssh.execute(&check_cmd).await?;
            if result.stdout.trim() != "exists" {
                let cmd = format!("iptables -A FORWARD {} -j ACCEPT", rule);
                let result = ssh.execute(&cmd).await?;
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
            let result = ssh.execute(&check_cmd).await?;
            if result.stdout.trim() != "exists" {
                let cmd = format!("iptables -A FORWARD {} -j ACCEPT", rule);
                let result = ssh.execute(&cmd).await?;
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

    async fn persist_iptables(&self, ssh: &Client) -> Result<()> {
        // Check if iptables-persistent is installed
        let result = ssh
            .execute("dpkg -l iptables-persistent 2>/dev/null | grep -q '^ii' && echo installed || echo missing")
            .await?;

        if result.stdout.trim() == "missing" {
            // Install iptables-persistent non-interactively
            println!("  Installing iptables-persistent...");
            let cmd = "DEBIAN_FRONTEND=noninteractive apt-get install -y iptables-persistent";
            let result = ssh.execute(cmd).await?;
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
        let result = ssh.execute("netfilter-persistent save").await?;
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

    async fn install_caddy(&self, ssh: &Client) -> Result<()> {
        // Check if already installed with correct version
        let check = ssh.execute("caddy version 2>/dev/null || true").await?;
        if check.stdout.contains(CADDY_VERSION) {
            println!("  Caddy {} already installed", CADDY_VERSION);
            return Ok(());
        }

        // Download and install
        let arch = ssh.execute("uname -m").await?;
        let arch = match arch.stdout.trim() {
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

        let result = ssh.execute(&cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to install Caddy: {}", result.stdout);
        }

        // Verify installation
        let verify = ssh.execute("caddy version").await?;
        if !verify.stdout.contains(CADDY_VERSION) {
            bail!("Caddy installation verification failed");
        }

        println!("  [ok] Caddy {} installed", CADDY_VERSION);
        Ok(())
    }

    async fn setup_directories(&self, ssh: &Client) -> Result<()> {
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
            let result = ssh.execute(cmd).await?;
            if result.exit_status != 0 {
                bail!("Failed to setup directories: {} - {}", cmd, result.stdout);
            }
        }

        println!("  [ok] Directories created");
        println!("  [ok] caddy user created");
        Ok(())
    }

    async fn write_caddy_config(&self, ssh: &Client) -> Result<()> {
        let caddyfile = self.generate_caddyfile();

        // Write Caddyfile
        let cmd = format!(
            "cat > /etc/caddy/Caddyfile << 'EOFCONFIG'\n{}\nEOFCONFIG",
            caddyfile
        );
        let result = ssh.execute(&cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to write Caddyfile: {}", result.stdout);
        }
        println!("  [ok] /etc/caddy/Caddyfile");

        // Set ownership and permissions (640 = rw-r-----)
        ssh.execute("chown caddy:caddy /etc/caddy/Caddyfile && chmod 640 /etc/caddy/Caddyfile")
            .await?;

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

    async fn write_systemd_service(&self, ssh: &Client) -> Result<()> {
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
        let result = ssh.execute(&cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to write systemd service: {}", result.stdout);
        }

        // Reload systemd
        let result = ssh.execute("systemctl daemon-reload").await?;
        if result.exit_status != 0 {
            bail!("Failed to reload systemd: {}", result.stdout);
        }

        println!("  [ok] /etc/systemd/system/caddy.service");
        Ok(())
    }

    async fn start_caddy(&self, ssh: &Client) -> Result<()> {
        // Enable and start
        let result = ssh.execute("systemctl enable --now caddy").await?;
        if result.exit_status != 0 {
            bail!("Failed to start Caddy: {}", result.stdout);
        }

        // Wait a moment for startup
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Check status
        let result = ssh.execute("systemctl is-active caddy").await?;
        if result.stdout.trim() != "active" {
            // Get logs for debugging
            let logs = ssh.execute("journalctl -u caddy -n 20 --no-pager").await?;
            bail!(
                "Caddy failed to start. Status: {}. Logs:\n{}",
                result.stdout.trim(),
                logs.stdout
            );
        }

        println!("  [ok] Caddy service started");
        Ok(())
    }

    async fn verify_caddy(&self, ssh: &Client) -> Result<()> {
        // Check if listening on ports 80 and 443
        let result = ssh
            .execute("ss -tlnp | grep -E ':80|:443' | grep caddy || true")
            .await?;
        if result.stdout.is_empty() {
            println!("  [warn] Caddy not yet listening on ports (may still be starting)");
        } else {
            println!("  [ok] Listening on ports 80 and 443");
        }

        // Check for certificate storage directory
        let result = ssh
            .execute("test -d /var/lib/caddy/certificates && echo exists || echo missing")
            .await?;
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
            host: "test".into(),
            ssh_port: 22,
            ssh_user: "root".into(),
            ssh_password: "".into(),
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
}
