#!/usr/bin/env sh
set -eu

BASE_URL="${1:-${DC_WEB_URL:-https://dev.decent-cloud.org}}"
THRESHOLD_PX="${2:-2}"

MARKETPLACE_URL="$BASE_URL/dashboard/marketplace?q=gpu"
RENTALS_URL="$BASE_URL/dashboard/rentals"

echo "[probe] checking dashboard CTA peer height consistency (threshold=${THRESHOLD_PX}px)" >&2

MARKETPLACE_RESULT="$(node scripts/browser.js eval "$MARKETPLACE_URL" "(() => {
  const threshold = Number('${THRESHOLD_PX}');

  function readHeights(elements) {
    return elements.map((el) => ({
      text: (el.textContent || '').trim().replace(/\s+/g, ' '),
      heightPx: Math.round(el.getBoundingClientRect().height),
      className: String(el.className || '')
    }));
  }

  function findActiveFilterPeers() {
    const label = [...document.querySelectorAll('span')]
      .find((el) => (el.textContent || '').trim() === 'Active filters:');
    if (!label || !label.parentElement) return [];
    return [...label.parentElement.querySelectorAll('button,a')];
  }

  function findSortPeers() {
    const sortLabel = [...document.querySelectorAll('span')]
      .find((el) => (el.textContent || '').trim() === 'Sort:');
    if (!sortLabel || !sortLabel.parentElement) return [];
    return [...sortLabel.parentElement.querySelectorAll('button,a')];
  }

  function evaluateGroup(name, elements) {
    const controls = readHeights(elements).filter((item) => item.heightPx > 0);
    if (controls.length < 2) {
      return { name, skipped: true, reason: 'not-enough-visible-controls', controls };
    }

    const heights = controls.map((item) => item.heightPx);
    const minHeight = Math.min(...heights);
    const maxHeight = Math.max(...heights);
    const deltaPx = maxHeight - minHeight;

    return {
      name,
      skipped: false,
      controls,
      minHeight,
      maxHeight,
      deltaPx,
      pass: deltaPx <= threshold
    };
  }

  const groups = [
    evaluateGroup('marketplace.activeFilters', findActiveFilterPeers()),
    evaluateGroup('marketplace.sortPills', findSortPeers())
  ];

  const violations = groups.filter((group) => !group.skipped && !group.pass);
  return {
    route: location.pathname,
    threshold,
    groups,
    pass: violations.length === 0,
    violations
  };
})()")"

printf '%s\n' "$MARKETPLACE_RESULT"

node -e "const result = JSON.parse(process.argv[1]); if (!result.pass) { process.exit(1); }" "$MARKETPLACE_RESULT"

RENTALS_RESULT="$(node scripts/browser.js eval "$RENTALS_URL" "(() => {
  const threshold = Number('${THRESHOLD_PX}');

  function readHeights(elements) {
    return elements.map((el) => ({
      text: (el.textContent || '').trim().replace(/\s+/g, ' '),
      heightPx: Math.round(el.getBoundingClientRect().height),
      className: String(el.className || '')
    }));
  }

  const tabButtons = [...document.querySelectorAll('button')]
    .filter((el) => {
      const text = (el.textContent || '').trim();
      return ['All', 'Active', 'Pending', 'Cancelled / Failed'].includes(text);
    });

  const controls = readHeights(tabButtons).filter((item) => item.heightPx > 0);
  if (controls.length < 2) {
    return {
      route: location.pathname,
      threshold,
      group: {
        name: 'rentals.statusTabs',
        skipped: true,
        reason: 'not-enough-visible-controls',
        controls
      },
      pass: true,
      violations: []
    };
  }

  const heights = controls.map((item) => item.heightPx);
  const minHeight = Math.min(...heights);
  const maxHeight = Math.max(...heights);
  const deltaPx = maxHeight - minHeight;
  const pass = deltaPx <= threshold;

  return {
    route: location.pathname,
    threshold,
    group: {
      name: 'rentals.statusTabs',
      skipped: false,
      controls,
      minHeight,
      maxHeight,
      deltaPx,
      pass
    },
    pass,
    violations: pass ? [] : [{
      name: 'rentals.statusTabs',
      minHeight,
      maxHeight,
      deltaPx
    }]
  };
})()")"

printf '%s\n' "$RENTALS_RESULT"

node -e "const result = JSON.parse(process.argv[1]); if (!result.pass) { process.exit(1); }" "$RENTALS_RESULT"
