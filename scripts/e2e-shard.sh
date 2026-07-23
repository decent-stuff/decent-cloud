#!/usr/bin/env bash
# Run the Playwright e2e suite sharded across N warm stacks for radical speed.
#
# The single-stack ceiling is ~150s no matter how many workers you throw at it
# (measured: 4w=151s clean, 8w+ degrades — the single API saturates, not the
# box). This script fans the suite out across N independent API+Web stacks
# (each its own port pair + connection pool, sharing the single Postgres) and
# runs Playwright's --shard=i/N against stack i. 3 stacks x 4 workers targets
# ~50s reliably green.
#
# Usage:
#   scripts/e2e-shard.sh [N] [--no-teardown]
#   E2E_SHARDS=3 scripts/e2e-shard.sh
#   E2E_WORKERS=4 scripts/e2e-shard.sh 3
#
# Port scheme per stack i (1-based): web = 59100 + i*10, api = +1.
#   stack 1 -> 59110/59111, stack 2 -> 59120/59121, stack 3 -> 59130/59131
#
# Logs:   .dev-pids/shard-runs/shard-<i>.log
# Stacks: managed via STACK_INDEX (see dev-server.sh start --e2e).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEV="$ROOT/scripts/dev-server.sh"
WEB_DIR="$ROOT/website"

N=""
NO_TEARDOWN=0
for arg in "$@"; do
  case "$arg" in
    --no-teardown) NO_TEARDOWN=1 ;;
    *) N="$arg" ;;
  esac
done
N="${N:-${E2E_SHARDS:-3}}"

if ! [[ "$N" =~ ^[0-9]+$ ]] || [ "$N" -lt 1 ]; then
  echo "usage: $0 <num-shards> [--no-teardown]" >&2
  exit 1
fi

WORKERS_PER_SHARD="${E2E_WORKERS:-4}"
web_port() { echo $((59100 + $1 * 10)); }
api_port() { echo $((59100 + $1 * 10 + 1)); }

SHARD_LOGS="$ROOT/.dev-pids/shard-runs"
mkdir -p "$SHARD_LOGS"

cleanup() {
  if [ "$NO_TEARDOWN" -eq 1 ]; then
    echo ">> --no-teardown: leaving stacks up"
    return
  fi
  echo ">> Tearing down $N stacks..."
  local i
  for i in $(seq 1 "$N"); do
    STACK_INDEX="$i" "$DEV" stop >/dev/null 2>&1 &
  done
  wait
}
trap cleanup EXIT

# 1. Bring up N stacks in parallel (each own port pair + pid dir).
echo ">> Starting $N stacks ($WORKERS_PER_SHARD workers/shard)..."
declare -a stack_pids
for i in $(seq 1 "$N"); do
  STACK_INDEX="$i" WEB_PORT="$(web_port "$i")" API_PORT="$(api_port "$i")" "$DEV" start --e2e &
  stack_pids[$i]=$!
done
start_fail=0
for i in $(seq 1 "$N"); do
  if ! wait "${stack_pids[$i]}"; then
    echo "!! stack $i failed to start" >&2
    start_fail=1
  fi
done
if [ "$start_fail" -ne 0 ]; then
  echo "!! Aborting: one or more stacks failed to start." >&2
  exit 1
fi

# 2. Run N shards in parallel.
echo ">> Running $N shards..."
declare -a shard_pids
for i in $(seq 1 "$N"); do
  (
    cd "$WEB_DIR"
    PLAYWRIGHT_BASE_URL="http://localhost:$(web_port "$i")" \
    PLAYWRIGHT_API_URL="http://localhost:$(api_port "$i")" \
    E2E_WORKERS="$WORKERS_PER_SHARD" \
    npx playwright test --shard="$i/$N" --reporter=list >"$SHARD_LOGS/shard-$i.log" 2>&1
  ) &
  shard_pids[$i]=$!
done

# 3. Aggregate exit codes (report each shard's result).
shard_fail=0
for i in $(seq 1 "$N"); do
  if wait "${shard_pids[$i]}"; then
    echo ">> shard $i/$N: PASS"
  else
    echo "!! shard $i/$N: FAIL — see $SHARD_LOGS/shard-$i.log" >&2
    shard_fail=1
  fi
done

if [ "$shard_fail" -eq 0 ]; then
  echo ">> All $N shards passed."
else
  echo "!! One or more shards failed." >&2
fi
exit "$shard_fail"
