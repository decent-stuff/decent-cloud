"""Tests for scripts/gen-prod-secret.py — loud on missing REQUIRED prod secrets.

The generator maps dc-secrets values onto the 37-key prod Kubernetes Secret.
REQUIRED keys (those the template gives a `REPLACE_WITH_*` placeholder) must
FAIL LOUD when missing — never silently emit an empty value. Only keys the
template marks optional (`""`) may resolve to empty. Regression for the
silent-success class of bug (a missing required key previously became `""`).

Run from repo root:  ``python3 -m pytest scripts/test_gen_prod_secret.py -q``
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "gen-prod-secret.py"


@pytest.fixture(scope="module")
def gps():
    """Load the hyphenated script as a module (can't `import gen-prod-secret`)."""
    spec = importlib.util.spec_from_file_location("gen_prod_secret", SCRIPT)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


# ---------------------------------------------------------------------------
# resolve_value — required vs optional classification (the core fix)
# ---------------------------------------------------------------------------

def test_resolve_value_dies_loud_when_required_key_missing(gps, capsys):
    """A REQUIRED key absent from dc-secrets must exit loud naming the key — never
    silently emit an empty value (the incident class of bug)."""
    # CREDENTIAL_ENCRYPTION_KEY is REPLACE_WITH_* in the template => required.
    with pytest.raises(SystemExit) as exc:
        gps.resolve_value("CREDENTIAL_ENCRYPTION_KEY", {}, "", set())
    assert exc.value.code == 1
    err = capsys.readouterr().err
    assert "CREDENTIAL_ENCRYPTION_KEY" in err
    assert "REQUIRED" in err
    # Actionable remediation hint points at the prod layer.
    assert "scripts/dc-secrets set shared/prod CREDENTIAL_ENCRYPTION_KEY=" in err


def test_resolve_value_optional_key_empty_when_missing(gps, capsys):
    """An optional key (e.g. TWILIO_AUTH_TOKEN) may legitimately be unset — no die,
    emits an empty value, no error noise."""
    line = gps.resolve_value("TWILIO_AUTH_TOKEN", {}, "", {"TWILIO_AUTH_TOKEN"})
    assert line == '  TWILIO_AUTH_TOKEN: ""'
    assert capsys.readouterr().err == ""


def test_resolve_value_required_key_present_emits_value(gps, capsys):
    """A required key that IS present emits its value and stays quiet."""
    line = gps.resolve_value("STRIPE_SECRET_KEY", {"STRIPE_SECRET_KEY": "sk_live_x"}, "", set())
    assert line == '  STRIPE_SECRET_KEY: "sk_live_x"'
    assert capsys.readouterr().err == ""


def test_resolve_value_optional_key_present_emits_value(gps):
    """An optional key that happens to be set emits the real value (not empty)."""
    line = gps.resolve_value("OPENAI_API_KEY", {"OPENAI_API_KEY": "sk-x"}, "", {"OPENAI_API_KEY"})
    assert line == '  OPENAI_API_KEY: "sk-x"'


def test_resolve_value_escapes_special_chars(gps):
    """YAML double-quote escaping still holds on the required-key path."""
    line = gps.resolve_value("SMTP_PASSWORD", {"SMTP_PASSWORD": 'p"ass\\word'}, "", set())
    assert line == '  SMTP_PASSWORD: "p\\"ass\\\\word"'


def test_resolve_value_api_database_url_read_verbatim(gps, capsys):
    """API_DATABASE_URL is read VERBATIM from the prod layer — no DSN building,
    no PROD_POSTGRES_PASSWORD lookup. The generator is a dumb mapper."""
    dsn = "postgres://decent_cloud_prod:secret@192.168.0.2:5432/decent_cloud_prod"
    line = gps.resolve_value("API_DATABASE_URL", {"API_DATABASE_URL": dsn}, "", set())
    assert line == f'  API_DATABASE_URL: "{dsn}"'
    assert capsys.readouterr().err == ""


def test_resolve_value_api_database_url_dies_loud_when_missing(gps, capsys):
    """Missing API_DATABASE_URL in the prod layer dies loud with a remediation
    hint that stores the DSN directly in shared/prod (not built from a password)."""
    with pytest.raises(SystemExit) as exc:
        gps.resolve_value("API_DATABASE_URL", {}, "", set())
    assert exc.value.code == 1
    err = capsys.readouterr().err
    assert "API_DATABASE_URL" in err
    assert "shared/prod" in err
    # The hint must carry the verbatim-DSN shape, NOT a PROD_POSTGRES_PASSWORD build.
    assert "PROD_POSTGRES_PASSWORD" not in err
    assert "decent_cloud_prod" in err


def test_resolve_value_no_db_build_constants_remain(gps):
    """DRY: the prod-DB build constants were removed. None should exist on the module."""
    for attr in ("DB_HOST", "DB_PORT", "DB_NAME", "DB_USER", "DB_PASSWORD_KEY"):
        assert not hasattr(gps, attr), f"stale build constant still defined: gps.{attr}"


# ---------------------------------------------------------------------------
# load_dc_secrets — invokes the prod env layer (no env leakage into prod manifest)
# ---------------------------------------------------------------------------

def test_load_dc_secrets_invokes_export_prod(gps, monkeypatch):
    """load_dc_secrets must call `dc-secrets export prod` (common+prod only) so a
    stale dev/play value can never reach the prod manifest. Regression for the
    flat-store leak."""
    captured: dict = {}

    class _Proc:
        returncode = 0
        stdout = "API_DATABASE_URL=postgres://example/db\n"
        stderr = ""

    def fake_run(argv, **kwargs):
        captured["argv"] = list(argv)
        return _Proc()

    monkeypatch.setattr(gps.subprocess, "run", fake_run)
    vals = gps.load_dc_secrets()
    assert vals == {"API_DATABASE_URL": "postgres://example/db"}
    # The env layer MUST be "prod" — never a bare export.
    assert captured["argv"][1:] == ["export", "prod"]


# ---------------------------------------------------------------------------
# check_template_drift — derives the optional set from the template (drift-proof)
# ---------------------------------------------------------------------------

def test_check_template_drift_returns_optional_keys_from_template(gps):
    """check_template_drift must derive the optional set from the template's `""`
    markers — the authoritative source — so the required/optional split cannot
    drift between generator and template."""
    optional = gps.check_template_drift()
    expected_optional = {
        "INVOICE_SELLER_IBAN",
        "OPENAI_API_KEY",
        "DEFAULT_ESCALATION_USER",
        "TEXTBEE_DEVICE_ID",
        "TEXTBEE_API_KEY",
        "TEXTBEE_API_URL",
        "TWILIO_ACCOUNT_SID",
        "TWILIO_AUTH_TOKEN",
        "TWILIO_PHONE_NUMBER",
    }
    assert optional == expected_optional
    # Sanity: optional must be a subset of the full key set the generator emits.
    assert optional <= set(gps.SECRET_KEYS)
    # Required keys must NOT be classified optional.
    for required in ("CREDENTIAL_ENCRYPTION_KEY", "STRIPE_SECRET_KEY", "CF_API_TOKEN", "API_DATABASE_URL"):
        assert required not in optional
