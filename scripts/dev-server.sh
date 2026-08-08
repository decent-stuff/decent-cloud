#!/usr/bin/env bash
# Local dev environment: SvelteKit website (port 59010) + local API (port 59011).
#
# One source of truth for stack lifecycle. Detached (setsid) so processes
# survive the caller's exit; idempotent so re-running start is a no-op if the
# stack is already healthy.
#
# The API runs in RELEASE by default. Debug builds run Ed25519 curve math
# ~150x slower than release (~17ms vs ~106us per auth), which both distorts
# E2E timing and does not reflect production. Use --dev for fast Rust iteration
# (incremental debug build ~1-44s vs ~6min release).
#
# Usage:
#   scripts/dev-server.sh start [--e2e|--dev]   — start stack (idempotent)
#   scripts/dev-server.sh stop                  — stop stack (process-group kill)
#   scripts/dev-server.sh status                — show running status with port health
#   scripts/dev-server.sh restart [--e2e|--dev] — stop + start
#   scripts/dev-server.sh logs [api|web]        — tail merged stdout log
#
# Modes:
#   default   — website always local; API uses remote dev if no local binary.
#   --dev     — serves the DEBUG api binary (fast iteration, honors CARGO_TARGET_DIR).
#               Mutually exclusive with --e2e. Build first: cargo build -p api --bin api-server
#   --e2e     — forces LOCAL RELEASE api (no remote fallback), builds binary if missing,
#               disables rate limiting so parallel test workers don't 429.
#
# Test entrypoints (see website/package.json):
#   npm run e2e:up           = scripts/dev-server.sh start --e2e
#   npm run test:e2e:fast    = playwright against pre-started stack (no spawn)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Per-stack isolation: STACK_INDEX (env, default 0) selects a separate pid/log
# dir so multiple stacks can coexist (used by scripts/e2e-shard.sh). Stack 0
# keeps the legacy .dev-pids/ location for backward compatibility.
STACK_INDEX="${STACK_INDEX:-0}"
if [ "$STACK_INDEX" -eq 0 ] 2>/dev/null; then
  PIDS="$ROOT/.dev-pids"
else
  PIDS="$ROOT/.dev-pids/stack-$STACK_INDEX"
fi
API_PORT="${API_PORT:-59011}"
WEB_PORT="${WEB_PORT:-59010}"
REMOTE_API_URL="https://dev-api.decent-cloud.org"
# Honor CARGO_TARGET_DIR: CI builds the api-server into $CARGO_TARGET_DIR (not
# $ROOT/target), so looking only under $ROOT/target means `env` fails to exec the
# binary with "No such file or directory" and the health check times out. Fall back
# to $ROOT/target for local dev (where CARGO_TARGET_DIR is typically unset).
API_BINARY="${API_BINARY:-${CARGO_TARGET_DIR:-$ROOT/target}/release/api-server}"
DEFAULT_CANISTER_ID="ggi4a-wyaaa-aaaai-actqq-cai"

# Source all env vars from cf/.env.dev (operator-local, gitignored). When absent
# (e.g. fresh CI checkout, where the gitignored file is never present), fall back
# to the tracked cf/.env.dev.example so the --e2e warm stack can boot with
# local-loop defaults — honoring the intent above that .env.dev is optional in
# --e2e mode. cf/.env.dev itself is never committed (operator-local secrets).
_env_file=""
_using_example=0
if [ -f "$ROOT/cf/.env.dev" ]; then
  _env_file="$ROOT/cf/.env.dev"
elif [ -f "$ROOT/cf/.env.dev.example" ]; then
  _env_file="$ROOT/cf/.env.dev.example"
  _using_example=1
  echo "warning: cf/.env.dev not found — using tracked cf/.env.dev.example." >&2
  echo "         (CI / throwaway --e2e stack. For local dev: cp cf/.env.dev.example cf/.env.dev)" >&2
else
  echo "error: neither cf/.env.dev nor cf/.env.dev.example found — repo is incomplete." >&2
  exit 1
fi

# The env file's API_DATABASE_URL is a local-loop placeholder (hostname `postgres`).
# A caller-provided DATABASE_URL names the real Postgres host, so when using the
# example fallback prefer it over the placeholder to avoid pointing the API at the wrong DB.
_caller_db_url="${DATABASE_URL:-}"
# shellcheck disable=SC1090
set -a
# shellcheck source=/dev/null
source "$_env_file"
set +a
if [ "$_using_example" -eq 1 ] && [ -n "$_caller_db_url" ]; then
  export API_DATABASE_URL="$_caller_db_url"
fi
unset _env_file _using_example _caller_db_url

E2E_MODE=0
DEV_MODE=0
for arg in "$@"; do
  case "$arg" in
    --e2e) E2E_MODE=1 ;;
    --dev) DEV_MODE=1 ;;
  esac
done
if [ "$E2E_MODE" -eq 1 ] && [ "$DEV_MODE" -eq 1 ]; then
  echo "error: --dev and --e2e are mutually exclusive (e2e needs release for correct auth timing)" >&2
  exit 1
