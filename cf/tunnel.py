#!/usr/bin/env python3
"""Idempotent Cloudflare tunnel create-or-get + DNS ingress configuration.

Runs from CI (holds CF_API_TOKEN + CF_ACCOUNT_ID). Single source of truth for
tunnel names and hostname→service routing. Uses stdlib urllib only — the repo
is zero-Python-dependency (no `requests`).

Usage:
    python3 cf/tunnel.py prod            # prints connector token to stdout
    python3 cf/tunnel.py prod --json     # prints {"id":..., "token":...}

Connector tokens are issued ONLY at tunnel creation. If the tunnel already
exists, the script re-applies DNS/config (idempotent) and prints an empty
token — the token was captured on first creation and lives in the GitHub
secret TUNNEL_TOKEN_PROD/TUNNEL_TOKEN_DEV.
"""

import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

# ---------------------------------------------------------------------------
# Single source of truth: tunnel names + hostname→service routing.
# ---------------------------------------------------------------------------

ZONE_NAME = "decent-cloud.org"
CF_API_BASE = "https://api.cloudflare.com/client/v4"
HTTP_TIMEOUT = 30  # never bare — every HTTP call carries this

# (hostname, in-network service URL) tuples per environment.
TUNNELS: dict[str, dict] = {
    "prod": {
        # Matches the EXISTING live Cloudflare tunnel "decent-cloud" (connector id
        # c4e24160-...) so tunnel.py reuses it (re-points ingress) instead of
        # creating a duplicate. The connector token for it is stored as the
        # TUNNEL_TOKEN_PROD key of dc-secret in the k8s PGP-SOPS
        # store (see deploy/k8s/SETUP.md §3). configure_ingress() re-points the
        # live tunnel's hostnames to the in-cluster Service FQDNs below.
        "name": "decent-cloud",
        # In-cluster Services (namespace apps, all `dc-` prefixed). The
        # cloudflared Deployment runs in the same cluster and routes
        # decent-cloud.org via these FQDNs. Services expose port 80
        # (see deploy/k8s/decent-cloud/).
        "ingress": [
            ("decent-cloud.org", "http://dc-website.apps.svc.cluster.local:80"),
            ("api.decent-cloud.org", "http://dc-api.apps.svc.cluster.local:80"),
            ("support.decent-cloud.org", "http://dc-chatwoot-web.apps.svc.cluster.local:80"),
        ],
    },
    # dev keeps compose-style targets (docker service names + raw ports) for
    # the local docker-compose dev stack.
    "dev": {
        "name": "decent-cloud-dev",
        "ingress": [
            ("dev.decent-cloud.org", "http://website:59000"),
            ("dev-api.decent-cloud.org", "http://api-serve:59001"),
            ("dev-support.decent-cloud.org", "http://chatwoot-web:59002"),
        ],
    },
}


def _log(msg: str) -> None:
    """Loud progress to stderr (stdout is reserved for the token capture)."""
    print(f"[tunnel] {msg}", file=sys.stderr)


# ---------------------------------------------------------------------------
# HTTP wrapper — fail loud, always timeouted.
# ---------------------------------------------------------------------------

