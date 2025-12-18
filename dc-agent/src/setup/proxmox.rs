//! Proxmox VE setup wizard - automates template creation and API token setup.

use anyhow::{bail, Context, Result};
use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Supported OS templates for Proxmox VMs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsTemplate {
    Ubuntu2404,
    Ubuntu2204,
    Debian12,
    RockyLinux9,
}

impl OsTemplate {
    /// Cloud image download URL.
    pub fn image_url(&self) -> &'static str {
        match self {
            OsTemplate::Ubuntu2404 => {
                "https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img"
            }
            OsTemplate::Ubuntu2204 => {
                "https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img"
            }
            OsTemplate::Debian12 => {
                "https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.qcow2"
            }
            OsTemplate::RockyLinux9 => {
                "https://download.rockylinux.org/pub/rocky/9/images/x86_64/Rocky-9-GenericCloud.latest.x86_64.qcow2"
            }
        }
    }

    /// Filename for the downloaded image.
    pub fn image_filename(&self) -> &'static str {
        match self {
            OsTemplate::Ubuntu2404 => "noble-server-cloudimg-amd64.img",
            OsTemplate::Ubuntu2204 => "jammy-server-cloudimg-amd64.img",
            OsTemplate::Debian12 => "debian-12-generic-amd64.qcow2",
            OsTemplate::RockyLinux9 => "Rocky-9-GenericCloud.latest.x86_64.qcow2",
        }
    }

    /// Template name in Proxmox.
    pub fn template_name(&self) -> &'static str {
        match self {
            OsTemplate::Ubuntu2404 => "dc-ubuntu-2404",
            OsTemplate::Ubuntu2204 => "dc-ubuntu-2204",
            OsTemplate::Debian12 => "dc-debian-12",
            OsTemplate::RockyLinux9 => "dc-rocky-9",
        }
    }

    /// OS type for Proxmox (l26 = Linux 2.6+ kernel).
    pub fn os_type(&self) -> &'static str {
        "l26"
    }

    /// Default VMID for this template (can be overridden).
    pub fn default_vmid(&self) -> u32 {
        match self {
            OsTemplate::Ubuntu2404 => 9000,
            OsTemplate::Ubuntu2204 => 9001,
            OsTemplate::Debian12 => 9002,
            OsTemplate::RockyLinux9 => 9003,
        }
    }

    /// All available templates.
    pub fn all() -> &'static [OsTemplate] {
        &[
            OsTemplate::Ubuntu2404,
            OsTemplate::Ubuntu2204,
            OsTemplate::Debian12,
            OsTemplate::RockyLinux9,
        ]
    }

    /// Parse from string name.
    pub fn parse(s: &str) -> Option<OsTemplate> {
        match s.to_lowercase().as_str() {
            "ubuntu-24.04" | "ubuntu2404" | "noble" => Some(OsTemplate::Ubuntu2404),
            "ubuntu-22.04" | "ubuntu2204" | "jammy" => Some(OsTemplate::Ubuntu2204),
            "debian-12" | "debian12" | "bookworm" => Some(OsTemplate::Debian12),
            "rocky-9" | "rocky9" | "rockylinux9" => Some(OsTemplate::RockyLinux9),
            _ => None,
        }
    }
}

impl std::fmt::Display for OsTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsTemplate::Ubuntu2404 => write!(f, "Ubuntu 24.04 LTS"),
            OsTemplate::Ubuntu2204 => write!(f, "Ubuntu 22.04 LTS"),
            OsTemplate::Debian12 => write!(f, "Debian 12 (Bookworm)"),
            OsTemplate::RockyLinux9 => write!(f, "Rocky Linux 9"),
        }
    }
}

/// Proxmox setup configuration.
pub struct ProxmoxSetup {
    pub host: String,
    pub port: u16,
    pub ssh_user: String,
    pub ssh_password: String,
    pub proxmox_user: String,
    pub proxmox_password: String,
    pub storage: String,
    pub templates: Vec<OsTemplate>,
}

