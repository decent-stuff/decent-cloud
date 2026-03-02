#!/usr/bin/env sh
set -eu

BASE_URL="${1:-${DC_WEB_URL:-https://dev.decent-cloud.org}}"
URL="$BASE_URL/dashboard/marketplace"

echo "[probe] checking quick-filter and preset pill classes on: $URL" >&2
node scripts/browser.js eval "$URL" "(() => { const byLabel = (label) => [...document.querySelectorAll('button')].find((b) => (b.textContent || '').includes(label)); const labels = ['Recently Added', 'Most Trusted', 'GPU Servers', 'North America', 'Europe']; const classes = labels.map((label) => { const el = byLabel(label); return { label, className: el ? el.className : null }; }); return { url: location.href, pills: classes }; })()"
