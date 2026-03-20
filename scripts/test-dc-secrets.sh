#!/usr/bin/env bash
# Tests for dc-secrets (SOPS backend). Run from repo root: bash scripts/test-dc-secrets.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEST_DIR=$(mktemp -d)
export DC_SECRETS_DIR="$TEST_DIR"
DC_SECRETS="$SCRIPT_DIR/dc-secrets"

pass=0; fail=0
assert_eq() {
    local label="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        pass=$((pass + 1)); echo "  PASS: $label"
    else
        fail=$((fail + 1)); echo "  FAIL: $label"; echo "    expected: $expected"; echo "    actual:   $actual"
    fi
}
assert_fail() {
    local label="$1"; shift
    if "$DC_SECRETS" "$@" >/dev/null 2>&1; then
        fail=$((fail + 1)); echo "  FAIL: $label (expected failure, got success)"
    else
        pass=$((pass + 1)); echo "  PASS: $label"
    fi
}

cleanup() { rm -rf "$TEST_DIR"; }
trap cleanup EXIT

echo "--- init ---"
"$DC_SECRETS" init >/dev/null 2>&1
assert_eq "creates identity" "true" "$([[ -f "$TEST_DIR/.age-identity" ]] && echo true || echo false)"
assert_eq "creates sops config" "true" "$([[ -f "$TEST_DIR/.sops.yaml" ]] && echo true || echo false)"
assert_eq "creates shared dir" "true" "$([[ -d "$TEST_DIR/shared" ]] && echo true || echo false)"
assert_eq "creates agents dir" "true" "$([[ -d "$TEST_DIR/agents" ]] && echo true || echo false)"
# Idempotent
"$DC_SECRETS" init >/dev/null 2>&1
assert_eq "init idempotent" "0" "$?"

echo "--- set/get ---"
"$DC_SECRETS" set shared/test KEY1=val1 KEY2=val2
assert_eq "get KEY1" "val1" "$("$DC_SECRETS" get shared/test KEY1)"
assert_eq "get KEY2" "val2" "$("$DC_SECRETS" get shared/test KEY2)"

echo "--- SOPS file structure ---"
first_line=$(head -1 "$TEST_DIR/shared/test.yaml")
assert_eq "keys visible in encrypted file" "true" "$([[ "$first_line" == KEY1:* ]] && echo true || echo false)"
assert_eq "values encrypted" "true" "$([[ "$first_line" == *"ENC["* ]] && echo true || echo false)"

echo "--- update existing key ---"
"$DC_SECRETS" set shared/test KEY1=updated
assert_eq "updated KEY1" "updated" "$("$DC_SECRETS" get shared/test KEY1)"
assert_eq "KEY2 unchanged" "val2" "$("$DC_SECRETS" get shared/test KEY2)"

echo "--- values with special chars ---"
"$DC_SECRETS" set shared/special 'URL=https://example.com/path?q=1&b=2' 'PASS=p@$$w0rd!#'
assert_eq "url value" 'https://example.com/path?q=1&b=2' "$("$DC_SECRETS" get shared/special URL)"
assert_eq "special chars" 'p@$$w0rd!#' "$("$DC_SECRETS" get shared/special PASS)"

echo "--- values with equals sign ---"
"$DC_SECRETS" set shared/eq BASE64=abc=def==
assert_eq "value with =" "abc=def==" "$("$DC_SECRETS" get shared/eq BASE64)"

echo "--- agent-specific creds ---"
"$DC_SECRETS" set agents/a1 AGENT_KEY=secret_a1
"$DC_SECRETS" set agents/a2 AGENT_KEY=secret_a2
assert_eq "agent-1 key" "secret_a1" "$("$DC_SECRETS" get agents/a1 AGENT_KEY)"
assert_eq "agent-2 key" "secret_a2" "$("$DC_SECRETS" get agents/a2 AGENT_KEY)"

echo "--- export ---"
"$DC_SECRETS" set shared/base DB_URL=postgres://localhost
export_out=$("$DC_SECRETS" export --agent a1)
assert_eq "export has shared" "true" "$(echo "$export_out" | grep -q 'DB_URL=postgres://localhost' && echo true || echo false)"
assert_eq "export has agent" "true" "$(echo "$export_out" | grep -q 'AGENT_KEY=secret_a1' && echo true || echo false)"
assert_eq "export no blank lines" "0" "$(echo "$export_out" | grep -c '^$')"

echo "--- export agent override ---"
"$DC_SECRETS" set shared/override SHARED_KEY=shared_val
"$DC_SECRETS" set agents/a3 SHARED_KEY=agent_val
override_out=$("$DC_SECRETS" export --agent a3)
last_val=$(echo "$override_out" | grep '^SHARED_KEY=' | tail -1 | cut -d= -f2-)
assert_eq "agent overrides shared" "agent_val" "$last_val"

echo "--- list ---"
list_out=$("$DC_SECRETS" list)
assert_eq "list has shared/test" "true" "$(echo "$list_out" | grep -q 'shared/test' && echo true || echo false)"
assert_eq "list has agents/a1" "true" "$(echo "$list_out" | grep -q 'agents/a1' && echo true || echo false)"
keys_out=$("$DC_SECRETS" list shared/test)
assert_eq "list keys has KEY1" "true" "$(echo "$keys_out" | grep -q 'KEY1' && echo true || echo false)"

echo "--- delete ---"
"$DC_SECRETS" set shared/del A=1 B=2 C=3
"$DC_SECRETS" delete shared/del B
assert_eq "A still exists" "1" "$("$DC_SECRETS" get shared/del A)"
assert_eq "C still exists" "3" "$("$DC_SECRETS" get shared/del C)"
assert_fail "B is gone" get shared/del B

echo "--- delete last key removes file ---"
"$DC_SECRETS" set shared/single ONLY=one
"$DC_SECRETS" delete shared/single ONLY
assert_eq "file removed" "false" "$([[ -f "$TEST_DIR/shared/single.yaml" ]] && echo true || echo false)"

echo "--- import ---"
cat > "$TEST_DIR/test.env" <<'ENVEOF'
# Comment line
export FOO=bar
BAZ=qux

EMPTY_LINE_ABOVE=yes
ENVEOF
"$DC_SECRETS" import "$TEST_DIR/test.env" shared/imported
assert_eq "imported FOO" "bar" "$("$DC_SECRETS" get shared/imported FOO)"
assert_eq "imported BAZ" "qux" "$("$DC_SECRETS" get shared/imported BAZ)"
assert_eq "imported EMPTY_LINE_ABOVE" "yes" "$("$DC_SECRETS" get shared/imported EMPTY_LINE_ABOVE)"

echo "--- error paths ---"
assert_fail "get nonexistent file" get shared/nonexistent KEY
assert_fail "get nonexistent key" get shared/test NONEXISTENT
assert_fail "delete nonexistent file" delete shared/nonexistent KEY
assert_fail "delete nonexistent key" delete shared/test NONEXISTENT
assert_fail "import nonexistent file" import /nonexistent shared/x
assert_fail "set bad format" set shared/x badformat
assert_fail "unknown command" bogus

echo "--- concurrent writes ---"
for i in $(seq 1 10); do
    "$DC_SECRETS" set shared/concurrent "KEY_$i=val_$i" &
done
wait
concurrent_keys=$("$DC_SECRETS" list shared/concurrent | wc -l)
assert_eq "concurrent: all 10 keys present" "10" "$concurrent_keys"

echo ""
echo "========================================="
echo "Results: $pass passed, $fail failed"
echo "========================================="
[[ $fail -eq 0 ]] || exit 1
