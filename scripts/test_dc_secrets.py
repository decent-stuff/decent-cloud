"""Black-box tests for ``scripts/dc-secrets`` (the uv inline-script SOPS credential store).

These exercise the REAL ``sops`` + ``age`` binaries against a throwaway
``DC_SECRETS_DIR`` (``tmp_path``) seeded with a freshly ``age-keygen``'d identity,
so the true encrypt/decrypt path is covered. The script is driven as a subprocess
(via its shebang) exactly as callers use it — including ``eval "$(dc-secrets export)"``.

Run from repo root:  ``python3 -m pytest scripts/test_dc_secrets.py -q``
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

DC_SECRETS_BIN = Path(__file__).parent / "dc-secrets"

# The regression value the bash version mangled (stored empty). Must round-trip
# byte-identical through set -> get and set -> export.
GNARLY_URL = "postgres://decent_cloud_prod:a@b'c\"d e://@host:5432/db"
# Covers backslash, ``=`` and double-quote simultaneously.
SPECIAL_CHARS = 'a\\b\\c=d"e:f@g'
MULTILINE = "line1\nline2\nline3"  # no trailing newline (get normalizes trailing \n)


# ─── helpers ───────────────────────────────────────────────────────────────────
def _base_env(dc_dir: Path) -> dict:
    """Inherit PATH/HOME (for sops/age/uv) but scrub ambient key sources."""
    env = {
        k: v
        for k, v in os.environ.items()
        if k not in ("SOPS_AGE_KEY", "SOPS_AGE_KEY_FILE", "DC_SECRETS_DIR")
    }
    env["DC_SECRETS_DIR"] = str(dc_dir)
    return env


def run_dc(args, *, dc_dir, env_extra=None, stdin=None) -> subprocess.CompletedProcess:
    env = _base_env(dc_dir)
    if env_extra:
        env.update(env_extra)
    return subprocess.run(
        [str(DC_SECRETS_BIN), *args],
        env=env,
        input=stdin,
        capture_output=True,
        text=True,
    )


def make_identity(path: Path) -> str:
    """Generate a fresh age identity at ``path`` (mode 600); return the secret line."""
    subprocess.run(["age-keygen", "-o", str(path)], check=True, capture_output=True)
    os.chmod(path, 0o600)
    return Path(path).read_text().strip()


def init_store(dc_dir: Path) -> None:
    """init a fresh store with a generated bootstrap key (no external key source)."""
    assert run_dc(["init"], dc_dir=dc_dir).returncode == 0


def export_value(export_stdout: str, key: str) -> str | None:
    """Return the value for ``key`` from export output (everything after first '=')."""
    prefix = key + "="
    for line in export_stdout.split("\n"):
        if line.startswith(prefix):
            return line[len(prefix):]
    return None


@pytest.fixture(scope="session", autouse=True)
def _warm_uv_cache(tmp_path_factory):
    """Prime uv's script-env cache once so per-test runs are fast and quiet."""
    d = tmp_path_factory.mktemp("dc-warmup")
    res = run_dc(["help"], dc_dir=d)
    assert res.returncode == 0, "dc-secrets help must work (uv + pyyaml install)"


# ─── init ──────────────────────────────────────────────────────────────────────
def test_init_creates_store_with_generated_key(tmp_path):
    store = tmp_path / "store"
    r = run_dc(["init"], dc_dir=store)
    assert r.returncode == 0, r.stderr
    assert (store / ".age-identity").is_file()
    assert (store / ".sops.yaml").is_file()
    for sub in ("shared", "agents", ".locks"):
        assert (store / sub).is_dir()
    assert "Initialized secrets store at" in r.stdout
    assert "Recipient:" in r.stdout
    # .sops.yaml points at the generated recipient.
    cfg = (store / ".sops.yaml").read_text()
    assert "creation_rules:" in cfg
    assert "age:" in cfg


def test_init_adopts_env_key_without_generating(tmp_path):
    store = tmp_path / "store"
    ident = tmp_path / "host-identity"
    make_identity(ident)
    r = run_dc(["init"], dc_dir=store, env_extra={"SOPS_AGE_KEY_FILE": str(ident)})
    assert r.returncode == 0, r.stderr
    assert "Adopted existing age key (env:SOPS_AGE_KEY_FILE)" in r.stdout
    # Adoption must NOT generate a competing local identity.
    assert not (store / ".age-identity").exists()
    assert (store / ".sops.yaml").is_file()


