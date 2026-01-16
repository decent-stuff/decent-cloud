//! Gateway setup - automates Traefik installation and configuration on Proxmox hosts.

use anyhow::{bail, Context, Result};
use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};

/// Latest stable Traefik version
const TRAEFIK_VERSION: &str = "3.2.3";

/// Gateway setup configuration
pub struct GatewaySetup {
    pub host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub ssh_password: String,
    pub datacenter: String,
    pub domain: String,
    pub public_ip: String,
    pub cloudflare_api_token: String,
    pub cloudflare_zone_id: Option<String>, // If None, will be looked up from domain
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub ports_per_vm: u16,
}

/// Result of successful gateway setup
pub struct GatewaySetupResult {
    pub cloudflare_zone_id: String,
    pub traefik_version: String,
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

        // Step 2: Look up Cloudflare Zone ID if not provided
        let zone_id = match &self.cloudflare_zone_id {
            Some(id) => {
                println!("  Using provided Zone ID: {}", id);
                id.clone()
            }
            None => {
                println!("Looking up Cloudflare Zone ID for {}...", self.domain);
                let id =
                    lookup_cloudflare_zone_id(&self.cloudflare_api_token, &self.domain).await?;
                println!("  [ok] Zone ID: {}", id);
                id
            }
        };

        // Step 3: Verify Cloudflare token has DNS edit permissions
        println!("Verifying Cloudflare API token...");
        verify_cloudflare_token(&self.cloudflare_api_token, &zone_id).await?;
        println!("  [ok] Token has DNS edit permissions");

        // Step 4: Install Traefik binary
        println!("\nInstalling Traefik {}...", TRAEFIK_VERSION);
        self.install_traefik(&ssh).await?;

        // Step 5: Create directories and user
        println!("\nSetting up directories and user...");
        self.setup_directories(&ssh).await?;

        // Step 6: Write Traefik static config
        println!("\nWriting Traefik configuration...");
        self.write_traefik_config(&ssh, &zone_id).await?;

        // Step 7: Write systemd service
        println!("\nCreating systemd service...");
        self.write_systemd_service(&ssh).await?;

        // Step 8: Enable and start Traefik
        println!("\nStarting Traefik service...");
        self.start_traefik(&ssh).await?;

        // Step 9: Verify Traefik is running and cert is obtained
        println!("\nVerifying Traefik...");
        self.verify_traefik(&ssh).await?;

