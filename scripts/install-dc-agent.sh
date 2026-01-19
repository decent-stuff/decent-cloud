#!/bin/bash
# DC-Agent One-Liner Installer
# Usage: curl -sSL .../install-dc-agent.sh | bash -s TOKEN [API_URL]
#   TOKEN   - Registration token from the Decent Cloud dashboard
#   API_URL - Optional API endpoint (default: https://api.decent-cloud.org)
#
# Run as root (no sudo needed):
#   curl -sSL .../install-dc-agent.sh | bash -s TOKEN
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
[[ $EUID -ne 0 ]] && error "Must run as root. If not logged in as root, use: su -c 'curl ... | bash -s TOKEN'"
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
BINARY_VERSION=$(/tmp/dc-agent --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1) || error "Binary verification failed"
EXPECTED_VERSION="${VERSION#v}"  # Strip leading 'v' if present
if [[ "$BINARY_VERSION" != "$EXPECTED_VERSION" ]]; then
    error "Version mismatch! Expected $EXPECTED_VERSION but binary reports $BINARY_VERSION. Release may be corrupted."
fi

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

info "Verifying configuration..."
if "${INSTALL_DIR}/dc-agent" --config "${CONFIG_DIR}/dc-agent.toml" doctor --no-test-provision; then
    echo ""
    info "Starting dc-agent service..."
    systemctl restart dc-agent

    info "dc-agent installed and running!"
    echo ""
    echo "Config: ${CONFIG_DIR}/dc-agent.toml"
    echo "Keys:   /root/.dc-agent/"
    echo ""
    echo "Commands:"
    echo "  systemctl status dc-agent     # Check status"
    echo "  journalctl -fu dc-agent       # View logs"
else
    echo ""
    echo "WARNING: Configuration verification failed!"
    echo "The agent service has NOT been started."
    echo ""
    echo "Review the errors above and fix the configuration:"
    echo "  ${CONFIG_DIR}/dc-agent.toml"
    echo ""
    echo "Then verify and start manually:"
    echo "  dc-agent --config ${CONFIG_DIR}/dc-agent.toml doctor"
    echo "  systemctl start dc-agent"
    exit 1
fi
