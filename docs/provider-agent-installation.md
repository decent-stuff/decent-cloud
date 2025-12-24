# DC-Agent Installation Guide

This guide covers installing the dc-agent provisioning agent on your infrastructure.

## Quick Start

### 1. Get a Setup Token

1. Log in to the Decent Cloud provider dashboard
2. Go to **Agents** > **[Your Pool]** > **Add Agent**
3. Copy the setup token

### 2. Run the Installer

On your Proxmox host (or any Linux x86_64 server):

```bash
curl -sSL https://raw.githubusercontent.com/decent-stuff/decent-cloud/main/scripts/install-dc-agent.sh | sudo bash -s YOUR_TOKEN
```

This will:
- Download the latest dc-agent binary
- Register the agent with your pool
- Install and start a systemd service

### 3. Configure Your Provisioner

Edit `/etc/dc-agent/dc-agent.toml` to configure your provisioner.

#### Proxmox VE

```toml
[provisioner.proxmox]
api_url = "https://your-proxmox-host:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "your-api-token-secret"
node = "pve1"
template_vmid = 9000
storage = "local-lvm"
# verify_ssl = true  # Set to false for self-signed certs
```

To create a Proxmox API token:
1. Go to Datacenter > Permissions > API Tokens
2. Create token for `root@pam` named `dc-agent`
3. Copy the token secret (shown only once)

#### Script-Based

```toml
[provisioner.script]
provision = "/opt/dc-agent/provision.sh"
terminate = "/opt/dc-agent/terminate.sh"
health_check = "/opt/dc-agent/health.sh"
timeout_seconds = 300
```

#### Manual

```toml
[provisioner.manual]
notification_webhook = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
```

### 4. Restart the Agent

```bash
systemctl restart dc-agent
```

## Monitoring

### Check Status

```bash
systemctl status dc-agent
```

### View Logs

```bash
# Follow logs in real-time
journalctl -fu dc-agent

# View last 100 lines
journalctl -u dc-agent -n 100
```

### Verify Configuration

```bash
dc-agent --config /etc/dc-agent/dc-agent.toml doctor
```

## File Locations

| Path | Description |
|------|-------------|
| `/usr/local/bin/dc-agent` | Agent binary |
| `/etc/dc-agent/dc-agent.toml` | Configuration file |
| `/root/.dc-agent/` | Agent keypair |
| `/etc/systemd/system/dc-agent.service` | Systemd service |

## Updating

To update to the latest version:

```bash
systemctl stop dc-agent
curl -sSL -o /usr/local/bin/dc-agent \
  https://github.com/decent-stuff/decent-cloud/releases/latest/download/dc-agent-linux-amd64
chmod +x /usr/local/bin/dc-agent
systemctl start dc-agent
```

## Uninstalling

```bash
systemctl stop dc-agent
systemctl disable dc-agent
rm /etc/systemd/system/dc-agent.service
rm /usr/local/bin/dc-agent
rm -rf /etc/dc-agent /root/.dc-agent
systemctl daemon-reload
```

## Troubleshooting

### Agent not connecting to API

1. Check network connectivity:
   ```bash
   curl -s https://api.decent-cloud.org/health
   ```

2. Verify the setup token was valid (check logs for registration errors):
   ```bash
   journalctl -u dc-agent | grep -i "error\|failed"
   ```

### Proxmox provisioning fails

1. Run the doctor command:
   ```bash
   dc-agent --config /etc/dc-agent/dc-agent.toml doctor
   ```

2. Verify API token has sufficient permissions (PVEAdmin role recommended)

3. Check template VM exists:
   ```bash
   pvesh get /nodes/NODE/qemu/VMID/status/current
   ```

### Permission denied errors

Ensure dc-agent runs as root (required for Proxmox API access):
```bash
ps aux | grep dc-agent
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DC_API_URL` | Override API endpoint | `https://api.decent-cloud.org` |
| `RUST_LOG` | Log level (error, warn, info, debug, trace) | `info` |
