#!/usr/bin/env bash
# Tests for dc-secrets (SOPS backend). Run from repo root: bash scripts/test-dc-secrets.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEST_DIR=$(mktemp -d)
export DC_SECRETS_DIR="$TEST_DIR"
# Sandboxes export SOPS_AGE_KEY_FILE; the test must start without an ambient key so
# `init` generates a fresh bootstrap key (and the portable-key tests below control
# their own key sources explicitly).
unset SOPS_AGE_KEY SOPS_AGE_KEY_FILE
# Hermetic: existing checks use age-only stores (no dependency on a gpg key). The
# dedicated pgp-recipient check below opts back in with a throwaway fingerprint.
export DC_SOPS_PGP_RECIPIENT=
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

EXTRA_DIRS=()
cleanup() { rm -rf "$TEST_DIR" "${EXTRA_DIRS[@]}"; }
trap cleanup EXIT

echo "--- init ---"
"$DC_SECRETS" init >/dev/null 2>&1
assert_eq "creates identity" "true" "$([[ -f "$TEST_DIR/.age-identity" ]] && echo true || echo false)"
assert_eq "creates sops config" "true" "$([[ -f "$TEST_DIR/.sops.yaml" ]] && echo true || echo false)"
assert_eq "creates shared dir" "true" "$([[ -d "$TEST_DIR/shared" ]] && echo true || echo false)"
assert_eq "creates agents dir" "true" "$([[ -d "$TEST_DIR/agents" ]] && echo true || echo false)"
# NOTE: `hires/` is the GENERIC dc-secrets per-name overlay layer (still created
# by `init`). This assertion checks the tool's `init` behaviour, not any
# particular storage policy.
assert_eq "creates hires dir" "true" "$([[ -d "$TEST_DIR/hires" ]] && echo true || echo false)"
# Idempotent
"$DC_SECRETS" init >/dev/null 2>&1
assert_eq "init idempotent" "0" "$?"

echo "--- sops config: age + operator-gpg recipients ---"
# DC_SOPS_PGP_RECIPIENT drives the pgp: line; use a throwaway value so the check
# never couples to the real operator fingerprint.
pgp_dir=$(mktemp -d); EXTRA_DIRS+=("$pgp_dir")
DC_SOPS_PGP_RECIPIENT="FAKEFINGERPRINTFORTHISTEST" DC_SECRETS_DIR="$pgp_dir" \
    "$DC_SECRETS" init >/dev/null 2>&1
cfg="$pgp_dir/.sops.yaml"
assert_eq "config has age recipient" "true" "$(grep -Eq '^[[:space:]]*age:' "$cfg" && echo true || echo false)"
assert_eq "config has pgp recipient" "true" "$(grep -Eq '^[[:space:]]*pgp:' "$cfg" && echo true || echo false)"
assert_eq "pgp recipient is the env override" "true" "$(grep -q 'pgp: FAKEFINGERPRINTFORTHISTEST' "$cfg" && echo true || echo false)"
# Opt-out: an empty DC_SOPS_PGP_RECIPIENT yields an age-ONLY rule (no pgp line).
nogpg_dir=$(mktemp -d); EXTRA_DIRS+=("$nogpg_dir")
DC_SOPS_PGP_RECIPIENT="" DC_SECRETS_DIR="$nogpg_dir" "$DC_SECRETS" init >/dev/null 2>&1
assert_eq "empty pgp recipient omits pgp line" "false" "$(grep -Eq '^[[:space:]]*pgp:' "$nogpg_dir/.sops.yaml" && echo true || echo false)"

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

echo "--- export: layered model ---"
"$DC_SECRETS" set shared/common COMMON_KEY=common_val DB_URL=postgres://localhost
"$DC_SECRETS" set shared/prod PROD_ONLY=secret DB_URL=postgres://prod
# Bare export = common-only (no env leakage) — the root-cause regression.
bare_out=$("$DC_SECRETS" export)
assert_eq "bare export has common key" "common_val" "$(echo "$bare_out" | grep '^COMMON_KEY=' | cut -d= -f2-)"
assert_eq "bare export does NOT leak prod" "false" "$(echo "$bare_out" | grep -q '^PROD_ONLY=' && echo true || echo false)"

# `export prod` merges common + prod; last DB_URL occurrence is prod.
prod_out=$("$DC_SECRETS" export prod)
assert_eq "export prod has common" "true" "$(echo "$prod_out" | grep -q '^COMMON_KEY=common_val' && echo true || echo false)"
assert_eq "export prod has prod-only" "true" "$(echo "$prod_out" | grep -q '^PROD_ONLY=secret' && echo true || echo false)"
last_db=$(echo "$prod_out" | grep '^DB_URL=' | tail -1 | cut -d= -f2-)
assert_eq "export prod last DB_URL wins" "postgres://prod" "$last_db"
assert_eq "export no blank lines" "0" "$(echo "$prod_out" | grep -c '^$')"

