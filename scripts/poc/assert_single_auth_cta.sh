#!/usr/bin/env sh
set -eu

BASE_URL="${1:-${DC_WEB_URL:-http://127.0.0.1:59010}}"
URL="$BASE_URL/dashboard/marketplace"
MAX_CTA="${MAX_CTA:-1}"

RESULT="$(node scripts/browser.js eval "$URL" "(() => { const signInEls = [...document.querySelectorAll('button,a')].filter((el) => { const txt = (el.textContent || '').trim().toLowerCase(); if (!txt.includes('sign in')) return false; const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; }); return { count: signInEls.length, labels: signInEls.map((el) => (el.textContent || '').trim()) }; })()")"
COUNT="$(printf '%s' "$RESULT" | grep -o '"count": [0-9]*' | head -n1 | awk '{print $2}')"

if [ -z "$COUNT" ]; then
  echo "[assert] failed to parse Sign In CTA count" >&2
  echo "$RESULT" >&2
  exit 1
fi

if [ "$COUNT" -gt "$MAX_CTA" ]; then
  echo "[assert] expected <= $MAX_CTA visible Sign In CTA, got $COUNT at $URL" >&2
  echo "$RESULT" >&2
  exit 1
fi

echo "[assert] pass: visible Sign In CTA count = $COUNT (<= $MAX_CTA)"
