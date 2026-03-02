#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <url> <seed phrase>"
  exit 1
fi

URL="$1"
SEED="$2"
OUT="$(mktemp)"

node scripts/browser.js snap "$URL" --seed "$SEED" > "$OUT"

echo "Onboarding probe for $URL"
echo "- contains 'Complete your profile': $(rg -q "Complete your profile" "$OUT" && echo yes || echo no)"
echo "- contains 'Add your SSH key': $(rg -q "Add your SSH key" "$OUT" && echo yes || echo no)"
echo "- contains 'Choose your next action': $(rg -q "Choose your next action" "$OUT" && echo yes || echo no)"
echo "- contains legacy 'Welcome to Decent Cloud': $(rg -q "Welcome to Decent Cloud" "$OUT" && echo yes || echo no)"

rm -f "$OUT"
