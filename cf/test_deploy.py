"""Tests for cf.deploy — silent-failure robustness fixes.

Mocks ONLY the I/O boundary (subprocess, pathlib) to assert the NEW loud
behavior: swallowed errors must be surfaced, silent sentinels must warn.
"""

from unittest import mock

import cf.deploy as deploy


# ---------------------------------------------------------------------------
# check_tunnel_status — the swallowed `except Exception: return "error"`
# ---------------------------------------------------------------------------

def test_check_tunnel_status_logs_error_on_subprocess_failure(capsys):
    """When the status check blows up (e.g. docker missing), the exception must be
    surfaced loudly — NOT swallowed into a bare 'error' with no detail. Regression
    for the bare `except Exception: return "error"` that lost the root cause."""
    with mock.patch.object(deploy.subprocess, "run", side_effect=FileNotFoundError("[Errno 2] docker: not found")):
        result = deploy.check_tunnel_status(["cf/docker-compose.dev.yml"], "dev")
    assert result == "error"
    err = capsys.readouterr().err  # print_error writes to stderr
    assert "Tunnel status check failed" in err
    assert "docker: not found" in err  # the actual root cause is preserved


def test_check_tunnel_status_still_returns_error_on_oserror(capsys):
    """Any failure class (not just FileNotFoundError) must be surfaced, not muted."""
    with mock.patch.object(deploy.subprocess, "run", side_effect=OSError("boom")):
        result = deploy.check_tunnel_status(["cf/docker-compose.dev.yml"], "prod")
    assert result == "error"
    assert "boom" in capsys.readouterr().err


# ---------------------------------------------------------------------------
# calculate_binary_hash — silent "no-binary" sentinel must now warn loudly
# ---------------------------------------------------------------------------

def test_calculate_binary_hash_warns_loudly_when_binary_missing(capsys):
    """A missing API binary is a path/build misconfiguration — must warn loudly,
    not silently return the constant 'no-binary' cache key (which would let a
    stale Docker image ship unnoticed)."""
    with mock.patch.object(deploy.Path, "exists", return_value=False):
        result = deploy.calculate_binary_hash()
    assert result == "no-binary"  # sentinel preserved (caller flow unchanged)
    out = capsys.readouterr().out  # print_warning writes to stdout
    assert "API binary not found" in out
    assert "no-binary" in out  # names the stale-cache consequence
