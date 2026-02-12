#!/usr/bin/env bash
# Detect PostgreSQL: compose first, then .env, then defaults
# Source this script to set PG_HOST, PG_PORT, PG_USER, PG_PASSWORD, PG_DB

# Load .env if present
if [ -f .env ]; then
    set -a; . .env; set +a
fi

COMPOSE_PG_HOST="${COMPOSE_PG_HOST:-172.18.0.2}"

# Try well-known postgres hosts with test/test/test credentials
_pg_detected=false
for _pg_host in "${COMPOSE_PG_HOST}" "postgres"; do
    if PGPASSWORD="test" psql -h "${_pg_host}" -p 5432 -U test -d test -c "SELECT 1" >/dev/null 2>&1; then
        export PG_HOST="${_pg_host}"
        export PG_PORT="5432"
        export PG_USER="test"
        export PG_PASSWORD="test"
        export PG_DB="test"
        export PG_SOURCE="compose"
        _pg_detected=true
        break
    fi
done

if [ "$_pg_detected" = false ]; then
    # Fall back to .env values or defaults
    export PG_HOST="${PG_HOST:-localhost}"
    export PG_PORT="${PG_PORT:-5432}"
    export PG_USER="${PG_USER:-test}"
    export PG_PASSWORD="${PG_PASSWORD:-test}"
    export PG_DB="${PG_DB:-test}"
    export PG_SOURCE="env"
fi