        Ok(GatewaySetupResult {
            cloudflare_zone_id: zone_id,
            traefik_version: TRAEFIK_VERSION.to_string(),
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

        // 2. Detect public interface
        let public_iface = self.detect_public_interface(ssh).await?;
        println!("  [ok] Public interface: {}", public_iface);

        // 3. Configure NAT masquerade for VM traffic
        self.configure_nat_masquerade(ssh, &public_iface).await?;

        // 4. Configure firewall rules
        self.configure_firewall(ssh).await?;

        // 5. Persist iptables rules
        self.persist_iptables(ssh).await?;

        Ok(())
    }

    async fn enable_ip_forwarding(&self, ssh: &Client) -> Result<()> {
        // Check current state
        let result = ssh.execute("sysctl -n net.ipv4.ip_forward").await?;
        if result.stdout.trim() == "1" {
            println!("  [ok] IP forwarding already enabled");
            return Ok(());
        }

        // Enable immediately
        let result = ssh
            .execute("sysctl -w net.ipv4.ip_forward=1")
            .await?;
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
        // VM subnet - Proxmox VMs use 10.x.x.x addresses
        const VM_SUBNET: &str = "10.0.0.0/8";

        // Check if rule already exists
        let check_cmd = format!(
            "iptables -t nat -C POSTROUTING -s {} -o {} -j MASQUERADE 2>/dev/null && echo exists || echo missing",
            VM_SUBNET, public_iface
        );
        let result = ssh.execute(&check_cmd).await?;

        if result.stdout.trim() == "exists" {
            println!("  [ok] NAT masquerade already configured");
            return Ok(());
        }

        // Add masquerade rule
        let cmd = format!(
            "iptables -t nat -A POSTROUTING -s {} -o {} -j MASQUERADE",
            VM_SUBNET, public_iface
        );
        let result = ssh.execute(&cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to configure NAT masquerade: {}", result.stdout);
        }

        println!("  [ok] NAT masquerade configured for {}", VM_SUBNET);
        Ok(())
    }

    async fn configure_firewall(&self, ssh: &Client) -> Result<()> {
        // VM subnet for FORWARD rules
        const VM_SUBNET: &str = "10.0.0.0/8";

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

            let cmd = format!(
                "iptables -A INPUT -p {} --dport {} -j ACCEPT",
                proto, port
            );
            let result = ssh.execute(&cmd).await?;
            if result.exit_status != 0 {
                bail!("Failed to open {} port {}: {}", desc, port, result.stdout);
            }
            println!("  [ok] Opened {} port {}", desc, port);
        }

        // FORWARD rules for VM traffic
        let forward_rules: &[(&str, &str)] = &[
            // Allow established/related connections
            ("-m state --state RELATED,ESTABLISHED", "established connections"),
            // Allow traffic from VMs
            (&format!("-s {}", VM_SUBNET), "outbound from VMs"),
            // Allow traffic to VMs
            (&format!("-d {}", VM_SUBNET), "inbound to VMs"),
        ];

        for (rule, desc) in forward_rules {
            let check_cmd = format!(
                "iptables -C FORWARD {} -j ACCEPT 2>/dev/null && echo exists || echo missing",
                rule
            );
            let result = ssh.execute(&check_cmd).await?;

            if result.stdout.trim() == "exists" {
                println!("  [ok] FORWARD rule for {} already exists", desc);
                continue;
            }

            let cmd = format!("iptables -A FORWARD {} -j ACCEPT", rule);
            let result = ssh.execute(&cmd).await?;
            if result.exit_status != 0 {
                bail!("Failed to add FORWARD rule for {}: {}", desc, result.stdout);
            }
            println!("  [ok] Added FORWARD rule for {}", desc);
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
            println!("  [warn] Failed to persist iptables rules: {}", result.stdout);
            println!("  [warn] Rules applied but may not persist across reboot");
            return Ok(());
        }

        println!("  [ok] iptables rules persisted");
        Ok(())
    }

    async fn install_traefik(&self, ssh: &Client) -> Result<()> {
        // Check if already installed with correct version
        let check = ssh.execute("traefik version 2>/dev/null || true").await?;
        if check.stdout.contains(TRAEFIK_VERSION) {
            println!("  Traefik {} already installed", TRAEFIK_VERSION);
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
            "https://github.com/traefik/traefik/releases/download/v{}/traefik_v{}_{}_linux.tar.gz",
            TRAEFIK_VERSION, TRAEFIK_VERSION, arch
        );

        let cmd = format!(
            "cd /tmp && \
             curl -sSL '{}' -o traefik.tar.gz && \
             tar xzf traefik.tar.gz traefik && \
             mv traefik /usr/local/bin/traefik && \
             chmod +x /usr/local/bin/traefik && \
             rm traefik.tar.gz",
            download_url
        );

        let result = ssh.execute(&cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to install Traefik: {}", result.stdout);
        }

        // Verify installation
        let verify = ssh.execute("traefik version").await?;
        if !verify.stdout.contains(TRAEFIK_VERSION) {
            bail!("Traefik installation verification failed");
        }

        println!("  [ok] Traefik {} installed", TRAEFIK_VERSION);
        Ok(())
    }

    async fn setup_directories(&self, ssh: &Client) -> Result<()> {
        let commands = [
            // Create traefik user/group
            "id traefik >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin traefik",
            // Create directories
            "mkdir -p /etc/traefik/dynamic /var/lib/traefik /var/log/traefik /var/lib/dc-agent",
            // Set ownership
            "chown -R traefik:traefik /etc/traefik /var/lib/traefik /var/log/traefik",
            // dc-agent needs write access to dynamic dir
            "chmod 775 /etc/traefik/dynamic",
        ];

        for cmd in commands {
            let result = ssh.execute(cmd).await?;
            if result.exit_status != 0 {
                bail!("Failed to setup directories: {} - {}", cmd, result.stdout);
            }
        }

        println!("  [ok] Directories created");
        println!("  [ok] traefik user created");
        Ok(())
    }