def test_init_idempotent(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    before = (store / ".age-identity").read_text()
    r = run_dc(["init"], dc_dir=store)
    assert r.returncode == 0, r.stderr
    # Second init adopts the now-existing repo identity; key unchanged.
    assert (store / ".age-identity").read_text() == before


# ─── set / get round-trip ─────────────────────────────────────────────────────
def test_set_get_simple(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    assert run_dc(["set", "shared/env", "KEY=simple value"], dc_dir=store).returncode == 0
    r = run_dc(["get", "shared/env", "KEY"], dc_dir=store)
    assert r.stdout == "simple value\n"


def test_set_get_postgres_url(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "DB_URL=postgres://u:p@h:5432/db"], dc_dir=store)
    r = run_dc(["get", "shared/env", "DB_URL"], dc_dir=store)
    assert r.stdout == "postgres://u:p@h:5432/db\n"


def test_roundtrip_gnarly_values(tmp_path):
    """The exact regression the bash version failed: values with ://, @, quotes,
    spaces, backslashes, colons and '=' must round-trip byte-identical."""
    store = tmp_path / "store"
    init_store(store)
    run_dc(
        ["set", "shared/env", f"GNARLY={GNARLY_URL}", f"SPECIAL={SPECIAL_CHARS}"],
        dc_dir=store,
    )
    # get path
    assert run_dc(["get", "shared/env", "GNARLY"], dc_dir=store).stdout == GNARLY_URL + "\n"
    assert run_dc(["get", "shared/env", "SPECIAL"], dc_dir=store).stdout == SPECIAL_CHARS + "\n"
    # export path (unquoted KEY=value, value as-is)
    out = run_dc(["export"], dc_dir=store).stdout
    assert export_value(out, "GNARLY") == GNARLY_URL
    assert export_value(out, "SPECIAL") == SPECIAL_CHARS


def test_roundtrip_multiline_value(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", f"ML={MULTILINE}"], dc_dir=store)
    assert run_dc(["get", "shared/env", "ML"], dc_dir=store).stdout == MULTILINE + "\n"


def test_encrypted_file_is_mode_600_and_keys_visible(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "KEY=v"], dc_dir=store)
    f = store / "shared" / "env.yaml"
    assert f.is_file()
    # keys are visible (plaintext key, encrypted value) — git-committable shape.
    first_line = f.read_text().splitlines()[0]
    assert first_line.startswith("KEY:")
    assert "ENC[" in first_line
    # hard requirement: encrypted file is mode 0600.
    assert (f.stat().st_mode & 0o777) == 0o600


# ─── set: multiple args & overwrite ───────────────────────────────────────────
def test_set_multiple_key_value_args(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "A=1", "B=2", "C=3"], dc_dir=store)
    keys = run_dc(["list", "shared/env"], dc_dir=store).stdout.split()
    assert keys == ["A", "B", "C"]
    for k, v in (("A", "1"), ("B", "2"), ("C", "3")):
        assert run_dc(["get", "shared/env", k], dc_dir=store).stdout == v + "\n"


def test_set_overwrites_without_duplicate(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "A=1", "B=2", "C=3"], dc_dir=store)
    run_dc(["set", "shared/env", "B=updated"], dc_dir=store)
    keys = run_dc(["list", "shared/env"], dc_dir=store).stdout.split()
    assert keys.count("B") == 1            # no duplicate
    assert run_dc(["get", "shared/env", "B"], dc_dir=store).stdout == "updated\n"
    # overwrite moves the key to the end (matches bash remove-then-append).
    assert keys == ["A", "C", "B"]


# ─── delete ───────────────────────────────────────────────────────────────────
def test_delete_removes_one_key(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "A=1", "B=2", "C=3"], dc_dir=store)
    assert run_dc(["delete", "shared/env", "B"], dc_dir=store).returncode == 0
    assert run_dc(["get", "shared/env", "A"], dc_dir=store).stdout == "1\n"
    assert run_dc(["get", "shared/env", "C"], dc_dir=store).stdout == "3\n"
    assert run_dc(["get", "shared/env", "B"], dc_dir=store).returncode == 1


def test_delete_last_key_removes_file(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "ONLY=1"], dc_dir=store)
    f = store / "shared" / "env.yaml"
    assert f.is_file()
    assert run_dc(["delete", "shared/env", "ONLY"], dc_dir=store).returncode == 0
    assert not f.exists()


