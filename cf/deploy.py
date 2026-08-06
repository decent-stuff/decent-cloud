#!/usr/bin/env python3
"""Deployment + config tooling for decent-cloud.

DEV runs as a local docker-compose stack managed by this script (deploy/stop/
logs/status/restart). PROD deploys via the k8s cluster (ArgoCD GitOps in the
k8s repo, namespace dc-prod) — those deploy subcommands fail loud on prod.

STAGE (the shared staging env, formerly "dev") deploys via k8s too: namespace
``dc-stage``, ArgoCD-synced from the k8s stage overlay.
``deploy stage`` builds + pushes the api image as the floating ``:stage`` tag
and bumps the stage overlay image (local k8s repo commit — the operator pushes;
ArgoCD then auto-syncs). See ``docs/MIGRATION-CUTOVER.md`` for the full cutover.
The legacy ``dev`` docker-compose path stays intact until the cutover retires it.

`config <env>` (read-only introspection) works for ALL envs: it shows every
config var, its source, and loudly flags any critical var missing/empty. See
cf/CONFIG.md for the authoritative per-var source map + edit/apply recipes.
"""

import os
import subprocess
import sys
import shlex
import hashlib
from pathlib import Path
from typing import Optional
import argparse


def calculate_binary_hash() -> str:
    """Calculate SHA256 hash of API binary for Docker cache invalidation.

    This ensures Docker rebuilds when the binary changes for ANY reason:
    - Migration changes (embedded via sqlx::migrate!)
    - Code changes (bug fixes, features)
    - Dependency updates
    """
    cf_dir = Path(__file__).parent
    binary_path = cf_dir.parent / "target" / "x86_64-unknown-linux-gnu" / "release" / "api-server"

    if not binary_path.exists():
        # The build step should have produced this. If it didn't, the Docker
        # cache key becomes a constant ("no-binary") and stale images may ship —
        # be loud so the operator notices the path/build mismatch instead of
        # silently building from a stale cache.
        print_warning(
            f"API binary not found at {binary_path} — Docker cache key will be 'no-binary' "
            f"(stale-cache risk). Ensure build_rust_binaries_natively() ran first."
        )
        return "no-binary"

    hasher = hashlib.sha256()

    # Hash the binary content
    with open(binary_path, "rb") as f:
        # Read in chunks for memory efficiency (binary can be large)
        for chunk in iter(lambda: f.read(4096), b""):
            hasher.update(chunk)

    return hasher.hexdigest()[:16]  # Short hash for readability


def get_env_config(environment: str) -> tuple[dict[str, str], list[str]]:
    """Get environment-specific configuration for the local docker-compose stack.

    Only ``dev`` is supported here: production deploys via the k8s cluster
    (ArgoCD, namespace dc-prod), not docker-compose. Selecting ``prod`` fails
    loud so no deploy subcommand silently starts a retired compose-prod stack.
    (Read-only `config prod` is handled separately by show_config.)
    """
    cf_dir = Path(__file__).parent

    if environment == "prod":
        print_error(
            "prod is deployed via k8s (ArgoCD, namespace dc-prod); docker-compose is dev-only. "
            "Inspect prod config with `python3 cf/deploy.py config prod`. See cf/CONFIG.md. Aborting."
        )
        sys.exit(1)

    env_vars = {"ENVIRONMENT": "dev", "NETWORK_NAME": "decent-cloud-dev"}
    compose_files = [str(cf_dir / "docker-compose.dev.yml")]

    return env_vars, compose_files


def deploy_environment(environment: str) -> int:
    """Deploy to specified environment."""
    env_vars, compose_files = get_env_config(environment)

    return deploy(environment, env_vars, compose_files)


def stop_environment(environment: str) -> int:
    """Stop services for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"Stopping {environment} services")

    if not run_docker_compose(compose_files, ["down"], env_vars, project_name):
        print_error(f"Failed to stop {environment} services")
        return 1

    print_success(f"{environment.title()} services stopped successfully")
    return 0


def show_logs(environment: str, follow: bool = False, service: Optional[str] = None) -> int:
    """Show logs for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"{environment.title()} logs")

    cmd = ["logs"]
    if follow:
        cmd.append("-f")
    if service:
        cmd.append(service)

    if not run_docker_compose(compose_files, cmd, env_vars, project_name):
        print_error(f"Failed to get logs for {environment}")
        return 1

    return 0


