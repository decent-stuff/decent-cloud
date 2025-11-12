#!/bin/bash
set -e

# Validator entrypoint script
# Periodically validates blockchain by running 'dc provider check-in'

# Configuration from environment variables
VALIDATION_INTERVAL_SECS=${VALIDATION_INTERVAL_SECS:-600}  # Default: 10 minutes (block time)
IDENTITY_PATH=${IDENTITY_PATH:-/identity}
VALIDATION_MEMO=${VALIDATION_MEMO:-"Automated validation"}
NETWORK=${NETWORK:-ic}
LOCAL_LEDGER_DIR=${LOCAL_LEDGER_DIR:-/data/ledger}

echo "Starting Decent Cloud Validator"
echo "Network: ${NETWORK}"
echo "Identity path: ${IDENTITY_PATH}"
echo "Validation interval: ${VALIDATION_INTERVAL_SECS}s"
echo "Ledger directory: ${LOCAL_LEDGER_DIR}"

echo "Waiting 10s before first validation..."
sleep 10

# Verify identity exists (checks for private.pem and public.pem)
if [ ! -d "${IDENTITY_PATH}" ]; then
    echo "ERROR: Identity directory not found at ${IDENTITY_PATH}"
    echo "Please mount your identity directory (e.g., ~/.dcc/identity/your-identity-name) to ${IDENTITY_PATH}"
    exit 1
fi

if [ ! -f "${IDENTITY_PATH}/private.pem" ] && [ ! -f "${IDENTITY_PATH}/public.pem" ]; then
    echo "ERROR: No identity files found at ${IDENTITY_PATH}"
    echo "Expected either private.pem or public.pem (or both)"
    echo "Mounted directory contents:"
    ls -la "${IDENTITY_PATH}"
    exit 1
fi

# Create ledger directory if it doesn't exist
mkdir -p "${LOCAL_LEDGER_DIR}"

# Function to perform validation
validate() {
    echo "$(date -Iseconds) - Starting validation..."

    # Run validation with dc CLI
    if dc provider check-in \
        --identity "${IDENTITY_PATH}" \
        --network "${NETWORK}" \
        --local-ledger-dir "${LOCAL_LEDGER_DIR}" \
        --memo "${VALIDATION_MEMO}"; then

        echo "$(date -Iseconds) - Validation successful"
        # Mark container as healthy
        touch /tmp/validator-healthy
        return 0
    else
        echo "$(date -Iseconds) - Validation failed"
        return 1
    fi
}

# Initial validation on startup
echo "Running initial validation..."
if ! validate; then
    echo "WARNING: Initial validation failed, will retry on schedule"
fi

# Main validation loop
echo "Entering validation loop (interval: ${VALIDATION_INTERVAL_SECS}s)..."
while true; do
    validate || echo "Continuing despite validation failure..."
    sleep "${VALIDATION_INTERVAL_SECS}"
done
