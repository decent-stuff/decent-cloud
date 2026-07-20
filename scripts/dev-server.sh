#!/usr/bin/env bash
# Local dev environment: SvelteKit website (port 59010) + local API (port 59011).
#
# One source of truth for stack lifecycle. Detached (setsid) so processes
# survive the caller's exit; idempotent so re-running start is a no-op if
# the stack is already healthy.
#
# Usage:
#   scripts/dev-server.sh start [--e2e]   — start stack (idempotent)
#   scripts/dev-server.sh stop            — stop stack (process-group kill)
#   scripts/dev-server.sh status          — show running status with port health
#   scripts/dev-server.sh restart [--e2e] — stop + start
#   scripts/dev-server.sh logs [api|web]  — tail merged stdout log
#
# Modes:
#   default   — website always local; API uses remote dev if no local binary.
#   --e2e     — forces LOCAL api (no remote fallback), builds binary if missing,
#               disables rate limiting so parallel test workers don't 429.
#
# Test entrypoints (see website/package.json):
#   npm run e2e:up           = scripts/dev-server.sh start --e2e
#   npm run test:e2e:fast    = playwright against pre-started stack (no spawn)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PIDS="$ROOT/.dev-pids"
API_PORT="${API_PORT:-59011}"
WEB_PORT="${WEB_PORT:-59010}"
REMOTE_API_URL="https://dev-api.decent-cloud.org"
API_BINARY="$ROOT/target/debug/api-server"
DEFAULT_CANISTER_ID="ggi4a-wyaaa-aaaai-actqq-cai"

# Source all env vars from cf/.env.dev (optional in --e2e mode if defaults work).
if [ ! -f "$ROOT/cf/.env.dev" ]; then
  echo "error: $ROOT/cf/.env.dev not found." >&2
  echo "       Copy cf/.env.dev.example to cf/.env.dev and edit it:" >&2
  echo "         cp cf/.env.dev.example cf/.env.dev" >&2
  exit 1
fi
# shellcheck disable=SC1090
set -a
# shellcheck source=/dev/null
source "$ROOT/cf/.env.dev"
set +a

E2E_MODE=0
for arg in "$@"; do
  case "$arg" in
    --e2e) E2E_MODE=1 ;;
  esac
done

# Resolve effective env for the API server.
# API_DATABASE_URL (from cf/.env.dev) wins; DATABASE_URL is the fallback.
effective_db_url() {
  printf '%s' "${API_DATABASE_URL:-${DATABASE_URL:-postgres://test:test@postgres:5432/test}}"
}

_wait_for() {
  local name="$1" url="$2" deadline="$3" now deadline_s
  now=$(date +%s)
  deadline_s=$((now + deadline))
  echo -n "Waiting for $name"
  while [ "$(date +%s)" -lt "$deadline_s" ]; do
    if curl -sf "$url" >/dev/null 2>&1; then
      echo " ready"
      return 0
    fi
    echo -n "."
    sleep 1
  done
  echo " TIMEOUT (${deadline}s)"
  return 1
}

# Is a service healthy? pid alive AND port responds.
_healthy() {
  local svc="$1" url="$2" pid
  [ -f "$PIDS/$svc.pid" ] || return 1
  pid=$(cat "$PIDS/$svc.pid" 2>/dev/null || true)
  [ -n "$pid" ] || return 1
  kill -0 "$pid" 2>/dev/null || return 1
  curl -sf "$url" >/dev/null 2>&1
}

# Launch a detached service. Captures the session-leader PID (== exec'd PID)
# so we can later kill the whole process group via kill -TERM -<pid>.
# Args: name cmd-working-dir cmd-and-args...
_start_service() {
  local name="$1" wd="$2"; shift 2
  local stdout="$PIDS/$name.log" stderr="$PIDS/$name-stderr.log" pidfile="$PIDS/$name.pid"
  : > "$stdout"
  : > "$stderr"
  # printf -v safely quotes each arg into the inner script so the inner bash
  # sees the literal cmd+env-array (its own $@ would be empty under `bash -c`).
  # setsid makes the service a session leader so kill -TERM -<pid> takes down
  # the whole group (vite, child workers, etc). stderr MUST be a separate file
  # — merged 2>&1 lets the outer shell's process-tree killer follow the FD and
  # reap the group.
  local q_wd q_pid q_out q_err cmd_str arg
  printf -v q_wd '%q' "$wd"
  printf -v q_pid '%q' "$pidfile"
  printf -v q_out '%q' "$stdout"
  printf -v q_err '%q' "$stderr"
  cmd_str=""
  for arg in "$@"; do
    local q_arg
    printf -v q_arg '%q' "$arg"
    cmd_str+=" $q_arg"
  done
  setsid --fork bash -c "
    cd $q_wd
    echo \$\$ > $q_pid
    exec env$cmd_str >>$q_out 2>>$q_err
  " </dev/null >/dev/null 2>&1
}

