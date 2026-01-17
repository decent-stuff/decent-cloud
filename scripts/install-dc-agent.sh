#!/bin/bash
# DC-Agent One-Liner Installer
# Usage: curl -sSL .../install-dc-agent.sh | bash -s TOKEN [API_URL]
#   TOKEN   - Registration token from the Decent Cloud dashboard
#   API_URL - Optional API endpoint (default: https://api.decent-cloud.org)
set -euo pipefail

INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/dc-agent"
SYSTEMD_DIR="/etc/systemd/system"
GITHUB_REPO="decent-stuff/decent-cloud"

error() { echo "ERROR: $1" >&2; exit 1; }
info() { echo "==> $1"; }

TOKEN="${1:-}"
API_URL="${2:-${DC_API_URL:-https://api.decent-cloud.org}}"
[[ -z "$TOKEN" ]] && error "Usage: curl -sSL .../install-dc-agent.sh | bash -s TOKEN [API_URL]"
[[ $EUID -ne 0 ]] && error "Must run as root"
command -v curl >/dev/null || error "curl required"
command -v systemctl >/dev/null || error "systemd required"

ARCH=$(uname -m)
[[ "$ARCH" != "x86_64" ]] && error "Only x86_64 supported (got: $ARCH)"

info "Installing dc-agent"
echo "    API: ${API_URL}"
echo "    Token: ${TOKEN:0:10}..."
echo ""

info "Getting latest release..."
VERSION=$(curl -sSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
[[ -z "$VERSION" ]] && error "Failed to get latest release version"

info "Downloading dc-agent $VERSION..."
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/dc-agent-linux-amd64"
curl -sSL -o /tmp/dc-agent "$DOWNLOAD_URL" || error "Failed to download from $DOWNLOAD_URL"
chmod +x /tmp/dc-agent
/tmp/dc-agent --version || error "Binary verification failed"

info "Installing binary to ${INSTALL_DIR}/dc-agent..."
mv /tmp/dc-agent "${INSTALL_DIR}/dc-agent"
mkdir -p "$CONFIG_DIR"

info "Registering agent with ${API_URL}..."
"${INSTALL_DIR}/dc-agent" setup token \
    --token "$TOKEN" \
    --api-url "$API_URL" \
    --output "${CONFIG_DIR}/dc-agent.toml" \
    --non-interactive || error "Agent setup failed - check token validity"

info "Installing systemd service..."
cat > "${SYSTEMD_DIR}/dc-agent.service" << 'EOF'
[Unit]
Description=Decent Cloud Provisioning Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/dc-agent --config /etc/dc-agent/dc-agent.toml run
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable dc-agent
systemctl start dc-agent

info "dc-agent installed and running!"
echo ""
echo "Config: ${CONFIG_DIR}/dc-agent.toml"
echo "Keys:   /root/.dc-agent/"
echo ""
echo "Commands:"
echo "  systemctl status dc-agent     # Check status"
echo "  journalctl -fu dc-agent       # View logs"
echo "  dc-agent --config ${CONFIG_DIR}/dc-agent.toml doctor"
echo ""
echo "If using Proxmox, edit ${CONFIG_DIR}/dc-agent.toml to configure:"
echo "  - api_url, api_token_id, api_token_secret"
echo "  - node, template_vmid, storage"
echo "Then restart: systemctl restart dc-agent"