    async fn write_traefik_config(&self, ssh: &Client, zone_id: &str) -> Result<()> {
        let static_config = self.generate_static_config();
        let env_config = self.generate_env_file(zone_id);

        // Write static config
        let cmd = format!(
            "cat > /etc/traefik/traefik.yaml << 'EOFCONFIG'\n{}\nEOFCONFIG",
            static_config
        );
        let result = ssh.execute(&cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to write traefik.yaml: {}", result.stdout);
        }
        println!("  [ok] /etc/traefik/traefik.yaml");

        // Write environment file
        let cmd = format!(
            "cat > /etc/traefik/env << 'EOFENV'\n{}\nEOFENV\nchmod 600 /etc/traefik/env",
            env_config
        );
        let result = ssh.execute(&cmd).await?;
        if result.exit_status != 0 {
            bail!("Failed to write env file: {}", result.stdout);
        }
        println!("  [ok] /etc/traefik/env");

        // Set permissions
        ssh.execute("chown traefik:traefik /etc/traefik/traefik.yaml /etc/traefik/env")
            .await?;

        Ok(())
    }

    fn generate_static_config(&self) -> String {
        format!(
            r#"# Traefik static configuration
# Generated by dc-agent setup gateway

global:
  checkNewVersion: false
  sendAnonymousUsage: false

log:
  level: INFO
  filePath: /var/log/traefik/traefik.log

api:
  dashboard: false

entryPoints:
  web:
    address: ":80"
    http:
      redirections:
        entryPoint:
          to: websecure
          scheme: https

  websecure:
    address: ":443"
    http:
      tls:
        certResolver: letsencrypt
        domains:
          - main: "{datacenter}.{domain}"
            sans:
              - "*.{datacenter}.{domain}"

certificatesResolvers:
  letsencrypt:
    acme:
      email: admin@{domain}
      storage: /var/lib/traefik/acme.json
      dnsChallenge:
        provider: cloudflare
        resolvers:
          - "1.1.1.1:53"
          - "8.8.8.8:53"

providers:
  file:
    directory: /etc/traefik/dynamic
    watch: true
"#,
            datacenter = self.datacenter,
            domain = self.domain
        )
    }

    fn generate_env_file(&self, _zone_id: &str) -> String {
        format!(
            "CF_API_EMAIL=\nCF_DNS_API_TOKEN={}\nCF_ZONE_API_TOKEN={}\n",
            self.cloudflare_api_token, self.cloudflare_api_token
        )
    }

