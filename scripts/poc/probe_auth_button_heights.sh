#!/usr/bin/env sh
set -eu

BASE_URL="${1:-${DC_WEB_URL:-https://dev.decent-cloud.org}}"

probe_page() {
  page_path="$1"
  page_url="$BASE_URL$page_path"
  echo "[probe] auth/button heights on: $page_url" >&2
  node scripts/browser.js eval "$page_url" "(() => {
    const selectors = ['button', 'a'];
    const nodes = [...document.querySelectorAll(selectors.join(','))]
      .filter((el) => {
        const txt = (el.textContent || '').trim();
        if (!txt) return false;
        const normalized = txt.toLowerCase();
        const match = normalized.includes('sign in')
          || normalized.includes('sign in with google')
          || normalized.includes('sign in with seed phrase instead')
          || normalized.includes('back to home');
        if (!match) return false;
        const r = el.getBoundingClientRect();
        return r.width > 0 && r.height > 0;
      })
      .map((el) => {
        const r = el.getBoundingClientRect();
        return {
          text: (el.textContent || '').trim(),
          heightPx: Math.round(r.height),
          className: String(el.className || ''),
        };
      });

    const heights = nodes.map((n) => n.heightPx);
    const min = heights.length ? Math.min(...heights) : null;
    const max = heights.length ? Math.max(...heights) : null;
    const delta = min === null || max === null ? null : max - min;

    return {
      url: location.href,
      controls: nodes,
      stats: { minHeight: min, maxHeight: max, delta },
    };
  })()"
}

probe_page "/login"
probe_page "/dashboard/marketplace"