impl ProxmoxSetup {
    /// Run the complete setup process.
    pub async fn run(&self) -> Result<SetupResult> {
        println!("Connecting to Proxmox host via SSH...");
        let ssh = self.connect_ssh().await?;
        println!("  Connected to {}@{}", self.ssh_user, self.host);

        // Get node name
        let node = self.get_node_name(&ssh).await?;
        println!("  Proxmox node: {}", node);

        // Check available storage
        self.verify_storage(&ssh).await?;
        println!("  Storage '{}' verified", self.storage);

        // Create templates
        let mut template_vmids = HashMap::new();
        for template in &self.templates {
            println!("\nSetting up {} template...", template);
            let vmid = self.create_template(&ssh, &node, *template).await?;
            template_vmids.insert(*template, vmid);
            println!("  Template created with VMID {}", vmid);
        }

        // Create API token via Proxmox API
        println!("\nCreating API token...");
        let (token_id, token_secret) = self.create_api_token().await?;
        println!("  Token created: {}", token_id);

        // Verify token works
        println!("\nVerifying API token...");
        self.verify_api_token(&token_id, &token_secret).await?;
        println!("  Token verified successfully");

        Ok(SetupResult {
            api_url: format!("https://{}:8006", self.host),
            api_token_id: token_id,
            api_token_secret: token_secret,
            node,
            storage: self.storage.clone(),
            template_vmids,
        })
    }

    async fn connect_ssh(&self) -> Result<Client> {
        let auth = AuthMethod::with_password(&self.ssh_password);
        let client = Client::connect(
            (self.host.as_str(), self.port),
            &self.ssh_user,
            auth,
            ServerCheckMethod::NoCheck,
        )
        .await
        .context("Failed to connect via SSH")?;
        Ok(client)
    }

    async fn get_node_name(&self, ssh: &Client) -> Result<String> {
        let result = ssh
            .execute("hostname")
            .await
            .context("Failed to get hostname")?;
        if result.exit_status != 0 {
            bail!("Failed to get hostname: exit status {}", result.exit_status);
        }
        Ok(result.stdout.trim().to_string())
    }

    async fn verify_storage(&self, ssh: &Client) -> Result<()> {
        let cmd = format!("pvesm status -storage {}", self.storage);
        let result = ssh.execute(&cmd).await.context("Failed to check storage")?;
        if result.exit_status != 0 {
            bail!(
                "Storage '{}' not found or not available. Check 'pvesm status' output.",
                self.storage
            );
        }
        Ok(())
    }

    async fn create_template(
        &self,
        ssh: &Client,
        _node: &str,
        template: OsTemplate,
    ) -> Result<u32> {
        let vmid = template.default_vmid();
        let name = template.template_name();
        let image_url = template.image_url();
        let image_file = template.image_filename();
        let tmp_path = format!("/tmp/{}", image_file);

        // Check if template already exists
        let check_cmd = format!("qm status {}", vmid);
        let check_result = ssh.execute(&check_cmd).await?;
        if check_result.exit_status == 0 {
            println!("  Template VMID {} already exists, skipping creation", vmid);
            return Ok(vmid);
        }

        // Download cloud image if not present
        let check_file = format!("test -f {}", tmp_path);
        let file_exists = ssh.execute(&check_file).await?;
        if file_exists.exit_status != 0 {
            println!("  Downloading cloud image (this may take a few minutes)...");
            let download_cmd = format!("wget -q -O {} {}", tmp_path, image_url);
            let download_result = ssh
                .execute(&download_cmd)
                .await
                .context("Failed to download cloud image")?;
            if download_result.exit_status != 0 {
                bail!("Failed to download image: {}", download_result.stdout);
            }
        } else {
            println!("  Cloud image already downloaded, reusing");
        }

        // Create VM
        println!("  Creating VM...");
        let create_cmd = format!(
            "qm create {} --name {} --ostype {} --memory 1024 --cores 1 \
             --net0 virtio,bridge=vmbr0 --agent enabled=1 \
             --serial0 socket --vga serial0",
            vmid,
            name,
            template.os_type()
        );
        let create_result = ssh.execute(&create_cmd).await?;
        if create_result.exit_status != 0 {
            bail!("Failed to create VM: {}", create_result.stdout);
        }

        // Import disk
        println!("  Importing disk...");
        let import_cmd = format!(
            "qm importdisk {} {} {} --format qcow2",
            vmid, tmp_path, self.storage
        );
        let import_result = ssh.execute(&import_cmd).await?;
        if import_result.exit_status != 0 {
            // Cleanup VM on failure
            let _ = ssh.execute(&format!("qm destroy {}", vmid)).await;
            bail!("Failed to import disk: {}", import_result.stdout);
        }

        // Attach disk and configure boot
        println!("  Configuring VM...");
        let disk_name = format!("vm-{}-disk-0", vmid);
        let config_cmds = [
            format!(
                "qm set {} --scsihw virtio-scsi-pci --scsi0 {}:{},discard=on",
                vmid, self.storage, disk_name
            ),
            format!("qm set {} --boot order=scsi0", vmid),
            format!("qm set {} --ide2 {}:cloudinit", vmid, self.storage),
        ];

        for cmd in &config_cmds {
            let result = ssh.execute(cmd).await?;
            if result.exit_status != 0 {
                let _ = ssh.execute(&format!("qm destroy {}", vmid)).await;
                bail!("Failed to configure VM: {}", result.stdout);
            }
        }

        // Clear machine-id (important for proper cloning)
        println!("  Clearing machine-id for clean clones...");
        // This requires virt-customize from libguestfs-tools
        let clear_machineid = format!(
            "command -v virt-customize >/dev/null 2>&1 && \
             virt-customize -a /var/lib/vz/images/{}/{} --run-command 'truncate -s 0 /etc/machine-id' 2>/dev/null || true",
            vmid, disk_name
        );
        let _ = ssh.execute(&clear_machineid).await; // Best effort, don't fail if not available

        // Convert to template
        println!("  Converting to template...");
        let template_cmd = format!("qm template {}", vmid);
        let template_result = ssh.execute(&template_cmd).await?;
        if template_result.exit_status != 0 {
            let _ = ssh.execute(&format!("qm destroy {}", vmid)).await;
            bail!("Failed to convert to template: {}", template_result.stdout);
        }

        Ok(vmid)
    }