fi
if [ "$DEV_MODE" -eq 1 ]; then
  API_BINARY="${CARGO_TARGET_DIR:-$ROOT/target}/debug/api-server"
  if [ ! -x "$API_BINARY" ]; then
    echo "error: --dev requires a debug binary at $API_BINARY" >&2
    echo "       build it first: CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target} cargo build -p api --bin api-server" >&2
    exit 1
  fi
fi

# Resolve effective env for the API server.
# API_DATABASE_URL (from cf/.env.dev) wins; DATABASE_URL is the fallback.
effective_db_url() {
  printf '%s' "${API_DATABASE_URL:-${DATABASE_URL:-postgres://test:test@postgres:5432/test}}"
}

# Populate SECRETS_ENV with SOPS-managed key=value pairs for child services
# (best-effort; mirrors agent/entrypoint.sh). Explicit dev-server vars below
# always win (they come AFTER in the `env` arg list, and `env` is last-wins).
# Placeholder values ("<set-me>") are skipped so the corresponding feature
# disables cleanly via env-absence rather than running on a broken credential.
SECRETS_ENV=()
load_secrets_env() {
  SECRETS_ENV=()
  local dc_secrets="$ROOT/scripts/dc-secrets"
  if [ ! -x "$dc_secrets" ]; then
    echo "warning: $dc_secrets unavailable — services will run WITHOUT SOPS secrets." >&2
    echo "         (set up the age key: see agent/docs/secrets.md)" >&2
    return 0
  fi
  local output
  if ! output=$(env DC_SECRETS_DIR="${DC_SECRETS_DIR:-$ROOT/secrets}" "$dc_secrets" export play 2>/dev/null); then
    echo "warning: dc-secrets export play failed — services will run WITHOUT SOPS secrets." >&2
    echo "         (is the age key present? see agent/docs/secrets.md)" >&2
    return 0
  fi
  local line n=0
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    case "$line" in
      *='<set-me>') continue ;;   # placeholder — leave unset so the feature disables cleanly
      *=*)
        # dc-secrets export is eval-formatted (designed for `source`). Inline
        # comments (KEY=val  # note) are stripped by bash but passed literally
        # by `env`, so "30  # default" would fail u64 parsing. Strip them.
        line="${line%% *# *}"
        SECRETS_ENV+=("$line"); n=$((n + 1)) ;;
    esac
  done <<< "$output"
  echo "Loaded $n secret(s) from SOPS store (placeholders skipped)."
}

_wait_for() {
  # Args: <svc-key> <label> <url> <deadline-seconds>. The svc-key (api|web) names the
  # $PIDS/<key>.log / -stderr.log files so a timeout can dump WHY the service didn't
  # come up — a health-check timeout with no diagnostics is a blind spot.
  local key="$1" label="$2" url="$3" deadline="$4" now deadline_s
  now=$(date +%s)
  deadline_s=$((now + deadline))
  echo -n "Waiting for $label"
  while [ "$(date +%s)" -lt "$deadline_s" ]; do
    if curl -sf "$url" >/dev/null 2>&1; then
      echo " ready"
      return 0
    fi
    echo -n "."
    sleep 1
  done
  echo " TIMEOUT (${deadline}s)" >&2
  # BE LOUD: surface the service's own startup logs so CI shows the real reason
  # (bind error / panic / migration failure) instead of a bare timeout.
  echo "----- $label stdout ($PIDS/$key.log) -----" >&2
  if [ -f "$PIDS/$key.log" ]; then tail -n 50 "$PIDS/$key.log" >&2; else echo "(none)" >&2; fi
  echo "----- $label stderr ($PIDS/$key-stderr.log) -----" >&2
  if [ -f "$PIDS/$key-stderr.log" ]; then cat "$PIDS/$key-stderr.log" >&2; else echo "(none)" >&2; fi
  echo "----------------------------------------------------------" >&2
  return 1
}

