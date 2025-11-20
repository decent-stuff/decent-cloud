#!/bin/bash
# Clean up test accounts from the database before running E2E tests
# Run this from the website directory: ./tests/e2e/cleanup-test-data.sh

set -e

# Get the repo root (2 levels up from tests/e2e/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
DB_PATH="$REPO_ROOT/data/api-data-dev/ledger.db"

echo "🧹 Cleaning up test data from database..."

if [ ! -f "$DB_PATH" ]; then
    echo "⚠️  Database not found at $DB_PATH"
    echo "   Tests will run with empty database"
    exit 0
fi

# Delete test accounts (usernames starting with 't' followed by digits)
DELETED=$(sqlite3 "$DB_PATH" "DELETE FROM accounts WHERE username GLOB 't[0-9]*'; SELECT changes();")

echo "✅ Deleted $DELETED test account(s)"
echo "   Ready to run E2E tests!"