    async fn create_api_token(&self) -> Result<(String, String)> {
        // First, get authentication ticket
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let auth_url = format!("https://{}:8006/api2/json/access/ticket", self.host);
        let auth_params = [
            ("username", self.proxmox_user.as_str()),
            ("password", self.proxmox_password.as_str()),
        ];

        let auth_response = client
            .post(&auth_url)
            .form(&auth_params)
            .send()
            .await
            .context("Failed to authenticate with Proxmox API")?;

        if !auth_response.status().is_success() {
            bail!("Proxmox authentication failed: {}", auth_response.status());
        }

        let auth_data: AuthResponse = auth_response
            .json()
            .await
            .context("Failed to parse auth response")?;

        let ticket = &auth_data.data.ticket;
        let csrf_token = &auth_data.data.csrf_prevention_token;

        // Create API token
        let token_name = "dc-agent";
        let user_part = self.proxmox_user.split('@').next().unwrap_or("root");
        let realm = self.proxmox_user.split('@').nth(1).unwrap_or("pam");
        let token_id = format!("{}@{}!{}", user_part, realm, token_name);

        let token_url = format!(
            "https://{}:8006/api2/json/access/users/{}/token/{}",
            self.host,
            urlencoding::encode(&format!("{}@{}", user_part, realm)),
            token_name
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("PVEAuthCookie={}", ticket))?,
        );
        headers.insert("CSRFPreventionToken", HeaderValue::from_str(csrf_token)?);

        // Check if token already exists, delete it first
        let check_response = client
            .get(&token_url)
            .headers(headers.clone())
            .send()
            .await?;

        if check_response.status().is_success() {
            // Token exists, delete it
            client
                .delete(&token_url)
                .headers(headers.clone())
                .send()
                .await
                .context("Failed to delete existing token")?;
        }

        // Create new token
        let token_response = client
            .post(&token_url)
            .headers(headers)
            .send()
            .await
            .context("Failed to create API token")?;

        if !token_response.status().is_success() {
            let status = token_response.status();
            let body = token_response.text().await.unwrap_or_default();
            bail!("Failed to create API token: {} - {}", status, body);
        }

        let token_data: TokenResponse = token_response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok((token_id, token_data.data.value))
    }

    async fn verify_api_token(&self, token_id: &str, token_secret: &str) -> Result<()> {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let url = format!("https://{}:8006/api2/json/version", self.host);
        let auth_header = format!("PVEAPIToken={}={}", token_id, token_secret);

        let response = client
            .get(&url)
            .header(AUTHORIZATION, auth_header)
            .send()
            .await
            .context("Failed to verify API token")?;

        if !response.status().is_success() {
            bail!("API token verification failed: {}", response.status());
        }

        Ok(())
    }
}

