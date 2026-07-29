#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""gen-prod-secret.py — emit the production decent-cloud Kubernetes Secret.

Sources real values from the product repo's AGE-SOPS store (`scripts/dc-secrets
export`), re-packages them under the 37 keys the prod manifests reference, and
writes a plaintext Secret yaml to STDOUT. Pipe the output into PGP-SOPS to land
it in the nuc-k3s cluster repo (the cluster's SOPS key is PGP, the product
repo's own secrets use AGE — different key types).

Usage:
    scripts/gen-prod-secret.py > /tmp/dc.yaml \\
      && sops --encrypt --pgp FA5814CF1935EE80C454C9F1660DCCF069EC9176 /tmp/dc.yaml \\
         > /project/decent-cloud/third_party/nuc-k3s/cluster/secrets/decent-cloud-secret.yaml \\
      && rm /tmp/dc.yaml

The 37 emitted keys are cross-checked against
deploy/k8s/decent-cloud-secret.yaml.template (fail loud on drift).
API_DATABASE_URL is read VERBATIM from the prod secrets layer
(`dc-secrets export prod`); set it to the prod DSN directly:
    scripts/dc-secrets set shared/prod API_DATABASE_URL=postgres://decent_cloud_prod:<pw>@192.168.0.2:5432/decent_cloud_prod
TUNNEL_TOKEN_PROD is reused from the dc-secrets 'TUNNEL_TOKEN' when present
(placeholder otherwise); ensure its tunnel ingress routes to the k8s FQDNs via
'cf/tunnel.py prod'. See deploy/k8s/TUNNEL.md.

Secret values are NEVER written to stderr — only status notes + the loud
TUNNEL_TOKEN_PROD warning are. The Secret body goes to stdout only.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TEMPLATE = REPO_ROOT / "deploy" / "k8s" / "decent-cloud-secret.yaml.template"
DC_SECRETS = REPO_ROOT / "scripts" / "dc-secrets"

# Canonical 37 keys (logical grouping; mirrors the template). The set is
# cross-checked against the template below so the generator cannot drift.
SECRET_KEYS: list[str] = [
    "API_DATABASE_URL",
    "CREDENTIAL_ENCRYPTION_KEY",
    "CF_API_TOKEN",
    "CF_ZONE_ID",
    "TUNNEL_TOKEN_PROD",
    "STRIPE_SECRET_KEY",
    "STRIPE_PUBLISHABLE_KEY",
    "STRIPE_WEBHOOK_SECRET",
    "INVOICE_SELLER_IBAN",
    "GOOGLE_OAUTH_CLIENT_ID",
    "GOOGLE_OAUTH_CLIENT_SECRET",
    "GOOGLE_OAUTH_REDIRECT_URL",
    "FRONTEND_URL",
    "MAILCHANNELS_API_KEY",
    "DKIM_DOMAIN",
    "DKIM_SELECTOR",
    "DKIM_PRIVATE_KEY",
    "CHATWOOT_API_TOKEN",
    "CHATWOOT_PLATFORM_API_TOKEN",
    "CHATWOOT_HMAC_SECRET",
    "CHATWOOT_POSTGRES_PASSWORD",
    "CHATWOOT_SECRET_KEY_BASE",
    "SMTP_ADDRESS",
    "SMTP_USERNAME",
    "SMTP_PASSWORD",
    "OPENAI_API_KEY",
    "DEFAULT_ESCALATION_USER",
    "TELEGRAM_BOT_TOKEN",
    "TEXTBEE_DEVICE_ID",
    "TEXTBEE_API_KEY",
    "TEXTBEE_API_URL",
    "TWILIO_ACCOUNT_SID",
    "TWILIO_AUTH_TOKEN",
    "TWILIO_PHONE_NUMBER",
    "LLM_API_KEY",
    "LLM_API_URL",
    "LLM_API_MODEL",
]

_TEMPLATE_KEY_RE = re.compile(r"^  ([A-Z][A-Z0-9_]+)")
# A template key whose value is an empty quoted scalar (`""`) is OPTIONAL; every
# other key is a `REPLACE_WITH_*` placeholder or a real value and is REQUIRED.
# Authoritative source: deploy/k8s/decent-cloud-secret.yaml.template.
_TEMPLATE_OPTIONAL_RE = re.compile(r'^  [A-Z][A-Z0-9_]+:\s*""')


def die(msg: str) -> None:
    """Print a loud error to stderr and exit 1."""
    print(f"gen-prod-secret: error: {msg}", file=sys.stderr)
    sys.exit(1)


def note(msg: str) -> None:
    """Print a status note to stderr (never the secret body)."""
    print(f"gen-prod-secret: {msg}", file=sys.stderr)


def check_template_drift() -> set[str]:
    """Fail loud if the generator's keys drift from the template.

    Returns the set of OPTIONAL keys (those templated as `""`), so resolve_value
    can fail loud on any OTHER (required) key that is missing from dc-secrets —
    never silently emitting an empty value for a required prod secret.
    """
    if not TEMPLATE.is_file():
        die(f"template not found: {TEMPLATE}")
    template_keys: set[str] = set()
    optional_keys: set[str] = set()
    for line in TEMPLATE.read_text().splitlines():
        m = _TEMPLATE_KEY_RE.match(line)
        if m:
            key = m.group(1)
            template_keys.add(key)
            if _TEMPLATE_OPTIONAL_RE.match(line):
                optional_keys.add(key)
    gen_keys = set(SECRET_KEYS)
    if template_keys != gen_keys:
        only_gen = sorted(gen_keys - template_keys)
        only_tpl = sorted(template_keys - gen_keys)
        details = []
        if only_gen:
            details.append(f"  in generator only: {only_gen}")
        if only_tpl:
            details.append(f"  in template only:  {only_tpl}")
        die("generator keys != template keys (drift detected):\n" + "\n".join(details))
    return optional_keys


def load_dc_secrets() -> dict[str, str]:
    """Run `scripts/dc-secrets export prod` once and parse KEY=VALUE lines into a dict.

    The env layer is fixed to ``prod``: the k8s Secret only ever carries prod
    values, so reading common+prod (and nothing else) is correct and prevents a
    stale dev/play value from leaking into the prod manifest.
    """
    if not DC_SECRETS.exists():
        die(f"'dc-secrets' not found: {DC_SECRETS}")
    try:
        proc = subprocess.run(
            [str(DC_SECRETS), "export", "prod"],
            capture_output=True,
            text=True,
            timeout=60,
            check=True,
        )
    except subprocess.CalledProcessError:
        die(
            "'dc-secrets export prod' failed — is the SOPS age key available?\n"
            "  (resolve via SOPS_AGE_KEY / SOPS_AGE_KEY_FILE, or dc-secrets age-key import)"
        )
    except subprocess.TimeoutExpired:
        die("'dc-secrets export prod' timed out after 60s")

    vals: dict[str, str] = {}
    for line in proc.stdout.splitlines():
        if not line or "=" not in line:
            continue
        k, _, v = line.partition("=")
        vals[k] = v
    return vals


def yaml_escape(value: str) -> str:
    """Escape a value for a YAML double-quoted scalar."""
    return (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )


def resolve_value(key: str, vals: dict[str, str], tunnel_val: str, optional_keys: set[str]) -> str:
    """Return the YAML line for one secret key."""
    if key == "API_DATABASE_URL":
        # Read VERBATIM from the prod layer (last-wins under `export prod`).
        # The prod DSN is stored directly so this generator stays a dumb mapper
        # with no DB-coordinate knowledge.
        value = vals.get("API_DATABASE_URL", "")
        if not value:
            die(
                "API_DATABASE_URL is MISSING in the prod secrets layer — cannot emit "
                "the prod DB connection string.\n"
                "  Set it: scripts/dc-secrets set shared/prod "
                "API_DATABASE_URL=postgres://decent_cloud_prod:<pw>@192.168.0.2:5432/decent_cloud_prod"
            )
    elif key == "TUNNEL_TOKEN_PROD":
        value = tunnel_val
    else:
        # 1:1 map to a dc-secrets key of the same name. A key the template marks
        # optional (`""`) may legitimately be unset; every OTHER key is REQUIRED
        # and must fail loud when missing — never silently emit an empty secret.
        value = vals.get(key, "")
        if not value and key not in optional_keys:
            die(
                f"{key} is MISSING in dc-secrets — this is a REQUIRED prod secret "
                f"(not marked optional in {TEMPLATE.name}).\n"
                f"  Set it: scripts/dc-secrets set shared/prod {key}=<value>\n"
                f"  If it is genuinely optional, mark it so in {TEMPLATE} (value: \"\") and re-run."
            )
    return f'  {key}: "{yaml_escape(value)}"'


def main() -> int:
    optional_keys = check_template_drift()
    vals = load_dc_secrets()

    # Resolve TUNNEL_TOKEN_PROD: reuse the existing prod tunnel token (TUNNEL_TOKEN)
    # from dc-secrets if present; otherwise leave a placeholder the operator fills.
    tunnel_val = vals.get("TUNNEL_TOKEN", "")
    if not tunnel_val:
        tunnel_val = "REPLACE_WITH_TUNNEL_TOKEN_PROD"
        note("WARNING — TUNNEL_TOKEN_PROD is a PLACEHOLDER ('REPLACE_WITH_TUNNEL_TOKEN_PROD').")
        note("  Generate the real prod token with 'python3 cf/tunnel.py prod' (needs CF_API_TOKEN +")
        note("  CF_ACCOUNT_ID) and overwrite the TUNNEL_TOKEN_PROD key in the encrypted secret")
        note("  before applying. See deploy/k8s/TUNNEL.md.")
    else:
        note("TUNNEL_TOKEN_PROD reused from existing dc-secrets 'TUNNEL_TOKEN'.")
        note("  Ensure its tunnel ingress routes to the in-cluster k8s FQDNs — run")
        note("  'python3 cf/tunnel.py prod' once (with CF_API_TOKEN + CF_ACCOUNT_ID) to")
        note("  (re)configure. See deploy/k8s/TUNNEL.md.")

    # Emit the Secret yaml to stdout only.
    out = sys.stdout
    out.write("apiVersion: v1\n")
    out.write("kind: Secret\n")
    out.write("metadata:\n")
    out.write("  name: decent-cloud-secret\n")
    out.write("  namespace: apps\n")
    out.write("type: Opaque\n")
    out.write("stringData:\n")
    for key in SECRET_KEYS:
        out.write(resolve_value(key, vals, tunnel_val, optional_keys) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
