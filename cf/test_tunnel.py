"""Tests for cf.tunnel — Cloudflare tunnel create-or-get + DNS ingress.

Mocks ONLY the I/O boundary: urllib.request.urlopen (via cf_request) and the
module-level helpers that wrap it (find_tunnel/create_tunnel when testing the
ensure_tunnel composition). Asserts behavior, not call ordering beyond what the
contract requires.
"""

import io
import json
import urllib.error
from unittest import mock

import pytest

import cf.tunnel as tunnel


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

def _urlopen_mock(payload, code=200):
    """A context-manager mock matching urllib.request.urlopen's return shape."""
    cm = mock.MagicMock()
    cm.__enter__.return_value = cm
    cm.getcode.return_value = code
    cm.read.return_value = json.dumps(payload).encode()
    return cm


# ---------------------------------------------------------------------------
# cf_request
# ---------------------------------------------------------------------------

def test_cf_request_rejects_missing_token():
    """No token => loud RuntimeError naming CF_API_TOKEN (fail-fast)."""
    with pytest.raises(RuntimeError, match="CF_API_TOKEN"):
        tunnel.cf_request("GET", "/zones", token=None)
    with pytest.raises(RuntimeError, match="CF_API_TOKEN"):
        tunnel.cf_request("GET", "/zones", token="")


def test_cf_request_raises_on_success_false():
    """CF returns success:false => errors array surfaced in the message."""
    body = {"success": False, "errors": [{"message": "auth denied"}]}
    with mock.patch.object(tunnel.urllib.request, "urlopen", return_value=_urlopen_mock(body)):
        with pytest.raises(RuntimeError, match="auth denied"):
            tunnel.cf_request("GET", "/zones", token="tk")


def test_cf_request_raises_on_http_error():
    """Non-2xx HTTP => errors array from the error body surfaced."""
    body = json.dumps({"success": False, "errors": [{"message": "forbidden"}]}).encode()
    err = urllib.error.HTTPError("https://api.cloudflare.com/x", 403, "Forbidden", {}, io.BytesIO(body))
    with mock.patch.object(tunnel.urllib.request, "urlopen", side_effect=err):
        with pytest.raises(RuntimeError, match="forbidden"):
            tunnel.cf_request("GET", "/zones", token="tk")


def test_cf_request_passes_timeout():
    """Every HTTP call must carry the module HTTP_TIMEOUT (never bare)."""
    with mock.patch.object(tunnel.urllib.request, "urlopen", return_value=_urlopen_mock({"success": True, "result": {}})) as u:
        tunnel.cf_request("GET", "/zones", token="tk")
    assert u.call_args.kwargs["timeout"] == tunnel.HTTP_TIMEOUT


def test_cf_request_raises_on_non_json_success_body():
    """A 2xx response with a non-JSON body must surface the endpoint + body
    snippet — not a bare JSONDecodeError with zero context."""
    cm = mock.MagicMock()
    cm.__enter__.return_value = cm
    cm.getcode.return_value = 200
    cm.read.return_value = b"<html>proxy intercept - not json</html>"
    with mock.patch.object(tunnel.urllib.request, "urlopen", return_value=cm):
        with pytest.raises(RuntimeError, match=r"GET /zones.*non-JSON") as exc:
            tunnel.cf_request("GET", "/zones", token="tk")
    assert "HTTP 200" in str(exc.value)
    assert "not json" in str(exc.value)


# ---------------------------------------------------------------------------
# find_tunnel
# ---------------------------------------------------------------------------

def test_find_tunnel_returns_id_when_present():
    with mock.patch.object(tunnel, "cf_request", return_value={"success": True, "result": [{"id": "tun-abc"}]}):
        assert tunnel.find_tunnel("tk", "acc", "decent-cloud-prod") == "tun-abc"


def test_find_tunnel_returns_none_when_absent():
    with mock.patch.object(tunnel, "cf_request", return_value={"success": True, "result": []}):
        assert tunnel.find_tunnel("tk", "acc", "decent-cloud-prod") is None


# ---------------------------------------------------------------------------
# ensure_tunnel — core idempotency
# ---------------------------------------------------------------------------

def test_ensure_tunnel_creates_when_absent():
    """Absent => create_tunnel called exactly once; connector token returned."""
    with mock.patch.object(tunnel, "find_tunnel", return_value=None), \
         mock.patch.object(tunnel, "create_tunnel", return_value=("tun-abc", "connector-token")) as cr:
        result = tunnel.ensure_tunnel("tk", "acc", "prod")
    assert result == "connector-token"
    assert cr.call_count == 1


def test_ensure_tunnel_reuses_when_present():
    """Present => create_tunnel NEVER called (idempotency — no duplicate POST)."""
    with mock.patch.object(tunnel, "find_tunnel", return_value="tun-abc"), \
         mock.patch.object(tunnel, "create_tunnel") as cr:
        tunnel.ensure_tunnel("tk", "acc", "prod")
    assert cr.call_count == 0


# ---------------------------------------------------------------------------
# get_zone_id
# ---------------------------------------------------------------------------

def test_get_zone_id_loud_error_on_zero_zones():
    with mock.patch.object(tunnel, "cf_request", return_value={"success": True, "result": []}):
        with pytest.raises(RuntimeError, match="1 zone"):
            tunnel.get_zone_id("tk")