# Loudly announce which API binary is being served. The RELEASE binary is the
# deliberate default (debug Ed25519 is ~150x slower, distorting e2e timing), but
# the choice is easy to miss — a sibling agent once rebuilt the DEBUG binary
# thinking it would affect the already-running RELEASE server (it does not: a
# running server keeps the old mapped binary until restarted). Make it unmissable.
_announce_api_binary() {
  local kind
  case "$API_BINARY" in
    */release/*) kind="RELEASE" ;;
    */debug/*)   kind="DEBUG" ;;
    *)           kind="custom path" ;;
  esac
  echo "▶ API binary: $API_BINARY  [$kind]"
  case "$kind" in
    RELEASE)
      echo "  Release binary (default). A running server KEEPS the old binary until"
      echo "  restarted — rebuilding does NOT hot-swap it. Rebuild after Rust edits:"
      echo "    cargo build -p api --bin api-server --release"
      echo "  (then: scripts/dev-server.sh restart). For fast Rust iteration without"
      echo "   the ~6min release rebuild: scripts/dev-server.sh restart --dev"
      ;;
    DEBUG)
      echo "  Debug binary — Ed25519 is ~150x slower; e2e timing is distorted and"
      echo "  does not reflect production. Rebuild after Rust edits:"
      echo "    cargo build -p api --bin api-server"
      ;;
  esac
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

# Reclaim a dev-stack port from a stale occupant before binding. The api (59011) and
# web (59010) ports are dedicated to this stack, so any process holding one that we do
# not track is a leftover from a prior run whose detached (setsid) service escaped job
# cleanup. Without this the new service hits EADDRINUSE, dies silently, and the health
# check times out — the failure that wedged the E2E job. Loud by design.
_reclaim_port() {
  local port="$1" label="$2" pid i
  pid=$(ss -lptnH "sport = :$port" 2>/dev/null | sed -n 's/.*pid=\([0-9]\+\).*/\1/p' | head -1)
  [ -n "$pid" ] || return 0
  echo "warning: $label port $port held by stale process (pid $pid) — reclaiming." >&2
  kill -TERM "$pid" 2>/dev/null || true
  for i in 1 2 3 4 5; do
    kill -0 "$pid" 2>/dev/null || return 0
    sleep 0.5
  done
  echo "warning: pid $pid did not exit on TERM; sending KILL." >&2
  kill -KILL "$pid" 2>/dev/null || true
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

  # Make the warm stack secret-aware: pull SOPS-managed keys (best-effort).
  load_secrets_env

  # ── API server ───────────────────────────────────────────────────────────
  local api_url
  if [ "$E2E_MODE" -eq 1 ]; then
    # E2E MUST run against current source. cargo's fingerprinting makes the
    # build below a fast no-op when nothing changed and a correct rebuild when
    # it did. Do NOT "optimize" this back to a build-only-if-missing guard: on
    # persistent CARGO_TARGET_DIR caches (e.g. the self-hosted CI runner) that
    # silently serves a stale pre-merge binary and tests pass/fail against the
    # wrong code (seen 2026-08-03: PR #456 e2e ran against the merged-#455
    # binary, so only the newest-commit-dependent spec failed).
    echo "E2E mode: ensuring $API_BINARY is up to date (cargo build -p api --bin api-server --release)..."
    (cd "$ROOT" && cargo build -p api --bin api-server --release)
    if _healthy api "http://localhost:$API_PORT/api/v1/health"; then
      echo "API already running (pid $(cat "$PIDS/api.pid"))"
    else
      echo "Starting local API on :$API_PORT (e2e profile, rate-limit disabled)..."
      _announce_api_binary
      _reclaim_port "$API_PORT" "API"
      _start_service api "$ROOT" \
        "${SECRETS_ENV[@]}" \
        "DATABASE_URL=$(effective_db_url)" \
        "API_SERVER_PORT=$API_PORT" \
        "FRONTEND_URL=http://localhost:$WEB_PORT" \
        "SQLX_OFFLINE=true" \
        "CANISTER_ID=${CANISTER_ID:-$DEFAULT_CANISTER_ID}" \
        "RATE_LIMIT_ENABLED=false" \
        "STRIPE_WEBHOOK_SECRET=whsec_test_secret" \
        "$API_BINARY" serve
      _wait_for api "local API" "http://localhost:$API_PORT/api/v1/health" 120 || return 1
    fi
    api_url="http://localhost:$API_PORT"
  elif [ -x "$API_BINARY" ]; then
    if _healthy api "http://localhost:$API_PORT/api/v1/health"; then
      echo "API already running (pid $(cat "$PIDS/api.pid"))"
    else
      echo "Starting local API on :$API_PORT..."
      _announce_api_binary
      _reclaim_port "$API_PORT" "API"
      _start_service api "$ROOT" \
        "${SECRETS_ENV[@]}" \
        "DATABASE_URL=$(effective_db_url)" \
        "API_SERVER_PORT=$API_PORT" \
        "FRONTEND_URL=http://localhost:$WEB_PORT" \
        "SQLX_OFFLINE=true" \
        "CANISTER_ID=${CANISTER_ID:-$DEFAULT_CANISTER_ID}" \
        "$API_BINARY" serve
      _wait_for api "local API" "http://localhost:$API_PORT/api/v1/health" 120 || return 1
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
    _reclaim_port "$WEB_PORT" "website"
    _start_service web "$ROOT/website" \
      "VITE_DECENT_CLOUD_API_URL=$api_url" \
      "VITE_CHATWOOT_WEBSITE_TOKEN=" \
      "VITE_CHATWOOT_BASE_URL=" \
      npm run dev -- --host 127.0.0.1 --port "$WEB_PORT" --strictPort
    _wait_for web "website" "http://localhost:$WEB_PORT" 60 || return 1
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
    echo "Usage: $0 start [--e2e|--dev]|stop|status|restart [--e2e|--dev]|logs [api|web]" >&2
    exit 1
    ;;
esac
