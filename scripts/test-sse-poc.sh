#!/bin/bash
# PoC script to test SSE endpoint with query parameter authentication
# This script demonstrates how to connect to the SSE endpoint after the fix is deployed

set -e

API_URL="${API_URL:-https://dev-api.decent-cloud.org}"
PUBKEY="${PUBKEY:-}"
SIGNATURE="${SIGNATURE:-}"
TIMESTAMP="${TIMESTAMP:-}"
NONCE="${NONCE:-}"

if [ -z "$PUBKEY" ] || [ -z "$SIGNATURE" ] || [ -z "$TIMESTAMP" ] || [ -z "$NONCE" ]; then
    echo "Usage: PUBKEY=xxx SIGNATURE=xxx TIMESTAMP=xxx NONCE=xxx $0"
    echo ""
    echo "Example (with dummy values):"
    echo "  PUBKEY=deadbeef SIGNATURE=abc123 TIMESTAMP=1234567890000000000 NONCE=uuid $0"
    exit 1
fi

# Build SSE URL with query params
SSE_URL="${API_URL}/api/v1/users/${PUBKEY}/contract-events?pubkey=${PUBKEY}&signature=${SIGNATURE}&timestamp=${TIMESTAMP}&nonce=${NONCE}"

echo "Testing SSE endpoint with query params..."
echo "URL: $SSE_URL"
echo ""
echo "Expected behavior:"
echo "  - If auth is valid: SSE stream with 'event: contract-status' events"
echo "  - If auth is invalid: 401 Unauthorized error"
echo "  - If pubkey doesn't match URL: 403 Forbidden error"
echo ""

# Test with curl
echo "Connecting..."
curl -N -H "Accept: text/event-stream" "$SSE_URL" 2>&1 | head -20

echo ""
echo "Done."
