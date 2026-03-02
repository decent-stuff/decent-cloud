#!/usr/bin/env sh
set -eu

echo "[poc] compare URL baseline vs target canonicalization"

node <<'EOF'
const COMPARE_MAX = 3;

function baselineParse(rawIds) {
  return rawIds
    .split(',')
    .map((s) => parseInt(s.trim(), 10))
    .filter((n) => !Number.isNaN(n) && n > 0)
    .slice(0, COMPARE_MAX);
}

function canonicalize(rawIds) {
  const seen = new Set();
  const normalized = [];

  for (const token of rawIds.split(',')) {
    const value = token.trim();
    if (!/^\d+$/.test(value)) continue;

    const id = Number(value);
    if (!Number.isSafeInteger(id) || id <= 0 || seen.has(id)) continue;

    seen.add(id);
    normalized.push(id);
    if (normalized.length >= COMPARE_MAX) break;
  }

  return normalized;
}

const cases = [
  '1,2,3',
  ' 3 , 2, 2, 1 ',
  '2abc,03,0,-1,4',
  '1,2,3,4,5',
  ',, 10 ,x, 11.1,12 '
];

for (const raw of cases) {
  const baseline = baselineParse(raw);
  const target = canonicalize(raw);
  const baselineUrl = `/dashboard/marketplace/compare?ids=${baseline.join(',')}`;
  const targetUrl = `/dashboard/marketplace/compare?ids=${target.join(',')}`;

  console.log(JSON.stringify({ raw, baseline, target, baselineUrl, targetUrl }));
}
EOF