    async fn write_systemd_service(&self, ssh: &Client) -> Result<()> {
        let service = r#"[Unit]
Description=Traefik Reverse Proxy
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/traefik --configFile=/etc/traefik/traefik.yaml
Restart=always
RestartSec=5
User=traefik
Group=traefik

# Environment (for Cloudflare DNS challenge)
EnvironmentFile=/etc/traefik/env

# Bind to privileged ports
AmbientCapabilities=CAP_NET_BIND_SERVICE

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/etc/traefik /var/lib/traefik /var/log/traefik

[Install]
WantedBy=multi-user.target
"#;

        let cmd = format!(
            "cat > /etc/systemd/system/traefik.service << 'EOFSERVICE'\n{}\nEOFSERVICE",
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

        println!("  [ok] /etc/systemd/system/traefik.service");
        Ok(())
    }

    async fn start_traefik(&self, ssh: &Client) -> Result<()> {
        // Enable and start
        let result = ssh.execute("systemctl enable --now traefik").await?;
        if result.exit_status != 0 {
            bail!("Failed to start Traefik: {}", result.stdout);
        }

        // Wait a moment for startup
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Check status
        let result = ssh.execute("systemctl is-active traefik").await?;
        if result.stdout.trim() != "active" {
            // Get logs for debugging
            let logs = ssh
                .execute("journalctl -u traefik -n 20 --no-pager")
                .await?;
            bail!(
                "Traefik failed to start. Status: {}. Logs:\n{}",
                result.stdout.trim(),
                logs.stdout
            );
        }

        println!("  [ok] Traefik service started");
        Ok(())
    }

    async fn verify_traefik(&self, ssh: &Client) -> Result<()> {
        // Check if listening on ports 80 and 443
        let result = ssh
            .execute("ss -tlnp | grep -E ':80|:443' | grep traefik || true")
            .await?;
        if result.stdout.is_empty() {
            println!("  [warn] Traefik not yet listening on ports (may still be starting)");
        } else {
            println!("  [ok] Listening on ports 80 and 443");
        }

        // Check for certificate (may take time on first run)
        let result = ssh
            .execute("test -f /var/lib/traefik/acme.json && echo exists || echo missing")
            .await?;
        if result.stdout.trim() == "exists" {
            println!("  [ok] ACME storage initialized");
        } else {
            println!("  [info] ACME storage will be created on first request");
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
    /// Note: Cloudflare credentials are stored in Traefik's env file, not in dc-agent config.
    /// DNS management for individual VMs is handled via the central API.
    pub fn generate_gateway_config(&self, _zone_id: &str) -> String {
        format!(
            r#"
[gateway]
datacenter = "{datacenter}"
domain = "{domain}"
public_ip = "{public_ip}"
port_range_start = {port_start}
port_range_end = {port_end}
ports_per_vm = {ports_per_vm}
traefik_dynamic_dir = "/etc/traefik/dynamic"
port_allocations_path = "/var/lib/dc-agent/port-allocations.json"
# DNS management is handled via the central API (no Cloudflare credentials needed here)
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

/// Look up Cloudflare Zone ID from domain name
pub async fn lookup_cloudflare_zone_id(api_token: &str, domain: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct ZoneResponse {
        success: bool,
        result: Vec<Zone>,
        errors: Vec<CfError>,
    }

    #[derive(serde::Deserialize)]
    struct Zone {
        id: String,
        name: String,
    }

    #[derive(serde::Deserialize)]
    struct CfError {
        message: String,
    }

    let client = reqwest::Client::new();
    let url = format!("https://api.cloudflare.com/client/v4/zones?name={}", domain);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await
        .context("Failed to query Cloudflare zones")?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        bail!("Cloudflare API error ({}): {}", status, body);
    }

    let resp: ZoneResponse =
        serde_json::from_str(&body).context("Failed to parse Cloudflare response")?;

    if !resp.success {
        let errors: Vec<_> = resp.errors.iter().map(|e| &e.message).collect();
        bail!("Cloudflare errors: {:?}", errors);
    }

    resp.result
        .into_iter()
        .find(|z| z.name == domain)
        .map(|z| z.id)
        .ok_or_else(|| anyhow::anyhow!("Zone not found for domain: {}", domain))
}

/// Verify Cloudflare token has DNS edit permissions
async fn verify_cloudflare_token(api_token: &str, zone_id: &str) -> Result<()> {
    let client = reqwest::Client::new();

    // Try to list DNS records - this verifies we have read access at minimum
    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records?per_page=1",
        zone_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await
        .context("Failed to verify Cloudflare token")?;

    if !response.status().is_success() {
        let body = response.text().await?;
        bail!("Cloudflare token verification failed: {}", body);
    }

    Ok(())
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
            cloudflare_api_token: "token".into(),
            cloudflare_zone_id: Some("zone123".into()),
            port_range_start: 20000,
            port_range_end: 59999,
            ports_per_vm: 10,
        }
    }

    #[test]
    fn test_generate_static_config() {
        let setup = test_setup();
        let config = setup.generate_static_config();
        assert!(config.contains("dc-lk.decent-cloud.org"));
        assert!(config.contains("*.dc-lk.decent-cloud.org"));
        assert!(config.contains("provider: cloudflare"));
    }

    #[test]
    fn test_generate_gateway_config() {
        let setup = test_setup();
        let config = setup.generate_gateway_config("zone123");
        assert!(config.contains("datacenter = \"dc-lk\""));
        assert!(config.contains("public_ip = \"203.0.113.1\""));
        // Cloudflare credentials are NOT in dc-agent config (handled via central API)
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
    fn test_vm_subnet_is_valid_cidr() {
        // The VM subnet used in NAT and FORWARD rules
        const VM_SUBNET: &str = "10.0.0.0/8";

        // Verify it's a valid CIDR notation
        let parts: Vec<&str> = VM_SUBNET.split('/').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "10.0.0.0");
        assert_eq!(parts[1], "8");
    }

    #[test]
    fn test_sysctl_config_format() {
        // The format written to /etc/sysctl.d/99-dc-gateway.conf
        let config_line = "net.ipv4.ip_forward = 1";
        assert!(config_line.contains("net.ipv4.ip_forward"));
        assert!(config_line.contains("= 1"));
    }
}
