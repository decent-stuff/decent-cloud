#!/bin/bash
# Clean up test accounts from the database before running E2E tests
# Run this from the website directory: ./tests/e2e/cleanup-test-data.sh

set -e

# Database connection
DATABASE_URL="${DATABASE_URL:-postgres://test:test@localhost:5432/test}"

echo "🧹 Cleaning up test data from database..."

# Check if PostgreSQL is running
if ! psql "$DATABASE_URL" -c "SELECT 1;" > /dev/null 2>&1; then
    echo "⚠️  Cannot connect to database at $DATABASE_URL"
    echo "   Tests will run with empty database"
    exit 0
fi

# Delete test accounts (usernames starting with 'test')
DELETED=$(psql "$DATABASE_URL" -t -c "DELETE FROM accounts WHERE username LIKE 'test%'; SELECT ROW_COUNT;" 2>/dev/null | tr -d ' ')

echo "✅ Deleted $DELETED test account(s)"
echo "   Ready to run E2E tests!"