def test_get_zone_id_loud_error_on_multiple_zones():
    payload = {"success": True, "result": [{"id": "z1"}, {"id": "z2"}]}
    with mock.patch.object(tunnel, "cf_request", return_value=payload):
        with pytest.raises(RuntimeError, match="1 zone"):
            tunnel.get_zone_id("tk")


def test_get_zone_id_returns_id_when_unique():
    with mock.patch.object(tunnel, "cf_request", return_value={"success": True, "result": [{"id": "zone-1"}]}):
        assert tunnel.get_zone_id("tk") == "zone-1"


# ---------------------------------------------------------------------------
# configure_ingress
# ---------------------------------------------------------------------------

def test_configure_ingress_upserts_cname_records():
    """Absent hostname => POST CNAME; present => PATCH; target = {id}.cfargotunnel.com."""
    # call order: find_tunnel(GET) -> PUT config -> per-hostname GET then POST/PATCH
    responses = [
        {"success": True, "result": [{"id": "tun-1"}]},          # find_tunnel
        {"success": True, "result": None},                        # PUT configurations
        {"success": True, "result": []},                          # GET decent-cloud.org (absent)
        {"success": True, "result": {"id": "rec-new-a"}},         # POST decent-cloud.org
        {"success": True, "result": [{"id": "rec-1"}]},           # GET api.decent-cloud.org (present)
        {"success": True, "result": {"id": "rec-1"}},             # PATCH api.decent-cloud.org
        {"success": True, "result": []},                          # GET support.decent-cloud.org (absent)
        {"success": True, "result": {"id": "rec-new-c"}},         # POST support.decent-cloud.org
    ]
    with mock.patch.object(tunnel, "cf_request", side_effect=responses) as cr:
        tunnel.configure_ingress("tk", "acc", "zone-1", "prod")

    posts = [c for c in cr.call_args_list if c.args[0] == "POST"]
    patches = [c for c in cr.call_args_list if c.args[0] == "PATCH"]
    assert len(posts) == 2, "absent hostnames (decent-cloud.org, support) must be POSTed"
    assert len(patches) == 1, "present hostname (api) must be PATCHed"

    expected_target = "tun-1.cfargotunnel.com"
    for c in posts + patches:
        body = c.kwargs.get("json_body") or {}
        assert body.get("type", "CNAME") == "CNAME"
        assert body.get("content") == expected_target, f"CNAME target mismatch: {body}"


# ---------------------------------------------------------------------------
# TUNNELS routing table — prod routes to in-cluster k8s FQDNs, dev to compose
# ---------------------------------------------------------------------------

def test_prod_ingress_targets_in_cluster_services():
    """prod is served by the k3s cloudflared Deployment: each hostname routes to
    a ClusterIP Service FQDN on port 80 (deploy/k8s/decent-cloud/). Guards
    against accidental regression to the old compose-style service names."""
    ingress = dict(tunnel.TUNNELS["prod"]["ingress"])
    assert ingress["decent-cloud.org"] == "http://dc-website.apps.svc.cluster.local:80"
    assert ingress["api.decent-cloud.org"] == "http://dc-api.apps.svc.cluster.local:80"
    assert ingress["support.decent-cloud.org"] == "http://dc-chatwoot-web.apps.svc.cluster.local:80"
    # Every prod target must be an in-cluster FQDN, not a bare compose hostname.
    for service in ingress.values():
        assert service.endswith(".apps.svc.cluster.local:80"), service


def test_prod_tunnel_name_matches_live_tunnel():
    """prod MUST reuse the existing live tunnel named 'decent-cloud' (connector id
    c4e24160-...). If this name drifts, tunnel.py creates a DUPLICATE tunnel and the
    live CNAME records (-> c4e24160.cfargotunnel.com) stop matching, breaking prod."""
    assert tunnel.TUNNELS["prod"]["name"] == "decent-cloud"


def test_dev_ingress_targets_compose_services():
    """dev keeps docker-compose service names + raw ports (local dev stack)."""
    ingress = dict(tunnel.TUNNELS["dev"]["ingress"])
    for service in ingress.values():
        assert ".svc.cluster.local" not in service, service


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def test_main_invalid_env_exits_nonzero():
    with pytest.raises(SystemExit):
        tunnel.main(["bogus"])


def test_main_missing_cf_credentials_exits_nonzero(monkeypatch):
    """Missing CF_API_TOKEN/CF_ACCOUNT_ID => exit 1 with a loud, named message."""
    monkeypatch.delenv("CF_API_TOKEN", raising=False)
    monkeypatch.delenv("CF_ACCOUNT_ID", raising=False)
    with pytest.raises(SystemExit) as exc:
        tunnel.main(["prod"])
    assert exc.value.code == 1


def test_main_auth_error_is_clean_and_actionable(monkeypatch, capsys):
    """A CF API auth error (HTTP 401) must return 1 with an actionable hint naming
    the required scopes — NOT an unhandled traceback. Regression for the bad-token path."""
    monkeypatch.setenv("CF_API_TOKEN", "bogus")
    monkeypatch.setenv("CF_ACCOUNT_ID", "acc-1")
    with mock.patch.object(tunnel, "ensure_tunnel",
                           side_effect=RuntimeError("Cloudflare API GET /x -> HTTP 401: Authentication error")):
        rc = tunnel.main(["prod"])
    assert rc == 1
    err = capsys.readouterr().err
    assert "HTTP 401" in err
    assert "CF_API_TOKEN was rejected" in err
    assert "Cloudflare Tunnel:Edit" in err  # actionable scope hint
