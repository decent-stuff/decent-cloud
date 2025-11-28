#!/bin/bash
# Test script for end-to-end account recovery flow
#
# Usage: ./scripts/test-account-recovery.sh <email>
#
# This script tests the account recovery flow by:
# 1. Requesting a recovery token
# 2. Reading the token from the database (simulating email click)
# 3. Completing recovery with a new public key

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
API_URL="${API_URL:-http://localhost:59001/api/v1}"
DATABASE_URL="${DATABASE_URL:-sqlite:./api/test.db?mode=rwc}"
EMAIL="$1"

if [ -z "$EMAIL" ]; then
    echo -e "${RED}Error: Email address required${NC}"
    echo "Usage: $0 <email>"
    exit 1
fi

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}Account Recovery Flow Test${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""
echo -e "Email: ${GREEN}$EMAIL${NC}"
echo -e "API: ${GREEN}$API_URL${NC}"
echo ""

# Step 1: Request recovery
echo -e "${YELLOW}Step 1: Requesting recovery token...${NC}"
RECOVERY_REQUEST=$(curl -s -X POST "$API_URL/accounts/recovery/request" \
  -H "Content-Type: application/json" \
  -d "{\"email\": \"$EMAIL\"}")

echo "$RECOVERY_REQUEST" | jq .

if [ "$(echo "$RECOVERY_REQUEST" | jq -r .success)" != "true" ]; then
    echo -e "${RED}Failed to request recovery${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Recovery requested${NC}"
echo ""

# Step 2: Get token from database (simulating email link click)
echo -e "${YELLOW}Step 2: Retrieving recovery token from database...${NC}"
TOKEN_HEX=$(sqlite3 "$DATABASE_URL" "SELECT hex(token) FROM recovery_tokens WHERE created_at = (SELECT MAX(created_at) FROM recovery_tokens)" 2>/dev/null || echo "")

if [ -z "$TOKEN_HEX" ]; then
    echo -e "${RED}Error: No recovery token found in database${NC}"
    echo -e "${YELLOW}Make sure:${NC}"
    echo "  1. Database is running and accessible"
    echo "  2. Account exists with email: $EMAIL"
    echo "  3. Email processor is running or email was queued"
    exit 1
fi

echo -e "Token: ${GREEN}$TOKEN_HEX${NC}"
echo ""

# Step 3: Generate a new keypair for recovery
echo -e "${YELLOW}Step 3: Generating new recovery keypair...${NC}"
NEW_KEYPAIR=$(python3 -c "
import ed25519
import binascii
signing_key = ed25519.SigningKey.generate()
verifying_key = signing_key.get_verifying_key()
print(verifying_key.to_bytes().hex())
" 2>/dev/null || echo "")

if [ -z "$NEW_KEYPAIR" ]; then
    echo -e "${YELLOW}Python ed25519 not available, using dummy key for test${NC}"
    # Generate random 32-byte hex string
    NEW_KEYPAIR=$(openssl rand -hex 32)
fi

echo -e "New public key: ${GREEN}$NEW_KEYPAIR${NC}"
echo ""

# Step 4: Complete recovery
echo -e "${YELLOW}Step 4: Completing recovery with new key...${NC}"
RECOVERY_COMPLETE=$(curl -s -X POST "$API_URL/accounts/recovery/complete" \
  -H "Content-Type: application/json" \
  -d "{\"token\": \"$TOKEN_HEX\", \"public_key\": \"$NEW_KEYPAIR\"}")

echo "$RECOVERY_COMPLETE" | jq .

if [ "$(echo "$RECOVERY_COMPLETE" | jq -r .success)" = "true" ]; then
    echo -e "${GREEN}✅ SUCCESS! Account recovery completed${NC}"
    echo ""
    echo -e "${GREEN}Summary:${NC}"
    echo "  1. Recovery requested for: $EMAIL"
    echo "  2. Token generated: $TOKEN_HEX"
    echo "  3. New key added: $NEW_KEYPAIR"
    echo ""
    echo -e "${YELLOW}Note: In production, the token would be sent via email.${NC}"
    exit 0
else
    ERROR=$(echo "$RECOVERY_COMPLETE" | jq -r .error)
    echo -e "${RED}❌ FAILED: $ERROR${NC}"
    exit 1
fi
