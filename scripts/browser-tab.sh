#!/usr/bin/env bash
# Browser tab lifecycle management via CDP HTTP API.
#
# Usage:
#   browser-tab.sh open             → create blank tab, print TAB_ID to stdout
#   browser-tab.sh open <url>       → create tab at URL, print TAB_ID to stdout
#   browser-tab.sh close <TAB_ID>   → close the tab
#   browser-tab.sh list             → list open page tabs as JSON
#
# The CDP HTTP base URL defaults to http://192.168.0.13:9223.
# Override via BROWSER_CDP_URL env var.
#
# Example workflow:
#   TAB_ID=$(bash scripts/browser-tab.sh open https://dev.decent-cloud.org)
#   # ... use browser MCP tools ...
#   bash scripts/browser-tab.sh close "$TAB_ID"

set -euo pipefail

CDP_HTTP="${BROWSER_CDP_URL:-http://192.168.0.13:9223}"

_require_jq() {
  if ! command -v jq &>/dev/null; then
    echo "ERROR: jq is required but not installed." >&2
    exit 1
  fi
}

case "${1:-}" in
  open)
    _require_jq
    URL="${2:-about:blank}"
    RESULT=$(curl -sf --request PUT "${CDP_HTTP}/json/new?${URL}" 2>&1) || {
      echo "ERROR: Cannot connect to Chrome CDP at ${CDP_HTTP}" >&2
      echo "  Is Chrome running with --remote-debugging-port?" >&2
      echo "  Expected: google-chrome --remote-debugging-port=9223 --remote-debugging-address=0.0.0.0" >&2
      exit 1
    }
    TAB_ID=$(echo "$RESULT" | jq -r '.id // empty')
    if [ -z "$TAB_ID" ]; then
      echo "ERROR: Failed to create tab. CDP response: $RESULT" >&2
      exit 1
    fi
    echo "$TAB_ID"
    ;;

  close)
    TAB_ID="${2:?ERROR: Tab ID required. Usage: browser-tab.sh close <TAB_ID>}"
    curl -sf "${CDP_HTTP}/json/close/${TAB_ID}" >/dev/null || {
      echo "WARN: Failed to close tab ${TAB_ID} (may already be closed)" >&2
    }
    ;;

  list)
    _require_jq
    curl -sf "${CDP_HTTP}/json/list" \
      | jq '[.[] | select(.type == "page") | {id, title, url}]'
    ;;

  *)
    echo "Usage: $0 open [url] | close <tab_id> | list" >&2
    exit 1
    ;;
esac