def show_status(environment: str) -> int:
    """Show status for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"{environment.title()} status")

    if not run_docker_compose(compose_files, ["ps"], env_vars, project_name):
        print_error(f"Failed to get status for {environment}")
        return 1

    # Check tunnel status if running
    status = check_tunnel_status(compose_files, environment)
    print()
    if status == "connected":
        print_success("Tunnel connection: Active")
    elif status == "unauthorized":
        print_warning("Tunnel connection: Unauthorized")
    else:
        print_warning("Tunnel connection: Unknown")

    return 0


def restart_environment(environment: str) -> int:
    """Restart services for specified environment."""
    env_vars, compose_files = get_env_config(environment)
    project_name = f"decent-cloud-{environment}"

    print_header(f"Restarting {environment} services")

    if not run_docker_compose(compose_files, ["restart"], env_vars, project_name):
        print_error(f"Failed to restart {environment} services")
        return 1

    print_success(f"{environment.title()} services restarted successfully")
    return 0


# ANSI color codes
RED = "\033[0;31m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
BLUE = "\033[0;34m"
NC = "\033[0m"


def print_header(text: str) -> None:
    """Print a colored header."""
    print(f"{GREEN}{text}{NC}")
    print("=" * len(text))
    print()


def print_success(text: str) -> None:
    """Print success message."""
    print(f"{GREEN}✓{NC} {text}")


def print_error(text: str) -> None:
    """Print error message."""
    print(f"{RED}✗{NC} {text}", file=sys.stderr)


def print_warning(text: str) -> None:
    """Print warning message."""
    print(f"{YELLOW}⚠{NC}  {text}")


def print_info(text: str) -> None:
    """Print info message."""
    print(f"{BLUE}→{NC} {text}")


def setup_telegram_webhook(env_vars: dict[str, str], is_prod: bool) -> bool:
    """Register Telegram bot webhook with Telegram API."""
    import urllib.request
    import json

    token = env_vars.get("TELEGRAM_BOT_TOKEN")
    if not token:
        print_warning("TELEGRAM_BOT_TOKEN not set, skipping webhook registration")
        return True  # Not an error, just not configured

    api_domain = "api.decent-cloud.org" if is_prod else "dev-api.decent-cloud.org"
    webhook_url = f"https://{api_domain}/api/v1/webhooks/telegram"

    try:
        url = f"https://api.telegram.org/bot{token}/setWebhook?url={webhook_url}"
        with urllib.request.urlopen(url, timeout=10) as response:
            result = json.loads(response.read().decode())
            if result.get("ok"):
                print_success(f"Telegram webhook registered: {webhook_url}")
                return True
            else:
                print_error(f"Telegram webhook registration failed: {result.get('description')}")
                return False
    except Exception as e:
        print_error(f"Failed to register Telegram webhook: {e}")
        return False


def check_docker() -> bool:
    """Check if Docker and Docker Compose are installed."""
    try:
        subprocess.run(["docker", "--version"], check=True, capture_output=True)
        subprocess.run(["docker", "compose", "version"], check=True, capture_output=True)
        print_success("Docker and Docker Compose are installed")
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_error("Docker or Docker Compose is not installed")
        print_info("Install Docker: https://docs.docker.com/get-docker/")
        return False


def load_secrets_from_sops(environment: str) -> Optional[dict[str, str]]:
    """Load all secrets from dc-secrets (SOPS-encrypted store) for one env layer.

    ``environment`` selects the secrets layer (``dev``/``prod``) merged over the
    common layer. It MUST match the deploy target so a prod deploy never reads
    dev credentials (or vice versa).
    """
    dc_secrets = Path(__file__).parent.parent / "scripts" / "dc-secrets"
    if not dc_secrets.exists():
        print_error(f"dc-secrets not found at {dc_secrets}")
        return None

    try:
        result = subprocess.run(
            [str(dc_secrets), "export", environment],
            capture_output=True, text=True, check=True,
        )
    except subprocess.CalledProcessError as e:
        print_error(f"dc-secrets export {environment} failed: {e.stderr.strip()}")
        return None

    env_vars: dict[str, str] = {}
    for line in result.stdout.splitlines():
        if not line or "=" not in line:
            continue
        key, _, value = line.partition("=")
        env_vars[key] = value

    if not env_vars:
        print_error("dc-secrets export returned no credentials")
        return None

    return env_vars


# ---------------------------------------------------------------------------
# `config <env>` — read-only config introspection. See cf/CONFIG.md.
# ---------------------------------------------------------------------------

# Critical vars that MUST be non-empty for the env to serve. These are present
# in BOTH dev dc-secrets and the prod stores (ConfigMap+Secret) — no inline
# manifest literals, so no false positives. Missing/empty => loud fail.
CRITICAL_VARS: list[str] = [
    "API_DATABASE_URL", "CREDENTIAL_ENCRYPTION_KEY",
    "CF_API_TOKEN", "CF_ZONE_ID",
    "STRIPE_SECRET_KEY", "STRIPE_PUBLISHABLE_KEY", "STRIPE_WEBHOOK_SECRET",
    "GOOGLE_OAUTH_CLIENT_ID", "GOOGLE_OAUTH_CLIENT_SECRET", "GOOGLE_OAUTH_REDIRECT_URL",
    "FRONTEND_URL",
    "MAILCHANNELS_API_KEY", "DKIM_DOMAIN", "DKIM_SELECTOR", "DKIM_PRIVATE_KEY",
    "CHATWOOT_API_TOKEN", "CHATWOOT_PLATFORM_API_TOKEN", "CHATWOOT_HMAC_SECRET",
    "TELEGRAM_BOT_TOKEN",
    "LLM_API_KEY", "LLM_API_URL", "LLM_API_MODEL",
    "SMTP_ADDRESS", "SMTP_USERNAME", "DEFAULT_ESCALATION_USER",
]

# Non-secret (public identifiers / URLs / model names) — value printed verbatim.
# Everything else is masked to presence+length. Mirrors the prod 12-factor split
# (dc-config ConfigMap = plaintext vs dc-secret Secret).
NON_SECRET_VARS: set[str] = {
    "CF_ZONE_ID", "CF_ACCOUNT_ID", "CF_DOMAIN", "CF_GW_PREFIX",
    "STRIPE_PUBLISHABLE_KEY", "GOOGLE_OAUTH_CLIENT_ID", "GOOGLE_OAUTH_REDIRECT_URL",
    "FRONTEND_URL", "API_PUBLIC_URL", "API_SERVER_PORT",
    "DKIM_DOMAIN", "DKIM_SELECTOR", "DEFAULT_ESCALATION_USER",
    "TEXTBEE_DEVICE_ID", "TEXTBEE_API_URL",
    "LLM_API_URL", "LLM_API_MODEL",
    "SMTP_ADDRESS", "SMTP_USERNAME",
    "CHATWOOT_BASE_URL", "CHATWOOT_FRONTEND_URL", "CHATWOOT_ACCOUNT_ID", "CHATWOOT_INBOX_ID",
    "CANISTER_ID", "TELEGRAM_BOT_USERNAME", "ENVIRONMENT", "RUST_LOG", "LEDGER_DIR",
}


def _render_value(name: str, value: str) -> str:
    """Print non-secret values verbatim; mask secrets to presence+length."""
    if value == "":
        return "EMPTY"
    if name in NON_SECRET_VARS:
        return value
    return f"<set, {len(value)} chars>"


def _print_var_table(title: str, items: dict[str, str]) -> None:
    """Print a sorted KEY → value table."""
    print(f"\n  {title} ({len(items)}):")
    if not items:
        print("    (none)")
        return
    width = max(len(k) for k in items)
    for name in sorted(items):
        print(f"    {name.ljust(width)}  {_render_value(name, items[name])}")


def _read_cluster_stores(
    namespace: str, configmap: str, secret: str
) -> tuple[dict[str, str], dict[str, str]]:
    """Read live config from the cluster via kubectl for one namespace.

    Returns (configmap_vars, secret_vars). Raises RuntimeError with an
    actionable message if kubectl is missing or the resources can't be read.
    Used by ``config prod`` (dc-prod) and ``config stage`` (dc-stage).
    """
    import base64
    import json

    def _kubectl(args: list[str]) -> dict:
        try:
            r = subprocess.run(["kubectl", "-n", namespace, *args],
                               capture_output=True, text=True, check=True)
        except FileNotFoundError as e:
            raise RuntimeError("kubectl not found on PATH") from e
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"kubectl failed: {(e.stderr or e.stdout).strip()}") from e
        try:
            return json.loads(r.stdout)
        except json.JSONDecodeError as e:
            raise RuntimeError(f"kubectl returned non-JSON: {r.stdout[:200]!r}") from e

    cm = _kubectl(["get", "configmap", configmap, "-o", "json"])
    cm_vars = dict(cm.get("data") or {})

    sec = _kubectl(["get", "secret", secret, "-o", "json"])
    sec_raw = dict(sec.get("data") or {})
    # Secret data values are base64-encoded; decode to measure length (never printed).
    sec_vars: dict[str, str] = {}
    for k, v in sec_raw.items():
        try:
            sec_vars[k] = base64.b64decode(v).decode("utf-8", errors="replace")
        except Exception:
            sec_vars[k] = ""
    return cm_vars, sec_vars


def show_config(environment: str) -> int:
    """Print every config var for `environment` with its source + current value,
    and loudly flag any critical var that is missing/empty. Read-only.

    dev   → dc-secrets (common+dev merged, AGE-SOPS) consumed by docker-compose.
    prod  → live cluster (kubectl: dc-config ConfigMap + dc-secret Secret).
    stage → live cluster (kubectl: dc-stage-config ConfigMap + dc-stage-secret Secret).
    See cf/CONFIG.md for the authoritative per-var source map + edit/apply recipes.
    """
    print_header(f"Configuration audit — {environment}")
    print_info("Where each var lives + how to change it: cf/CONFIG.md")

    if environment == "dev":
        store = load_secrets_from_sops("dev")
        if store is None:
            return 1
        _print_var_table("dc-secrets (common+dev)", store)
        all_vars = dict(store)
        print(f"\n  source: dc-secrets (repo/secrets/shared/{{common,dev}}.yaml)")
    else:
        # prod + stage both read live k8s stores (namespace + names differ).
        if environment == "stage":
            ns, cm_name, sec_name = "dc-stage", "dc-stage-config", "dc-stage-secret"
            overlay = "cluster/apps/decent-cloud/stage/"
        else:
            ns, cm_name, sec_name = "dc-prod", "dc-config", "dc-secret"
            overlay = "cluster/apps/decent-cloud/"
        try:
            cm_vars, sec_vars = _read_cluster_stores(ns, cm_name, sec_name)
        except RuntimeError as e:
            print_error(f"Could not read live {environment} config: {e}")
            print_info(f"{environment} config lives in the k8s repo (third_party/k8s):")
            print_info(f"  {overlay}  (kustomize overlay → ConfigMap, non-secret)")
            print_info(f"  cluster/secrets/{sec_name}.yaml   (SOPS Secret)")
            print_info("Edit + apply via `sops` + `kubectl` — see cf/CONFIG.md.")
            return 1
        _print_var_table(f"{cm_name} ConfigMap (non-secret)", cm_vars)
        _print_var_table(f"{sec_name} Secret", sec_vars)
        all_vars = {**cm_vars, **sec_vars}
        print(f"\n  source: live cluster (kubectl -n {ns} get cm/{cm_name} secret/{sec_name})")

    missing = [v for v in CRITICAL_VARS if not all_vars.get(v)]
    if missing:
        print_error(f"{len(missing)} critical var(s) missing/empty:")
        for v in missing:
            print(f"      - {v}")
        print_info("Set these before deploying — see cf/CONFIG.md for the per-var source.")
        return 1
    print_success(f"all {len(CRITICAL_VARS)} critical vars present")
    return 0


def run_docker_compose(
    compose_files: list[str], command: list[str], env_vars: dict[str, str], project_name: Optional[str] = None
) -> bool:
    """Run docker compose with specified files and environment."""
    cmd = ["docker", "compose"]
    if project_name:
        cmd.extend(["-p", project_name])
    for file in compose_files:
        cmd.extend(["-f", file])
    cmd.extend(command)

    try:
        print_info(f"$ {' '.join(shlex.quote(arg) for arg in cmd)}")
        subprocess.run(cmd, check=True, env={**os.environ, **env_vars})
        return True
    except subprocess.CalledProcessError:
        return False


def check_tunnel_status(compose_files: list[str], env_name: str) -> str:
    """Check tunnel connection status from logs."""
    try:
        project_name = f"decent-cloud-{env_name[:4]}"  # dev -> dev, prod -> prod

        cmd = ["docker", "compose", "-p", project_name]
        for f in compose_files:
            cmd.extend(["-f", f])
        cmd.extend(["logs", "cloudflared"])

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            env=os.environ,
            check=False,
        )
        logs = result.stdout + result.stderr

        if "Registered tunnel connection connIndex=" in logs:
            return "connected"
        elif "Unauthorized" in logs:
            return "unauthorized"
        else:
            return "unclear"
    except Exception as e:
        # Don't swallow the cause: surface WHY the status check blew up (docker
        # missing, compose file unreadable, etc.) so the operator can fix it.
        print_error(f"Tunnel status check failed: {e!r}")
        return "error"


def check_prerequisites() -> bool:
    """Check if required build tools are installed."""
    print_header("Checking prerequisites")

    # Check Rust toolchain
    try:
        subprocess.run(["rustc", "--version"], check=True, capture_output=True)
        print_success("Rust toolchain found")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_error("Rust not found. Please install Rust: https://rustup.rs/")
        return False

    # Check Node.js
    try:
        subprocess.run(["node", "--version"], check=True, capture_output=True)
        print_success("Node.js found")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_error("Node.js not found. Please install Node.js: https://nodejs.org/")
        return False

    # Check Rust target for cross-compilation
    try:
        result = subprocess.run(["rustup", "target", "list"], check=True, capture_output=True, text=True)
        if "x86_64-unknown-linux-musl" not in result.stdout:
            print_info("Adding Rust target for cross-compilation...")
            subprocess.run(["rustup", "target", "add", "x86_64-unknown-linux-musl"], check=True)
            print_success("Added x86_64-unknown-linux-musl target")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print_warning("rustup not found. Cross-compilation may fail")

    return True


def build_rust_binaries_natively() -> bool:
    """Build API server binary natively before Docker build."""
    cf_dir = Path(__file__).parent
    api_dir = cf_dir.parent / "api"

    print_header("Building API server natively")

    if not api_dir.exists():
        print_error("API directory not found")
        return False

    try:
        # Change to project root and build API binary
        project_root = cf_dir.parent
        os.chdir(project_root)

        # Build for linux/amd64 target (required for Docker)
        # SQLX_OFFLINE=true uses pre-prepared .sqlx queries instead of live DB
        build_env = {**os.environ, "SQLX_OFFLINE": "true"}
        subprocess.run(
            [
                "cargo",
                "build",
                "--release",
                "--bin",
                "api-server",
                "--bin",
                "dc",
                "--target",
                "x86_64-unknown-linux-gnu",
            ],
            check=True,
            env=build_env,
        )

        # Verify binary was created
        binary_path = project_root / "target" / "x86_64-unknown-linux-gnu" / "release" / "api-server"
        if not binary_path.exists():
            print_error(f"API binary not found at {binary_path}")
            return False

        print_success(f"API server built successfully: {binary_path}")
        return True
    except subprocess.CalledProcessError as e:
        print_error(f"API build failed: {e}")
        print_error(f"stdout: {e.stdout}")
        print_error(f"stderr: {e.stderr}")
        return False
    except FileNotFoundError:
        print_error("Cargo not found. Please install Rust")
        return False
    except Exception as e:
        print_error(f"Unexpected error during API build: {e}")
        return False


def build_website_natively(environment: str, env_vars: dict[str, str]) -> bool:
    """Build SvelteKit website natively before Docker build.

    Args:
        environment: 'dev' or 'prod' - determines API endpoint configuration
        env_vars: Environment variables from dc-secrets (contains Stripe keys)
    """
    cf_dir = Path(__file__).parent
    project_root = cf_dir.parent
    website_dir = project_root / "website"

    print_header(f"Building SvelteKit website for {environment}")

    if not website_dir.exists():
        print_error("Website directory not found")
        return False

    try:
        # Configure API endpoint and Stripe keys based on environment
        env_local_file = website_dir / ".env.local"

        # Check for Stripe publishable key
        stripe_key = env_vars.get("STRIPE_PUBLISHABLE_KEY")
        if not stripe_key:
            print_warning("STRIPE_PUBLISHABLE_KEY not found in environment config")
            print_warning("Credit card payments will NOT work without this key")
            print_info(f"Add it: scripts/dc-secrets set shared/{environment} STRIPE_PUBLISHABLE_KEY=pk_test_...")
            print_info("Use pk_test_... for dev, pk_live_... for prod")
            print()
            # Don't fail - allow deployment without Stripe (DCT payments still work)

        with open(env_local_file, "w") as f:
            f.write("# Auto-generated by deploy.py - DO NOT EDIT\n")
            f.write(f"# Environment: {environment}\n")
            f.write("\n")

            if environment == "dev":
                f.write("# Development/staging API endpoint\n")
                f.write("VITE_DECENT_CLOUD_API_URL=https://dev-api.decent-cloud.org\n")
            else:  # prod
                f.write("# Production API endpoint (uses default from .env)\n")
                f.write("VITE_DECENT_CLOUD_API_URL=https://api.decent-cloud.org\n")

            f.write("\n")

            if stripe_key:
                f.write("# Stripe publishable key (safe to embed in client-side code)\n")
                key_type = "TEST" if stripe_key.startswith("pk_test_") else "LIVE"
                f.write(f"# Key type: {key_type}\n")
                f.write(f"VITE_STRIPE_PUBLISHABLE_KEY={stripe_key}\n")
                print_success(f"Configured Stripe {key_type} key for website build")
            else:
                f.write("# Stripe not configured - credit card payments disabled\n")
                f.write("# VITE_STRIPE_PUBLISHABLE_KEY=pk_test_...\n")

            # Chatwoot widget configuration. Require BOTH the token and the
            # base URL: the website widget only renders when both are present
            # (see ChatwootWidget.svelte). Never default to a hardcoded host —
            # a dead/misconfigured host causes 404 + X-Frame-Options console
            # errors on every page.
            f.write("\n")
            chatwoot_token = env_vars.get("CHATWOOT_WEBSITE_TOKEN")
            chatwoot_base_url = env_vars.get("CHATWOOT_BASE_URL")
            if chatwoot_token and chatwoot_base_url:
                f.write("# Chatwoot support widget\n")
                f.write(f"VITE_CHATWOOT_WEBSITE_TOKEN={chatwoot_token}\n")
                f.write(f"VITE_CHATWOOT_BASE_URL={chatwoot_base_url}\n")
                print_success("Configured Chatwoot widget for website build")
            elif chatwoot_token and not chatwoot_base_url:
                # Token set but no base URL: emit the token but leave the URL
                # commented so the widget stays gated OFF (no dead-host fetch).
                f.write("# Chatwoot token present but CHATWOOT_BASE_URL missing — widget DISABLED\n")
                f.write(f"VITE_CHATWOOT_WEBSITE_TOKEN={chatwoot_token}\n")
                f.write("# VITE_CHATWOOT_BASE_URL=https://your-chatwoot.example.org\n")
                print_warning("CHATWOOT_WEBSITE_TOKEN is set but CHATWOOT_BASE_URL is missing")
                print_warning("Support widget will NOT render until CHATWOOT_BASE_URL is set")
            else:
                f.write("# Chatwoot not configured — support widget disabled\n")
                f.write("# VITE_CHATWOOT_WEBSITE_TOKEN=your_token\n")
                f.write("# VITE_CHATWOOT_BASE_URL=https://your-chatwoot.example.org\n")

            # Telegram bot username for UI display
            f.write("\n")
            telegram_bot_username = env_vars.get("TELEGRAM_BOT_USERNAME", "DecentCloudBot")
            print_info(f"Telegram bot username: {telegram_bot_username}")
            f.write(f"VITE_TELEGRAM_BOT_USERNAME={telegram_bot_username}\n")

        print_success(f"Created .env.local for {environment} build")
        print()

        # Change to website directory and run build
        os.chdir(website_dir)
        subprocess.run(["npm", "run", "build"], check=True)
        print_success("Website built successfully")

        # Verify build output exists
        build_dir = website_dir / "build"
        if not build_dir.exists():
            print_error(f"Build directory not found at {build_dir}")
            return False

        print_success(f"Build output verified at {build_dir}")
        return True
    except subprocess.CalledProcessError as e:
        print_error(f"Website build failed: {e}")
        return False
    except FileNotFoundError:
        print_error("Node.js not found. Please install Node.js")
        return False
    except Exception as e:
        print_error(f"Unexpected error during website build: {e}")
        return False


def deploy(env_name: str, env_vars: dict[str, str], compose_files: list[str]) -> int:
    """Shared deployment logic for dev and prod environments."""
    is_prod = env_name == "prod"

    # Header
    print_header(f"Decent Cloud - {env_name.title()} Deployment")

    # Check Docker
    if not check_docker():
        return 1
    print()

    # Load all secrets from dc-secrets (SOPS-encrypted store) for THIS env layer.
    secrets = load_secrets_from_sops(env_name)
    if not secrets:
        print_error("Failed to load secrets from dc-secrets. Run: scripts/dc-secrets init")
        return 1

    print_success(f"Loaded {len(secrets)} credentials from dc-secrets")
    print()

    # Merge secrets into env_vars (secrets take precedence)
    env_vars.update(secrets)

    # Verify tunnel token exists
    if not env_vars.get("TUNNEL_TOKEN"):
        if is_prod:
            print_error("TUNNEL_TOKEN not found in dc-secrets")
            print()
            print(f"Add it: scripts/dc-secrets set shared/{env_name} TUNNEL_TOKEN=<token>")
            print("Get token from: https://one.dash.cloudflare.com/")
            print()
            return 1
        else:
            print_warning("TUNNEL_TOKEN not found - public access will not work")
            print_info(f"Add it: scripts/dc-secrets set shared/{env_name} TUNNEL_TOKEN=<token>")
            print()
    else:
        print_success("Tunnel token loaded")
        print()

    # Log loaded OAuth config (without showing secrets)
    if env_vars.get("GOOGLE_OAUTH_CLIENT_ID"):
        print_success("Google OAuth credentials loaded")
        print_info(f"  Redirect URL: {env_vars.get('GOOGLE_OAUTH_REDIRECT_URL', 'not set')}")
        print_info(f"  Frontend URL: {env_vars.get('FRONTEND_URL', 'not set')}")
        print()
    else:
        print_warning("Google OAuth not configured (optional)")
        print()

    # Build website natively first with environment-specific API configuration and Stripe keys
    if not build_website_natively(env_name, env_vars):
        print_error("Failed to build website")
        return 1
    print()

    # Build API server natively first
    if not build_rust_binaries_natively():
        print_error("Failed to build API server")
        return 1
    print()

    # Calculate binary hash for Docker cache invalidation
    binary_hash = calculate_binary_hash()
    env_vars["BINARY_HASH"] = binary_hash
    print_info(f"API binary hash: {binary_hash}")
    print()

    # Build and start services
    action = "production services" if is_prod else "services"
    print_warning(f"Building and starting {action}...")
    print()

    # Use a specific project name to isolate dev and prod environments
    project_name = f"decent-cloud-{env_name}"  # dev -> dev, prod -> prod

    if not run_docker_compose(compose_files, ["up", "-d", "--build", "--remove-orphans"], env_vars, project_name):
        print()
        print_error(f"{env_name.title()} deployment failed")
        print()
        compose_args = " ".join(f"-f {f}" for f in compose_files)
        project_args = f"-p {project_name}"
        print(f"Check logs: {BLUE}docker compose {project_args} {compose_args} logs{NC}")
        print()
        return 1

    # Success message
    print()
    print(f"{GREEN}========================================")
    if is_prod:
        print("Production Deployment Complete!")
    else:
        print("Development Deployment Complete!")
    print(f"========================================{NC}")
    print()
    print("Services started:")
    if is_prod:
        print("  • Decent Cloud Website (production)")
        print("  • Cloudflare Tunnel (api.decent-cloud.org)")
    else:
        print("  • Decent Cloud Website (development)")
        print("  • Cloudflare Tunnel (dev-api.decent-cloud.org)")
    print()

    # Check tunnel connection (both dev and prod now use tunnels)
    if env_vars.get("TUNNEL_TOKEN"):
        print_warning("Verifying tunnel connection...")
        import time

        time.sleep(5)

        status = check_tunnel_status(compose_files, env_name)

        if status == "connected":
            print_success("Tunnel connected successfully!")
            print()
            if is_prod:
                print("Your website is live at: https://decent-cloud.org")
                print("API available at: https://api.decent-cloud.org")
            else:
                print("Your website is live at: https://dev.decent-cloud.org")
                print("API available at: https://dev-api.decent-cloud.org")
            print()
            print("Verify deployment:")
            print("  • Check tunnel status in Cloudflare dashboard")
            domain = "decent-cloud.org" if is_prod else "dev.decent-cloud.org"
            print(f"  • Test your domain: https://{domain}/health")
            print()

            # Register Telegram webhook now that API is accessible
            setup_telegram_webhook(env_vars, is_prod)
        elif status == "unauthorized":
            print_error("Tunnel authentication failed!")
            print()
            print("Possible causes:")
            print("  1. Tunnel doesn't exist in Cloudflare dashboard")
            print("  2. Token is invalid or expired")
            print()
            print(f"Fix: {BLUE}python3 cf/tunnel.py dev{NC}")
            print()
            if is_prod:
                return 1
        else:
            msg = "Could not verify tunnel status" if status == "error" else "Tunnel status unclear"
            print_warning(msg)
            compose_args = " ".join(f"-f {f}" for f in compose_files)
            project_args = f"-p {project_name}"
            print(f"  {BLUE}docker compose {project_args} {compose_args} logs cloudflared{NC}")
            print()

    # Management commands
    print("Useful commands:" if not is_prod else "Management commands:")
    compose_args = " ".join(f"-f {f}" for f in compose_files)
    project_args = f"-p {project_name}"
    if is_prod:
        print(f"  View logs:    {BLUE}docker compose {project_args} {compose_args} logs -f{NC}")
        print(f"  Check status: {BLUE}docker compose {project_args} {compose_args} ps{NC}")
        print(f"  Restart:      {BLUE}docker compose {project_args} {compose_args} restart{NC}")
        print(f"  Stop:         {BLUE}docker compose {project_args} {compose_args} down{NC}")
    else:
        print(f"  {BLUE}docker compose {project_args} {compose_args} logs -f{NC}")
        print(f"  {BLUE}docker compose {project_args} {compose_args} ps{NC}")
        print(f"  {BLUE}docker compose {project_args} {compose_args} down{NC}")
    print()

    return 0


# ---------------------------------------------------------------------------
# `deploy stage` — k8s/ArgoCD (namespace dc-stage), NOT docker-compose.
# Post-cutover flow (see docs/MIGRATION-CUTOVER.md): build the api image, push it
# as the floating :stage tag, bump the stage overlay image in the k8s repo
# (local commit — operator pushes; ArgoCD then auto-syncs dc-stage). The legacy
# `dev` docker-compose path stays intact until the cutover runbook retires it.
# ---------------------------------------------------------------------------

# Forgejo registry (git.kalaj.org, owner decent-stuff) — same registry prod uses.
# The api-serve + api-sync Deployments share this one image.
STAGE_API_IMAGE = "git.kalaj.org/decent-stuff/decent-cloud-api"
# Floating tag this command (and CI) push on every stage update. Until :stage
# ships, the stage overlay pins prod's tag — see docs/MIGRATION-CUTOVER.md §C.
STAGE_DEFAULT_TAG = "stage"


def _nuc_k3s_dir() -> Path:
    """Resolve the k8s repo checkout (the GitOps source for dc-prod/dc-stage).

    Default: ``<outer-workspace>/third_party/k8s`` (``repo/`` is a submodule of
    the outer workspace, so ``cf/../..`` is the outer workspace). Override with
    ``NUC_K3S_DIR`` for non-standard layouts. Returns the resolved path; does
    NOT verify it exists (callers check the specific file they need).
    """
    override = os.environ.get("NUC_K3S_DIR")
    if override:
        return Path(override).expanduser().resolve()
    cf_dir = Path(__file__).resolve().parent
    return (cf_dir.parent.parent / "third_party" / "k8s").resolve()


def _update_stage_image_tag(kustomization_path: Path, new_tag: str) -> bool:
    """Set the dc-stage api image tag in the stage kustomization overlay.

    Edits the ``images:`` block in-place, preserving the rest of the file
    byte-for-byte (operators read/diff this manifest). Updates the
    ``decent-cloud-api`` target's ``newTag`` (inserts one if absent).

    Returns True if the file changed, False if it already pinned ``new_tag``.
    Raises RuntimeError with context if the file, the ``images:`` section, or
    the api target is missing — never silently no-ops (a missing target means
    the overlay drifted from the expected shape and the bump would be a lie).
    """
    if not kustomization_path.exists():
        raise RuntimeError(
            f"stage overlay not found at {kustomization_path}. Track 1 "
            f"(k8s base/prod/stage manifests) must land first — see "
            f"docs/MIGRATION-CUTOVER.md § Prerequisites."
        )

    lines = kustomization_path.read_text().splitlines(keepends=True)

    # Locate the top-level `images:` mapping key (column 0).
    images_idx: Optional[int] = None
    for i, ln in enumerate(lines):
        if ln.startswith("images:"):
            images_idx = i
            break
    if images_idx is None:
        raise RuntimeError(
            f"{kustomization_path} has no top-level `images:` field — the stage "
            f"overlay cannot set an image tag. Reconcile with Track 1's overlay."
        )

    # The images: block runs until the next top-level mapping key. List items
    # (even at column 0) and comments stay part of the block.
    block_end = len(lines)
    for i in range(images_idx + 1, len(lines)):
        s = lines[i]
        if not s.strip():
            continue
        if s[0].isspace():
            continue  # indented continuation
        first = s.lstrip()
        if first.startswith("#") or first.startswith("-"):
            continue  # comment or list item belongs to images:
        block_end = i  # next top-level key
        break

    # Find the api image target entry, then its existing newTag (if any).
    target = "decent-cloud-api"  # substring of the api image; NOT the website image
    target_start: Optional[int] = None
    target_newtag: Optional[int] = None
    newtag_indent = "    "
    i = images_idx + 1
    while i < block_end:
        stripped = lines[i].strip()
        if stripped.startswith("- name:") and target in stripped:
            target_start = i
            newtag_indent = " " * (lines[i].index("-") + 2)
            j = i + 1
            while j < block_end:
                s = lines[j].strip()
                if s.startswith("- name:"):
                    break  # next entry began without a newTag
                if s.startswith("newTag:"):
                    target_newtag = j
                    break
                j += 1
            break
        i += 1

    if target_start is None:
        raise RuntimeError(
            f"{kustomization_path}: no `images:` entry whose name contains "
            f"'{target}'. Expected `git.kalaj.org/decent-stuff/decent-cloud-api`."
        )

    if target_newtag is not None:
        current = lines[target_newtag].split("newTag:", 1)[1].strip()
        if current == new_tag:
            return False  # already pinned — idempotent no-op
        lines[target_newtag] = f"{newtag_indent}newTag: {new_tag}\n"
    else:
        lines.insert(target_start + 1, f"{newtag_indent}newTag: {new_tag}\n")

    kustomization_path.write_text("".join(lines))
    return True


def deploy_stage(tag: str) -> int:
    """Build + push the dc-stage api image, then bump the stage overlay (k8s repo).

    Stage deploys via k8s (ArgoCD, namespace dc-stage), NOT docker-compose. This
    command is the manual ship-image flow (CI's ``:stage`` build is the automated
    equivalent — see docs/MIGRATION-CUTOVER.md § Step C). Steps:

      1. Build the api binary natively (reuses the dev build path).
      2. ``docker build`` the api image from ``api/Dockerfile``.
      3. Tag + push it as ``<STAGE_API_IMAGE>:<tag>`` (default ``:stage``).
      4. Bump the ``images:`` entry in the k8s stage overlay to ``<tag>``.
      5. Commit the k8s repo change LOCALLY (we cannot push the k8s repo from here);
         print the exact ``git push`` the operator must run so ArgoCD auto-syncs.

    Returns 0 on success, 1 on any failure (each step fails loud with context).
    """
    print_header(f"Decent Cloud — stage deploy (tag: {tag})")
    print_info("stage runs on k8s (namespace dc-stage, ArgoCD-synced from the k8s repo)")
    print_info("this ships the api image + bumps the overlay; see docs/MIGRATION-CUTOVER.md")
    print()

    # 0. Docker must be present (build + push).
    if not check_docker():
        return 1
    print()

    # 1. Build the api binary natively (also yields the hash for cache-busting).
    if not build_rust_binaries_natively():
        print_error("API binary build failed — cannot proceed")
        return 1
    binary_hash = calculate_binary_hash()
    print_info(f"API binary hash: {binary_hash}")
    print()

    project_root = Path(__file__).resolve().parent.parent
    full_image = f"{STAGE_API_IMAGE}:{tag}"

    # 2-3. docker build directly to the registry tag (BINARY_HASH busts the
    # layer cache when the binary changes for any reason).
    print_header(f"Building + tagging {full_image}")
    build_cmd = [
        "docker", "build",
        "-f", "api/Dockerfile",
        "-t", full_image,
        "--build-arg", f"USER_ID={os.getuid()}",
        "--build-arg", f"GROUP_ID={os.getgid()}",
        "--build-arg", f"BINARY_HASH={binary_hash}",
        ".",
    ]
    print_info(f"$ {' '.join(shlex.quote(c) for c in build_cmd)}")
    try:
        subprocess.run(build_cmd, check=True, cwd=project_root)
    except subprocess.CalledProcessError as e:
        print_error(f"docker build failed (rc={e.returncode})")
        return 1
    except FileNotFoundError:
        print_error("docker not found on PATH")
        return 1
    print_success(f"Built {full_image}")
    print()

    # 3b. push to the Forgejo registry.
    print_header(f"Pushing {full_image}")
    push_cmd = ["docker", "push", full_image]
    print_info(f"$ {' '.join(shlex.quote(c) for c in push_cmd)}")
    try:
        subprocess.run(push_cmd, check=True)
    except subprocess.CalledProcessError as e:
        print_error(f"docker push failed (rc={e.returncode})")
        print_info("If 401/unauthorized: run `docker login git.kalaj.org` (operator token).")
        return 1
    except FileNotFoundError:
        print_error("docker not found on PATH")
        return 1
    print_success(f"Pushed {full_image}")
    print()

    # 4. bump the k8s stage overlay image tag.
    nuc_k3s = _nuc_k3s_dir()
    overlay = nuc_k3s / "cluster" / "apps" / "decent-cloud" / "stage" / "kustomization.yaml"
    print_header(f"Bumping stage overlay: {overlay}")
    try:
        changed = _update_stage_image_tag(overlay, tag)
    except RuntimeError as e:
        print_error(f"Could not bump stage overlay: {e}")
        print_info("Track 1 (k8s stage manifests) must land first — see docs/MIGRATION-CUTOVER.md")
        return 1
    if changed:
        print_success(f"Stage overlay api image → :{tag}")
    else:
        print_success(f"Stage overlay already pins :{tag} (no manifest change)")
    print()

    # 5. commit the k8s repo LOCALLY (operator pushes; we cannot — see plan APPENDIX A).
    rel_overlay = "cluster/apps/decent-cloud/stage/kustomization.yaml"
    if changed:
        print_header("Committing k8s repo change (local only)")
        try:
            subprocess.run(["git", "-C", str(nuc_k3s), "add", rel_overlay], check=True)
            subprocess.run(
                ["git", "-C", str(nuc_k3s), "commit", "-m",
                 f"deploy(stage): bump dc-stage api image to {tag}"],
                check=True, capture_output=True, text=True,
            )
        except subprocess.CalledProcessError as e:
            err = (e.stderr or e.stdout or str(e)).strip()
            print_error(f"git commit in the k8s repo failed: {err}")
            print_info(f"The overlay edit is on disk at {overlay} — commit + push it manually.")
            return 1
        print_success("Committed locally in the k8s repo")
        print()
    else:
        print_info("No k8s repo commit needed (overlay unchanged)")

    # Operator handoff — the one step this command cannot do (k8s repo push).
    print(f"{GREEN}========================================")
    print(f"Stage image shipped ({full_image})")
    print(f"========================================{NC}")
    print()
    if changed:
        print("The k8s repo change is committed LOCALLY. Push it so ArgoCD auto-syncs dc-stage:")
        print(f"  {BLUE}cd {nuc_k3s} && git push origin main{NC}")
        print()
    print("Force an ArgoCD refresh + roll dc-stage so it picks up the new image:")
    print(f"  {BLUE}kubectl -n argocd patch application decent-cloud-stage \\{NC}")
    print(f"    {BLUE}--type=merge -p '{{\"metadata\":{{\"annotations\":{{\"argocd.argoproj.io/refresh\":\"normal\"}}}}}}'{NC}")
    print(f"  {BLUE}kubectl -n dc-stage rollout restart deploy/dc-api{NC}")
    print()
    print("Verify stage health:")
    print(f"  {BLUE}kubectl -n dc-stage get pods -l app=dc-api{NC}")
    print(f"  {BLUE}curl -fsS https://api.stage.decent-cloud.org/api/v1/health{NC}")
    print()
    return 0


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Deploy and manage Decent Cloud environments",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s deploy dev                 # Deploy the local dev stack (docker-compose)
  %(prog)s deploy stage               # Ship the stage api image + bump k8s overlay (k8s)
  %(prog)s deploy stage --tag <sha>   # Ship a pinned stage image instead of the floating :stage
  %(prog)s stop dev                  # Stop dev services
  %(prog)s logs dev -f website       # Follow dev website logs
  %(prog)s status dev                # Show dev status
  %(prog)s restart dev               # Restart dev services
  %(prog)s config dev                # Audit dev config vars + sources (read-only)
  %(prog)s config prod               # Audit prod config (reads live cluster)
  %(prog)s config stage              # Audit stage config (reads live dc-stage cluster)

Prod + stage deploy via k8s (ArgoCD, namespaces dc-prod / dc-stage); see cf/CONFIG.md
and docs/MIGRATION-CUTOVER.md. `deploy dev` is the legacy local docker-compose stack,
retained until the stage cutover retires it.
        """,
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # Deploy command. `dev` = local docker-compose; `stage` = build+push image +
    # bump the k8s overlay (k8s/ArgoCD); `prod` fails loud (GitOps-only).
    deploy_parser = subparsers.add_parser("deploy", aliases=["start", "up"], help="Deploy to environment")
    deploy_parser.add_argument("environment", choices=["dev", "development", "prod", "production", "stage"], help="Target environment")
    deploy_parser.add_argument("--tag", default=STAGE_DEFAULT_TAG,
                               help=f"Image tag for `deploy stage` (default: {STAGE_DEFAULT_TAG}, the floating tag)")

    # Stop command
    stop_parser = subparsers.add_parser("stop", help="Stop environment services")
    stop_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Logs command
    logs_parser = subparsers.add_parser("logs", help="Show environment logs")
    logs_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")
    logs_parser.add_argument("-f", "--follow", action="store_true", help="Follow log output")
    logs_parser.add_argument(
        "service",
        nargs="?",
        choices=["website", "api-serve", "api-sync", "cloudflared"],
        help="Specific service to show logs for",
    )

    # Status command
    status_parser = subparsers.add_parser("status", help="Show environment status")
    status_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Restart command
    restart_parser = subparsers.add_parser("restart", help="Restart environment services")
    restart_parser.add_argument("environment", choices=["dev", "development", "prod", "production"], help="Target environment")

    # Config command (read-only introspection; works for dev, prod, AND stage)
    config_parser = subparsers.add_parser("config", help="Show config vars + sources for an env (read-only audit)")
    config_parser.add_argument("environment", choices=["dev", "development", "prod", "production", "stage"], help="Target environment")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    # Normalize environment names
    env_map = {"dev": "dev", "development": "dev", "prod": "prod", "production": "prod", "stage": "stage"}
    environment = env_map[args.environment]

    # Execute command
    try:
        if args.command in ("deploy", "start", "up"):
            if environment == "stage":
                return deploy_stage(args.tag)
            return deploy_environment(environment)
        elif args.command == "stop":
            return stop_environment(environment)
        elif args.command == "logs":
            return show_logs(environment, args.follow, args.service)
        elif args.command == "status":
            return show_status(environment)
        elif args.command == "restart":
            return restart_environment(environment)
        elif args.command == "config":
            return show_config(environment)
        else:
            print_error(f"Unknown command: {args.command}")
            return 1
    except KeyboardInterrupt:
        print_warning("\nOperation cancelled by user")
        return 130
    except Exception as e:
        print_error(f"Unexpected error: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