echo "--- export: agent overlay after env ---"
"$DC_SECRETS" set shared/common SHARED_KEY=common_val
"$DC_SECRETS" set agents/a3 SHARED_KEY=agent_val
override_out=$("$DC_SECRETS" export common --agent a3)
last_val=$(echo "$override_out" | grep '^SHARED_KEY=' | tail -1 | cut -d= -f2-)
assert_eq "agent overrides common" "agent_val" "$last_val"

echo "--- export: missing/invalid env layer dies loud ---"
# dev.yaml does NOT exist in this store (only common.yaml + prod.yaml), so a
# valid-but-absent env layer must fail-fast — never silently fall to common-only.
assert_fail "missing env layer dies" export dev
assert_fail "unknown env name rejected" export bogus
# Bare export with no shared/common.yaml must die loud (common is mandatory).
# assert_fail inherits the exported DC_SECRETS_DIR, so swap it to a fresh empty
# store for this one check, then restore.
NO_COMMON=$(mktemp -d); EXTRA_DIRS+=("$NO_COMMON")
DC_SECRETS_DIR="$NO_COMMON" "$DC_SECRETS" init >/dev/null 2>&1
SAVED_DIR="$DC_SECRETS_DIR"; export DC_SECRETS_DIR="$NO_COMMON"
assert_fail "bare export w/o common.yaml dies" export
export DC_SECRETS_DIR="$SAVED_DIR"

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

echo "--- portable age key (resolution + export/import) ---"
# age-key export prints the identity (an AGE-SECRET-KEY-1 line)
"$DC_SECRETS" set shared/portable PORTABLE_KEY=portable_val
exported=$("$DC_SECRETS" age-key export)
assert_eq "age-key export is age identity" "true" "$(printf '%s\n' "$exported" | grep -q '^AGE-SECRET-KEY-1' && echo true || echo false)"

# External-key resolution: SOPS_AGE_KEY_FILE points at a COPY of the identity
# (models a host bind-mount into a fresh sandbox with no repo-local .age-identity).
ext_dir=$(mktemp -d); EXTRA_DIRS+=("$ext_dir")
cp "$TEST_DIR/.age-identity" "$ext_dir/age-identity"
got=$(SOPS_AGE_KEY_FILE="$ext_dir/age-identity" "$DC_SECRETS" get shared/portable PORTABLE_KEY 2>/dev/null)
assert_eq "decrypt via external SOPS_AGE_KEY_FILE" "portable_val" "$got"

# Inline-key resolution: SOPS_AGE_KEY is the bare secret line (CI / secret-manager model)
secret_line=$(printf '%s\n' "$exported" | grep '^AGE-SECRET-KEY-1')
got=$(SOPS_AGE_KEY="$secret_line" "$DC_SECRETS" get shared/portable PORTABLE_KEY 2>/dev/null)
assert_eq "decrypt via inline SOPS_AGE_KEY" "portable_val" "$got"

# init adoption guard: when a key is resolvable via SOPS_AGE_KEY_FILE, init must NOT
# generate a competing local identity (it adopts + writes .sops.yaml + returns 0).
adopt_dir=$(mktemp -d); EXTRA_DIRS+=("$adopt_dir")
SOPS_AGE_KEY_FILE="$ext_dir/age-identity" DC_SECRETS_DIR="$adopt_dir" "$DC_SECRETS" init >/dev/null 2>&1
assert_eq "init adopts, no local identity generated" "false" "$([[ -f "$adopt_dir/.age-identity" ]] && echo true || echo false)"
assert_eq "init adoption wrote .sops.yaml" "true" "$([[ -f "$adopt_dir/.sops.yaml" ]] && echo true || echo false)"

# age-key import: seed a FRESH store's identity from a host key file, then prove the
# imported key can decrypt secrets encrypted to the same key (the portability guarantee).
fresh_dir=$(mktemp -d); EXTRA_DIRS+=("$fresh_dir")
DC_SECRETS_DIR="$fresh_dir" "$DC_SECRETS" age-key import --from "$ext_dir/age-identity" >/dev/null 2>&1
assert_eq "age-key import creates identity" "true" "$([[ -f "$fresh_dir/.age-identity" ]] && echo true || echo false)"
mkdir -p "$fresh_dir/shared"
cp "$TEST_DIR/shared/portable.yaml" "$fresh_dir/shared/portable.yaml"
got=$(DC_SECRETS_DIR="$fresh_dir" "$DC_SECRETS" get shared/portable PORTABLE_KEY 2>/dev/null)
assert_eq "imported key decrypts existing secrets" "portable_val" "$got"
# import must refuse to overwrite an existing identity (orphaning risk)
assert_fail "import refuses overwrite" age-key import --from "$ext_dir/age-identity"
# import must reject non-age garbage
junk_dir=$(mktemp -d); EXTRA_DIRS+=("$junk_dir")
printf 'not-a-key\n' > "$junk_dir/bad"
DC_SECRETS_DIR="$junk_dir" assert_fail "import rejects garbage" age-key import --from "$junk_dir/bad"

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