/// Result of successful setup.
pub struct SetupResult {
    pub api_url: String,
    pub api_token_id: String,
    pub api_token_secret: String,
    pub node: String,
    pub storage: String,
    pub template_vmids: HashMap<OsTemplate, u32>,
}

impl SetupResult {
    /// Generate dc-agent.toml configuration file content.
    pub fn generate_config(&self, api_endpoint: &str, provider_pubkey: &str) -> String {
        let primary_vmid = self
            .template_vmids
            .get(&OsTemplate::Ubuntu2404)
            .or_else(|| self.template_vmids.values().next())
            .copied()
            .unwrap_or(9000);

        format!(
            r#"# Decent Cloud Agent Configuration
# Generated by dc-agent setup

[api]
endpoint = "{api_endpoint}"
provider_pubkey = "{provider_pubkey}"
provider_secret_key = "YOUR_SECRET_KEY_HERE"  # Replace with your Ed25519 secret key

[polling]
interval_seconds = 30
health_check_interval_seconds = 300

[provisioner]
type = "proxmox"

[provisioner.proxmox]
api_url = "{api_url}"
api_token_id = "{token_id}"
api_token_secret = "{token_secret}"
node = "{node}"
template_vmid = {vmid}
storage = "{storage}"
verify_ssl = false

# Available template VMIDs:
{template_comments}
"#,
            api_endpoint = api_endpoint,
            provider_pubkey = provider_pubkey,
            api_url = self.api_url,
            token_id = self.api_token_id,
            token_secret = self.api_token_secret,
            node = self.node,
            vmid = primary_vmid,
            storage = self.storage,
            template_comments = self
                .template_vmids
                .iter()
                .map(|(t, v)| format!("# {} = {}", t.template_name(), v))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Write configuration to file.
    pub fn write_config(
        &self,
        path: &Path,
        api_endpoint: &str,
        provider_pubkey: &str,
    ) -> Result<()> {
        let content = self.generate_config(api_endpoint, provider_pubkey);
        std::fs::write(path, content).context("Failed to write config file")?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct AuthResponse {
    data: AuthData,
}

#[derive(Deserialize)]
struct AuthData {
    ticket: String,
    #[serde(rename = "CSRFPreventionToken")]
    csrf_prevention_token: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    data: TokenData,
}

#[derive(Deserialize)]
struct TokenData {
    value: String,
}

// Hash implementation for OsTemplate to use in HashMap
impl std::hash::Hash for OsTemplate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_template_urls() {
        for template in OsTemplate::all() {
            assert!(!template.image_url().is_empty());
            assert!(template.image_url().starts_with("https://"));
        }
    }

    #[test]
    fn test_os_template_parse() {
        assert_eq!(
            OsTemplate::parse("ubuntu-24.04"),
            Some(OsTemplate::Ubuntu2404)
        );
        assert_eq!(OsTemplate::parse("noble"), Some(OsTemplate::Ubuntu2404));
        assert_eq!(OsTemplate::parse("debian-12"), Some(OsTemplate::Debian12));
        assert_eq!(OsTemplate::parse("rocky-9"), Some(OsTemplate::RockyLinux9));
        assert_eq!(OsTemplate::parse("invalid"), None);
    }

    #[test]
    fn test_os_template_vmids_unique() {
        let vmids: Vec<u32> = OsTemplate::all().iter().map(|t| t.default_vmid()).collect();
        let unique: std::collections::HashSet<u32> = vmids.iter().copied().collect();
        assert_eq!(vmids.len(), unique.len(), "Template VMIDs must be unique");
    }

    #[test]
    fn test_generate_config() {
        let mut template_vmids = HashMap::new();
        template_vmids.insert(OsTemplate::Ubuntu2404, 9000);

        let result = SetupResult {
            api_url: "https://192.168.1.100:8006".to_string(),
            api_token_id: "root@pam!dc-agent".to_string(),
            api_token_secret: "secret-uuid".to_string(),
            node: "pve".to_string(),
            storage: "local-lvm".to_string(),
            template_vmids,
        };

        let config = result.generate_config("https://api.example.com", "pubkey123");
        assert!(config.contains("api_url = \"https://192.168.1.100:8006\""));
        assert!(config.contains("api_token_id = \"root@pam!dc-agent\""));
        assert!(config.contains("template_vmid = 9000"));
    }
}
