#!/bin/bash
# DC-Agent One-Liner Installer (supports fresh install and upgrades)
# Usage: curl -sSL .../install-dc-agent.sh | bash -s TOKEN [API_URL]
#   TOKEN   - Registration token from the Decent Cloud dashboard
#   API_URL - Optional API endpoint (default: https://api.decent-cloud.org)
#
# Run as root (no sudo needed):
#   curl -sSL .../install-dc-agent.sh | bash -s TOKEN
#
# For upgrades, just re-run the same command - it will detect and upgrade.
set -euo pipefail

INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/dc-agent"
SYSTEMD_DIR="/etc/systemd/system"
GITHUB_REPO="decent-stuff/decent-cloud"

# Retry configuration (override via env vars)
SETUP_ATTEMPTS="${DC_SETUP_ATTEMPTS:-3}"
DOCTOR_ATTEMPTS="${DC_DOCTOR_ATTEMPTS:-3}"

error() { echo "ERROR: $1" >&2; exit 1; }
info() { echo "==> $1"; }

TOKEN="${1:-}"
API_URL="${2:-${DC_API_URL:-https://api.decent-cloud.org}}"
[[ -z "$TOKEN" ]] && error "Usage: curl -sSL .../install-dc-agent.sh | bash -s TOKEN [API_URL]"
[[ $EUID -ne 0 ]] && error "Must run as root. If not logged in as root, use: su -c 'curl ... | bash -s TOKEN'"
command -v curl >/dev/null || error "curl required"
command -v systemctl >/dev/null || error "systemd required"
command -v sha256sum >/dev/null || error "sha256sum required"

ARCH=$(uname -m)
[[ "$ARCH" != "x86_64" ]] && error "Only x86_64 supported (got: $ARCH)"

# Check for existing installation
CURRENT_VERSION=""
IS_UPGRADE=false
if [[ -f "${INSTALL_DIR}/dc-agent" ]]; then
    CURRENT_VERSION=$("${INSTALL_DIR}/dc-agent" --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1) || true
    if [[ -n "$CURRENT_VERSION" ]]; then
        IS_UPGRADE=true
    fi
fi

