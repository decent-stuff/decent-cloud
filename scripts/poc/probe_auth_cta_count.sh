#!/usr/bin/env sh
set -eu

BASE_URL="${1:-${DC_WEB_URL:-https://dev.decent-cloud.org}}"
URL="$BASE_URL/dashboard/marketplace"

echo "[probe] checking auth CTA count on: $URL" >&2
node scripts/browser.js eval "$URL" "(() => { const signInEls = [...document.querySelectorAll('button,a')].filter((el) => { const txt = (el.textContent || '').trim().toLowerCase(); if (!txt.includes('sign in')) return false; const rect = el.getBoundingClientRect(); return rect.width > 0 && rect.height > 0; }); return { url: location.href, signInCtaCount: signInEls.length, labels: signInEls.map((el) => (el.textContent || '').trim()) }; })()"
