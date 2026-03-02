#!/usr/bin/env sh
set -eu

BASE_URL="${1:-${DC_WEB_URL:-https://dev.decent-cloud.org}}"
URL="$BASE_URL/dashboard/marketplace"

echo "[probe] checking row action control heights on: $URL" >&2
node scripts/browser.js eval "$URL" "(() => { const wanted = ['Rent', 'Save', 'Saved', '+ Compare', '✓ Compare']; const items = [...document.querySelectorAll('button,a')]
  .filter((el) => wanted.includes((el.textContent || '').trim()))
  .filter((el) => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; })
  .slice(0, 12)
  .map((el) => ({ text: (el.textContent || '').trim(), heightPx: Math.round(el.getBoundingClientRect().height), className: el.className }));
return { url: location.href, controls: items }; })()"