info "Getting latest release..."
VERSION=$(curl -sSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
[[ -z "$VERSION" ]] && error "Failed to get latest release version"
EXPECTED_VERSION="${VERSION#v}"  # Strip leading 'v' if present

# Check if upgrade needed
if [[ "$IS_UPGRADE" == "true" ]]; then
    if [[ "$CURRENT_VERSION" == "$EXPECTED_VERSION" ]]; then
        info "dc-agent $CURRENT_VERSION already installed and up to date"
        # Still verify config and restart to pick up any changes
        if [[ -f "${CONFIG_DIR}/dc-agent.toml" ]]; then
            info "Verifying configuration..."
            if "${INSTALL_DIR}/dc-agent" --config "${CONFIG_DIR}/dc-agent.toml" doctor --no-test-provision; then
                info "Configuration valid, restarting service..."
                systemctl restart dc-agent
                info "dc-agent running!"
            fi
        fi
        exit 0
    else
        info "Upgrading dc-agent: $CURRENT_VERSION -> $EXPECTED_VERSION"
        systemctl stop dc-agent 2>/dev/null || true
    fi
else
    info "Installing dc-agent"
fi

echo "    API: ${API_URL}"
echo "    Token: ${TOKEN:0:10}..."
echo ""

info "Downloading dc-agent $VERSION..."
DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/dc-agent-linux-amd64"
CHECKSUMS_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/SHA256SUMS"

curl -sSL -o /tmp/dc-agent "$DOWNLOAD_URL" || error "Failed to download from $DOWNLOAD_URL"

# Download and verify checksum
info "Verifying checksum..."
curl -sSLf -o /tmp/SHA256SUMS "$CHECKSUMS_URL" || error "Failed to download SHA256SUMS from $CHECKSUMS_URL"
EXPECTED_SUM=$(grep "dc-agent-linux-amd64" /tmp/SHA256SUMS | cut -d' ' -f1)
[[ -z "$EXPECTED_SUM" ]] && error "Checksum for dc-agent-linux-amd64 not found in SHA256SUMS"
ACTUAL_SUM=$(sha256sum /tmp/dc-agent | cut -d' ' -f1)
if [[ "$EXPECTED_SUM" != "$ACTUAL_SUM" ]]; then
    rm -f /tmp/dc-agent /tmp/SHA256SUMS
    error "CHECKSUM VERIFICATION FAILED!\n  Expected: $EXPECTED_SUM\n  Got:      $ACTUAL_SUM\n\nThe binary may be corrupted or tampered with."
fi
echo "    [ok] SHA256 verified"
rm -f /tmp/SHA256SUMS

# Verify binary runs and reports correct version
chmod +x /tmp/dc-agent
BINARY_VERSION=$(/tmp/dc-agent --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1) || error "Binary verification failed"
if [[ "$BINARY_VERSION" != "$EXPECTED_VERSION" ]]; then
    rm -f /tmp/dc-agent
    error "Version mismatch! Expected $EXPECTED_VERSION but binary reports $BINARY_VERSION. Release may be corrupted."
fi

# Backup existing binary before replacing
if [[ -f "${INSTALL_DIR}/dc-agent" ]]; then
    cp "${INSTALL_DIR}/dc-agent" "${INSTALL_DIR}/dc-agent.previous"
fi

info "Installing binary to ${INSTALL_DIR}/dc-agent..."
mv /tmp/dc-agent "${INSTALL_DIR}/dc-agent"
mkdir -p "$CONFIG_DIR"

# Check if setup needs to run:
# - Fresh install (no config file)
# - Config has placeholder values that need to be replaced
NEEDS_SETUP=false
if [[ ! -f "${CONFIG_DIR}/dc-agent.toml" ]]; then
    NEEDS_SETUP=true
elif grep -q "YOUR-PROXMOX-HOST\|REPLACE-WITH-YOUR" "${CONFIG_DIR}/dc-agent.toml" 2>/dev/null; then
    info "Config has placeholder values - re-running setup"
    NEEDS_SETUP=true
fi

if [[ "$NEEDS_SETUP" == "true" ]]; then
    # Retry setup with exponential backoff
    setup_delay=5
    for attempt in $(seq 1 "$SETUP_ATTEMPTS"); do
        info "Registering agent with ${API_URL} (attempt $attempt/$SETUP_ATTEMPTS)..."
        if "${INSTALL_DIR}/dc-agent" setup token \
            --token "$TOKEN" \
            --api-url "$API_URL" \
            --output "${CONFIG_DIR}/dc-agent.toml" \
            --non-interactive; then
            break
        fi

        if [[ $attempt -eq $SETUP_ATTEMPTS ]]; then
            error "Setup failed after $SETUP_ATTEMPTS attempts. Check network connectivity and token validity."
        fi

        echo "Setup failed, retrying in ${setup_delay}s..."
        sleep $setup_delay
        setup_delay=$((setup_delay * 2))
    done
fi

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

# Retry doctor verification with exponential backoff (services may need time to start)
doctor_delay=5
doctor_success=false
for attempt in $(seq 1 "$DOCTOR_ATTEMPTS"); do
    info "Verifying configuration (attempt $attempt/$DOCTOR_ATTEMPTS)..."
    if "${INSTALL_DIR}/dc-agent" --config "${CONFIG_DIR}/dc-agent.toml" doctor --no-test-provision; then
        doctor_success=true
        break
    fi

    if [[ $attempt -eq $DOCTOR_ATTEMPTS ]]; then
        break
    fi

    echo "Verification failed, retrying in ${doctor_delay}s..."
    sleep $doctor_delay
    doctor_delay=$((doctor_delay * 2))
done

if [[ "$doctor_success" == "true" ]]; then
    echo ""
    info "Starting dc-agent service..."
    systemctl restart dc-agent

    if [[ "$IS_UPGRADE" == "true" ]]; then
        info "Upgrade complete: $CURRENT_VERSION -> $EXPECTED_VERSION"
    else
        info "dc-agent installed and running!"
    fi
    echo ""
    echo "Config: ${CONFIG_DIR}/dc-agent.toml"
    echo "Keys:   /root/.dc-agent/"
    echo ""
    echo "Commands:"
    echo "  systemctl status dc-agent     # Check status"
    echo "  journalctl -fu dc-agent       # View logs"
    echo "  dc-agent upgrade --check-only # Check for updates"
else
    echo ""
    echo "WARNING: Configuration verification failed after $DOCTOR_ATTEMPTS attempts!"
    echo "The agent service has NOT been started."
    echo ""
    # Rollback on upgrade failure
    if [[ "$IS_UPGRADE" == "true" ]] && [[ -f "${INSTALL_DIR}/dc-agent.previous" ]]; then
        echo "Rolling back to previous version..."
        mv "${INSTALL_DIR}/dc-agent.previous" "${INSTALL_DIR}/dc-agent"
        systemctl start dc-agent || true
        echo "Rolled back to $CURRENT_VERSION"
    fi
    echo ""
    echo "Review the errors above and fix the configuration:"
    echo "  ${CONFIG_DIR}/dc-agent.toml"
    echo ""
    echo "Then verify and start manually:"
    echo "  dc-agent --config ${CONFIG_DIR}/dc-agent.toml doctor"
    echo "  systemctl start dc-agent"
    exit 1
fi