start_stack() {
  mkdir -p "$PIDS"
  local start_time end_time elapsed
  start_time=$(date +%s)

  # ── API server ───────────────────────────────────────────────────────────
  local api_url
  if [ "$E2E_MODE" -eq 1 ]; then
    if [ ! -x "$API_BINARY" ]; then
      echo "E2E mode: $API_BINARY missing — building (cargo build -p api --bin api-server)..."
      (cd "$ROOT" && cargo build -p api --bin api-server)
    fi
    if _healthy api "http://localhost:$API_PORT/api/v1/health"; then
      echo "API already running (pid $(cat "$PIDS/api.pid"))"
    else
      echo "Starting local API on :$API_PORT (e2e profile, rate-limit disabled)..."
      _start_service api "$ROOT" \
        "DATABASE_URL=$(effective_db_url)" \
        "API_SERVER_PORT=$API_PORT" \
        "FRONTEND_URL=http://localhost:$WEB_PORT" \
        "SQLX_OFFLINE=true" \
        "CANISTER_ID=${CANISTER_ID:-$DEFAULT_CANISTER_ID}" \
        "RATE_LIMIT_ENABLED=false" \
        "$API_BINARY" serve
      _wait_for "local API" "http://localhost:$API_PORT/api/v1/health" 60 || return 1
    fi
    api_url="http://localhost:$API_PORT"
  elif [ -x "$API_BINARY" ]; then
    if _healthy api "http://localhost:$API_PORT/api/v1/health"; then
      echo "API already running (pid $(cat "$PIDS/api.pid"))"
    else
      echo "Starting local API on :$API_PORT..."
      _start_service api "$ROOT" \
        "DATABASE_URL=$(effective_db_url)" \
        "API_SERVER_PORT=$API_PORT" \
        "FRONTEND_URL=http://localhost:$WEB_PORT" \
        "SQLX_OFFLINE=true" \
        "CANISTER_ID=${CANISTER_ID:-$DEFAULT_CANISTER_ID}" \
        "$API_BINARY" serve
      _wait_for "local API" "http://localhost:$API_PORT/api/v1/health" 60 || return 1
    fi
    api_url="http://localhost:$API_PORT"
  else
    echo "No local API binary — using remote dev API: $REMOTE_API_URL"
    echo "  (build with: cargo build -p api --bin api-server)"
    api_url="$REMOTE_API_URL"
  fi

  # ── Website ──────────────────────────────────────────────────────────────
  if _healthy web "http://localhost:$WEB_PORT"; then
    echo "Website already running (pid $(cat "$PIDS/web.pid"))"
  else
    echo "Starting website on :$WEB_PORT (API: $api_url)..."
    _start_service web "$ROOT/website" \
      "VITE_DECENT_CLOUD_API_URL=$api_url" \
      "VITE_CHATWOOT_WEBSITE_TOKEN=" \
      "VITE_CHATWOOT_BASE_URL=" \
      npm run dev -- --host 127.0.0.1 --port "$WEB_PORT" --strictPort
    _wait_for "website" "http://localhost:$WEB_PORT" 60 || return 1
  fi

  end_time=$(date +%s)
  elapsed=$((end_time - start_time))
  echo ""
  echo "Dev stack ready in ${elapsed}s:"
  echo "  Website: http://localhost:$WEB_PORT  (API: $api_url)"
  if [ "$E2E_MODE" -eq 1 ]; then
    echo ""
    echo "Run tests against this stack:"
    echo "  cd website && npm run test:e2e:fast"
  fi
}

stop_stack() {
  local svc pid
  for svc in api web; do
    if [ -f "$PIDS/$svc.pid" ]; then
      pid=$(cat "$PIDS/$svc.pid" 2>/dev/null || true)
      if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        # Negative PID = process group. setsid made the service a group leader
        # with pgid == pid, so this kills the service + any children (vite, etc).
        kill -TERM "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
        echo "Stopped $svc (pid $pid, group)"
      else
        echo "$svc: not running (stale pid file removed)"
      fi
      rm -f "$PIDS/$svc.pid"
    else
      echo "$svc: not started"
    fi
  done
}

status_stack() {
  local svc pid url
  for svc in api web; do
    if [ "$svc" = "api" ]; then
      url="http://localhost:$API_PORT/api/v1/health"
    else
      url="http://localhost:$WEB_PORT"
    fi
    if [ -f "$PIDS/$svc.pid" ] && kill -0 "$(cat "$PIDS/$svc.pid" 2>/dev/null)" 2>/dev/null; then
      pid=$(cat "$PIDS/$svc.pid")
      if curl -sf "$url" >/dev/null 2>&1; then
        echo "$svc: healthy (pid $pid, $url -> 200)"
      else
        echo "$svc: alive but NOT responding (pid $pid, $url -> fail)"
      fi
    else
      echo "$svc: stopped"
    fi
  done
}

case "${1:-start}" in
  start)
    shift || true
    start_stack "$@"
    ;;
  stop)    stop_stack ;;
  status)  status_stack ;;
  restart)
    shift || true
    stop_stack
    start_stack "$@"
    ;;
  logs)
    tail -f "$PIDS/${2:-api}.log"
    ;;
  *)
    echo "Usage: $0 start [--e2e]|stop|status|restart [--e2e]|logs [api|web]" >&2
    exit 1
    ;;
esac
