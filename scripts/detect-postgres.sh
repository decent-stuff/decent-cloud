#!/usr/bin/env bash
# Detect PostgreSQL: compose first, then .env, then defaults
# Source this script to set PG_HOST, PG_PORT, PG_USER, PG_PASSWORD, PG_DB

# Load .env if present
if [ -f .env ]; then
    set -a; . .env; set +a
fi

COMPOSE_PG_HOST="${COMPOSE_PG_HOST:-172.18.0.2}"

# Try compose postgres first (test/test/test defaults)
if PGPASSWORD="test" psql -h "${COMPOSE_PG_HOST}" -p 5432 -U test -d test -c "SELECT 1" >/dev/null 2>&1; then
    export PG_HOST="${COMPOSE_PG_HOST}"
    export PG_PORT="5432"
    export PG_USER="test"
    export PG_PASSWORD="test"
    export PG_DB="test"
    export PG_SOURCE="compose"
else
    # Fall back to .env values or defaults
    export PG_HOST="${PG_HOST:-localhost}"
    export PG_PORT="${PG_PORT:-5432}"
    export PG_USER="${PG_USER:-test}"
    export PG_PASSWORD="${PG_PASSWORD:-test}"
    export PG_DB="${PG_DB:-test}"
    export PG_SOURCE="env"
fi
