#!/usr/bin/env bash
# Local dev environment: SvelteKit website (port 59010) + optional local API (port 59011)
#
# The website dev server is always started locally (no build needed, just npm run dev).
# The API defaults to the remote dev server (https://dev-api.decent-cloud.org).
# A local API can be started if target/debug/api-server already exists.
#
# Usage:
#   scripts/dev-server.sh [start]      — start website (+ local API if binary exists)
#   scripts/dev-server.sh stop         — stop everything
#   scripts/dev-server.sh status       — show running status
#   scripts/dev-server.sh logs [web]   — tail logs (default: api)
#
# After start, test from within the container (no external browser needed):
#   BROWSER_LOCAL=1 node scripts/browser.js snap  http://localhost:59010
#   BROWSER_LOCAL=1 node scripts/browser.js shot  http://localhost:59010/dashboard /tmp/shot.png
#   BROWSER_LOCAL=1 node scripts/browser.js errs  http://localhost:59010/dashboard

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PIDS="$ROOT/.dev-pids"
API_PORT=59011
WEB_PORT=59010
REMOTE_API_URL="https://dev-api.decent-cloud.org"
API_BINARY="$ROOT/target/debug/api-server"

# Source all env vars from cf/.env.dev
# shellcheck disable=SC1090
set -a
# shellcheck source=/dev/null
source "$ROOT/cf/.env.dev"
set +a

_wait_for() {
  local name="$1" url="$2"
  echo -n "Waiting for $name"
  for _ in $(seq 1 40); do
    if curl -sf "$url" >/dev/null 2>&1; then
      echo " ready"
      return 0
    fi
    echo -n "."
    sleep 2
  done
  echo " TIMEOUT"
  return 1
}

case "${1:-start}" in
  start)
    mkdir -p "$PIDS"

    # ── API server (optional — only if binary already built) ────────────────
    if [ -x "$API_BINARY" ]; then
      if [ -f "$PIDS/api.pid" ] && kill -0 "$(cat "$PIDS/api.pid")" 2>/dev/null; then
        echo "Local API already running (pid $(cat "$PIDS/api.pid"))"
      else
        echo "Starting local API server on :$API_PORT..."
        DATABASE_URL="$API_DATABASE_URL" \
        API_SERVER_PORT="$API_PORT" \
        FRONTEND_URL="http://localhost:$WEB_PORT" \
        SQLX_OFFLINE=true \
          "$API_BINARY" serve \
          >> "$PIDS/api.log" 2>&1 &
        echo $! > "$PIDS/api.pid"
        _wait_for "local API" "http://localhost:$API_PORT/api/v1/health"
      fi
      API_URL="http://localhost:$API_PORT"
    else
      echo "No local API binary — using remote dev API: $REMOTE_API_URL"
      API_URL="$REMOTE_API_URL"
    fi

    # ── Website ─────────────────────────────────────────────────────────────
    if [ -f "$PIDS/web.pid" ] && kill -0 "$(cat "$PIDS/web.pid")" 2>/dev/null; then
      echo "Website already running (pid $(cat "$PIDS/web.pid"))"
    else
      echo "Starting website on :$WEB_PORT (API: $API_URL)..."
      cd "$ROOT/website"
      VITE_DECENT_CLOUD_API_URL="$API_URL" \
        npm run dev -- --port "$WEB_PORT" --host 127.0.0.1 \
        >> "$PIDS/web.log" 2>&1 &
      echo $! > "$PIDS/web.pid"
      cd "$ROOT"
      _wait_for "website" "http://localhost:$WEB_PORT"
    fi

    echo ""
    echo "Dev environment ready:"
    echo "  Website: http://localhost:$WEB_PORT  (API: $API_URL)"
    echo ""
    echo "Browser (local headless Chromium, no external deps):"
    echo "  BROWSER_LOCAL=1 node scripts/browser.js snap  http://localhost:$WEB_PORT"
    echo "  BROWSER_LOCAL=1 node scripts/browser.js shot  http://localhost:$WEB_PORT/dashboard /tmp/shot.png"
    echo "  BROWSER_LOCAL=1 node scripts/browser.js errs  http://localhost:$WEB_PORT/dashboard"
    ;;

  stop)
    for svc in api web; do
      if [ -f "$PIDS/$svc.pid" ]; then
        pid=$(cat "$PIDS/$svc.pid")
        if kill "$pid" 2>/dev/null; then
          echo "Stopped $svc (pid $pid)"
        else
          echo "$svc was not running"
        fi
        rm -f "$PIDS/$svc.pid"
      else
        echo "$svc: not started"
      fi
    done
    ;;

  status)
    for svc in api web; do
      if [ -f "$PIDS/$svc.pid" ] && kill -0 "$(cat "$PIDS/$svc.pid")" 2>/dev/null; then
        echo "$svc: running (pid $(cat "$PIDS/$svc.pid"))"
      else
        echo "$svc: stopped"
      fi
    done
    ;;

  logs)
    tail -f "$PIDS/${2:-api}.log"
    ;;

  *)
    echo "Usage: $0 start|stop|status|logs [api|web]" >&2
    exit 1
    ;;
esac