# ─── export ───────────────────────────────────────────────────────────────────
def test_export_shared_then_agent(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/base", "SHARED=base"], dc_dir=store)
    run_dc(["set", "agents/a1", "AGENT=a1"], dc_dir=store)
    out = run_dc(["export", "--agent", "a1"], dc_dir=store).stdout
    # shared printed first, agent override after.
    assert out.index("SHARED=base") < out.index("AGENT=a1")
    # no blank lines (would break eval).
    assert all(line.strip() for line in out.splitlines())


def test_export_agent_override_wins_as_last(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "K=shared"], dc_dir=store)
    run_dc(["set", "agents/a2", "K=agent"], dc_dir=store)
    out = run_dc(["export", "--agent", "a2"], dc_dir=store).stdout
    # The agent value is the last K= occurrence -> a shell `eval` takes it.
    k_lines = [ln for ln in out.splitlines() if ln.startswith("K=")]
    assert k_lines[-1] == "K=agent"


# ─── list ─────────────────────────────────────────────────────────────────────
def test_list_files_and_keys(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/alpha", "A=1"], dc_dir=store)
    run_dc(["set", "agents/beta", "B=2"], dc_dir=store)
    files = run_dc(["list"], dc_dir=store).stdout.split()
    assert "shared/alpha" in files
    assert "agents/beta" in files
    # .sops.yaml is excluded.
    assert ".sops" not in files
    keys = run_dc(["list", "shared/alpha"], dc_dir=store).stdout.split()
    assert keys == ["A"]


# ─── import ───────────────────────────────────────────────────────────────────
def test_import_env_file(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    env_file = tmp_path / "secrets.env"
    env_file.write_text(
        "# a comment\n"
        "export FOO=bar\n"
        'BAZ="quoted value"\n'
        "\n"
        "QUX=plain\n"
    )
    r = run_dc(["import", str(env_file), "shared/imported"], dc_dir=store)
    assert r.returncode == 0, r.stderr
    # import prints the resolved absolute path + count (so cross-store writes are obvious)
    assert r.stdout.strip().startswith("wrote ")
    assert r.stdout.strip().endswith("(3 keys, imported from " + str(env_file) + ")")
    assert run_dc(["get", "shared/imported", "FOO"], dc_dir=store).stdout == "bar\n"
    assert run_dc(["get", "shared/imported", "BAZ"], dc_dir=store).stdout == "quoted value\n"
    assert run_dc(["get", "shared/imported", "QUX"], dc_dir=store).stdout == "plain\n"


# ─── loud-failure + path-confirmation (no silent data loss) ───────────────────
def test_set_decrypt_failure_is_loud_and_preserves_file(tmp_path):
    """A wrong-key/corrupt store must NOT be silently wiped.

    Regression: decrypt_dict used to return {} on decrypt failure, so ``set``
    would overwrite the file with ONLY the new key, destroying every other
    credential. It must now die loud and leave the file byte-intact.
    """
    store = tmp_path / "store"
    init_store(store)  # real bootstrap key A lands in .age-identity
    assert run_dc(["set", "shared/env", "ORIGINAL=keepme", "OTHER=v2"], dc_dir=store).returncode == 0
    env_file = store / "shared" / "env.yaml"
    assert env_file.is_file()
    # Force a DIFFERENT age key (env takes priority over .age-identity) → sops -d fails.
    key_b = make_identity(tmp_path / "b")
    r = run_dc(["set", "shared/env", "NEWKEY=newval"], dc_dir=store, env_extra={"SOPS_AGE_KEY": key_b})
    assert r.returncode == 1, r.stderr
    assert "failed to decrypt" in r.stderr
    assert "data loss" in r.stderr
    # File must be UNCHANGED: still decryptable with the store's real key, keys intact.
    assert run_dc(["get", "shared/env", "ORIGINAL"], dc_dir=store).stdout == "keepme\n"
    assert run_dc(["get", "shared/env", "OTHER"], dc_dir=store).stdout == "v2\n"
    # NEWKEY must NOT have been written.
    assert run_dc(["get", "shared/env", "NEWKEY"], dc_dir=store).returncode != 0


def test_set_prints_resolved_absolute_path(tmp_path):
    """Mutations print the absolute file path written (cross-store writes are obvious)."""
    store = tmp_path / "store"
    init_store(store)
    r = run_dc(["set", "shared/env", "FOO=bar"], dc_dir=store)
    assert r.returncode == 0, r.stderr
    expected = str((store / "shared" / "env.yaml").resolve())
    assert r.stdout.strip() == f"wrote {expected} (1 keys)"


# ─── age-key resolution + portability ─────────────────────────────────────────
def test_age_key_resolution_priority_inline_wins(tmp_path):
    """SOPS_AGE_KEY (inline) must win over SOPS_AGE_KEY_FILE."""
    store = tmp_path / "store"
    ident_i = tmp_path / "ident-I"
    secret_i = make_identity(ident_i)
    # Bootstrap + encrypt using identity I via SOPS_AGE_KEY_FILE.
    run_dc(["init"], dc_dir=store, env_extra={"SOPS_AGE_KEY_FILE": str(ident_i)})
    run_dc(
        ["set", "shared/env", "K=secret-via-I"],
        dc_dir=store,
        env_extra={"SOPS_AGE_KEY_FILE": str(ident_i)},
    )
    # A second, unrelated identity that CANNOT decrypt the secret.
    ident_ii = tmp_path / "ident-II"
    make_identity(ident_ii)
    # Offer BOTH: inline I + file II. Inline must win -> decrypt succeeds.
    r = run_dc(
        ["get", "shared/env", "K"],
        dc_dir=store,
        env_extra={"SOPS_AGE_KEY": secret_i, "SOPS_AGE_KEY_FILE": str(ident_ii)},
    )
    assert r.returncode == 0, r.stderr
    assert r.stdout == "secret-via-I\n"


def test_age_key_export_then_import_roundtrip(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/portable", "P=portable_val"], dc_dir=store)
    identity = run_dc(["age-key", "export"], dc_dir=store).stdout
    # age-keygen identity files include comment lines (# created / # public key)
    # before the AGE-SECRET-KEY-1 line — export prints the whole file.
    assert any(line.startswith("AGE-SECRET-KEY-1") for line in identity.splitlines())

    # Seed a fresh store from the exported identity, then decrypt the same secret.
    fresh = tmp_path / "fresh"
    r = run_dc(["age-key", "import"], dc_dir=fresh, stdin=identity)
    assert r.returncode == 0, r.stderr
    assert (fresh / ".age-identity").is_file()
    assert "Recipient: age1" in r.stdout
    # Copy the encrypted file over and decrypt with the imported key.
    (fresh / "shared").mkdir(parents=True, exist_ok=True)
    (store / "shared" / "portable.yaml").rename(fresh / "shared" / "portable.yaml")
    assert run_dc(["get", "shared/portable", "P"], dc_dir=fresh).stdout == "portable_val\n"


def test_age_key_import_refuses_overwrite(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    other = tmp_path / "other"
    make_identity(other)
    r = run_dc(["age-key", "import", "--from", str(other)], dc_dir=store)
    assert r.returncode == 1
    assert "already exists" in r.stderr


def test_age_key_import_rejects_garbage(tmp_path):
    store = tmp_path / "store"
    r = run_dc(["age-key", "import"], dc_dir=store, stdin="not-an-age-key\n")
    assert r.returncode == 1
    assert "valid age identity" in r.stderr


# ─── error paths ───────────────────────────────────────────────────────────────
def test_no_args_prints_help_and_exits_1(tmp_path):
    store = tmp_path / "store"
    r = run_dc([], dc_dir=store)
    assert r.returncode == 1
    assert "dc-secrets: credential store" in r.stdout


def test_help_command_exits_0(tmp_path):
    store = tmp_path / "store"
    r = run_dc(["help"], dc_dir=store)
    assert r.returncode == 0
    assert "COMMANDS:" in r.stdout


def test_unknown_command_exits_1(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    r = run_dc(["bogus-command"], dc_dir=store)
    assert r.returncode == 1
    assert "unknown command: bogus-command. Run 'dc-secrets help'" in r.stderr


def test_set_missing_equals_exits_1(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    r = run_dc(["set", "shared/env", "no_equals_here"], dc_dir=store)
    assert r.returncode == 1
    assert "invalid format: no_equals_here (expected key=value)" in r.stderr


def test_get_missing_key_exits_1(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    run_dc(["set", "shared/env", "A=1"], dc_dir=store)
    r = run_dc(["get", "shared/env", "MISSING"], dc_dir=store)
    assert r.returncode == 1
    assert "key not found: MISSING in shared/env" in r.stderr


def test_get_missing_file_exits_1(tmp_path):
    store = tmp_path / "store"
    init_store(store)
    r = run_dc(["get", "shared/nope", "K"], dc_dir=store)
    assert r.returncode == 1
    assert "secret file not found: shared/nope" in r.stderr