def cf_request(method: str, path: str, *, token: str, account_id: str | None = None, json_body: dict | None = None) -> dict:
    """Perform a Cloudflare API call. Returns the parsed JSON dict.

    Raises RuntimeError on missing token, non-2xx, URLError, or success:false —
    always surfacing the full CF errors array.
    """
    if not token:
        raise RuntimeError("CF_API_TOKEN is required but missing/empty")

    url = f"{CF_API_BASE}{path}"
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
    data = json.dumps(json_body).encode() if json_body is not None else None
    req = urllib.request.Request(url, data=data, headers=headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            body = resp.read().decode()
            code = resp.getcode()
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError:
            payload = {"errors": [{"message": raw}]}
        raise RuntimeError(
            f"Cloudflare API {method} {path} -> HTTP {e.code}: {payload.get('errors') or payload}"
        ) from e
    except urllib.error.URLError as e:
        raise RuntimeError(f"Cloudflare API {method} {path} unreachable: {e.reason}") from e

    if code < 200 or code >= 300:
        raise RuntimeError(f"Cloudflare API {method} {path} -> HTTP {code}")

    try:
        payload = json.loads(body)
    except json.JSONDecodeError as e:
        snippet = body.strip()[:200] or "(empty body)"
        raise RuntimeError(
            f"Cloudflare API {method} {path} -> HTTP {code} returned non-JSON: {snippet!r}"
        ) from e
    if not payload.get("success", False):
        raise RuntimeError(f"Cloudflare API {method} {path} failed: {payload.get('errors')}")
    return payload


# ---------------------------------------------------------------------------
# Zone lookup
# ---------------------------------------------------------------------------

def get_zone_id(token: str) -> str:
    """Return the zone id for ZONE_NAME. Loud error unless exactly one zone matches."""
    qs = urllib.parse.urlencode({"name": ZONE_NAME})
    payload = cf_request("GET", f"/zones?{qs}", token=token)
    zones = payload.get("result") or []
    if len(zones) != 1:
        raise RuntimeError(
            f"expected exactly 1 zone for '{ZONE_NAME}', found {len(zones)} — "
            "check the zone exists and CF_API_TOKEN can read it"
        )
    return zones[0]["id"]


# ---------------------------------------------------------------------------
# Tunnel create-or-get
# ---------------------------------------------------------------------------

def find_tunnel(token: str, account_id: str, name: str) -> str | None:
    """Return the tunnel id for `name`, or None if it does not exist."""
    qs = urllib.parse.urlencode({"name": name})
    payload = cf_request("GET", f"/accounts/{account_id}/cfd_tunnel?{qs}", token=token, account_id=account_id)
    tunnels = payload.get("result") or []
    return tunnels[0]["id"] if tunnels else None


def create_tunnel(token: str, account_id: str, name: str) -> tuple[str, str]:
    """Create a remotely-managed tunnel. Returns (tunnel_id, connector_token)."""
    payload = cf_request(
        "POST",
        f"/accounts/{account_id}/cfd_tunnel",
        token=token,
        account_id=account_id,
        json_body={"name": name, "tunnel_secret": True},
    )
    result = payload["result"]
    return result["id"], result["token"]


def ensure_tunnel(token: str, account_id: str, env: str) -> str:
    """Idempotent create-or-get. Returns the connector token.

    On create: returns the freshly-issued token (captured by CI as a GitHub
    secret). On reuse: returns "" — CF does not expose the token post-creation,
    so it must already be stored in TUNNEL_TOKEN_<ENV>.
    """
    name = TUNNELS[env]["name"]
    existing = find_tunnel(token, account_id, name)
    if existing is not None:
        _log(f"tunnel '{name}' exists (id={existing}) — reusing; connector token is NOT retrievable, "
             f"reuse the stored TUNNEL_TOKEN_{env.upper()}")
        return ""
    tunnel_id, connector_token = create_tunnel(token, account_id, name)
    _log(f"tunnel '{name}' created (id={tunnel_id})")
    return connector_token


# ---------------------------------------------------------------------------
# Ingress + DNS configuration
# ---------------------------------------------------------------------------

def configure_ingress(token: str, account_id: str, zone_id: str, env: str) -> None:
    """Apply tunnel ingress rules + upsert CNAME DNS records. Idempotent."""
    spec = TUNNELS[env]
    name = spec["name"]
    tunnel_id = find_tunnel(token, account_id, name)
    if not tunnel_id:
        raise RuntimeError(f"configure_ingress: tunnel '{name}' not found — call ensure_tunnel first")

    ingress_rules = [{"hostname": h, "service": s} for h, s in spec["ingress"]]
    ingress_rules.append({"service": "http_status:404"})  # catch-all (CF requires a final no-hostname rule)
    cf_request(
        "PUT",
        f"/accounts/{account_id}/cfd_tunnel/{tunnel_id}/configurations",
        token=token,
        account_id=account_id,
        json_body={"config": {"ingress": ingress_rules}},
    )
    _log(f"ingress config applied for '{name}' ({len(spec['ingress'])} hostnames)")

    cname_target = f"{tunnel_id}.cfargotunnel.com"
    for hostname, _service in spec["ingress"]:
        qs = urllib.parse.urlencode({"name": hostname, "type": "CNAME"})
        existing = cf_request("GET", f"/zones/{zone_id}/dns_records?{qs}", token=token)
        records = existing.get("result") or []
        if records:
            record_id = records[0]["id"]
            cf_request(
                "PATCH",
                f"/zones/{zone_id}/dns_records/{record_id}",
                token=token,
                json_body={"type": "CNAME", "name": hostname, "content": cname_target, "proxied": True},
            )
            _log(f"DNS PATCH {hostname} -> {cname_target} (id={record_id})")
        else:
            cf_request(
                "POST",
                f"/zones/{zone_id}/dns_records",
                token=token,
                json_body={"type": "CNAME", "name": hostname, "content": cname_target, "proxied": True},
            )
            _log(f"DNS POST {hostname} -> {cname_target}")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Create-or-get a Cloudflare tunnel and configure DNS ingress (idempotent).",
    )
    parser.add_argument("env", choices=["prod", "dev"], help="Target environment")
    parser.add_argument("--json", action="store_true", help='Emit {"id","token"} JSON instead of just the token')
    args = parser.parse_args(argv)

    token = os.environ.get("CF_API_TOKEN")
    account_id = os.environ.get("CF_ACCOUNT_ID")
    missing = [name for name, value in (("CF_API_TOKEN", token), ("CF_ACCOUNT_ID", account_id)) if not value]
    if missing:
        print(f"ERROR: missing required environment variable(s): {', '.join(missing)}", file=sys.stderr)
        print("Set CF_API_TOKEN (Cloudflare API token) and CF_ACCOUNT_ID (account id) in CI.", file=sys.stderr)
        sys.exit(1)

    try:
        connector_token = ensure_tunnel(token, account_id, args.env)
        zone_id = get_zone_id(token)
        configure_ingress(token, account_id, zone_id, args.env)
    except RuntimeError as e:
        msg = str(e)
        print(f"ERROR: {msg}", file=sys.stderr)
        if "HTTP 401" in msg or "Authentication error" in msg or "Invalid access token" in msg:
            print(
                "CF_API_TOKEN was rejected by Cloudflare. Verify the token is valid and has the "
                "scopes: Account:Read, Cloudflare Tunnel:Edit, Zone:Read, DNS:Edit. "
                "Regenerate it at https://one.dash.cloudflare.com/ -> Access -> Service Auth -> API Tokens.",
                file=sys.stderr,
            )
        return 1

    if args.json:
        tunnel_id = find_tunnel(token, account_id, TUNNELS[args.env]["name"])
        print(json.dumps({"id": tunnel_id, "token": connector_token}))
    else:
        print(connector_token)
    return 0


if __name__ == "__main__":
    sys.exit(main())
