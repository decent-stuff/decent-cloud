"""Tests for cf.deploy — silent-failure robustness + the k8s stage image-tag bumper.

Mocks ONLY the I/O boundary (subprocess, pathlib) to assert the NEW loud
behavior: swallowed errors must be surfaced, silent sentinels must warn.
The stage section exercises `_update_stage_image_tag` (pure logic; no mocks).
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


# ---------------------------------------------------------------------------
# _update_stage_image_tag — the dc-stage overlay image bumper (pure logic)
# ---------------------------------------------------------------------------

def _write_kustomization(tmp_path, text):
    p = tmp_path / "kustomization.yaml"
    p.write_text(text)
    return p


def test_updates_existing_newtag_indented(tmp_path):
    k = _write_kustomization(tmp_path, (
        "apiVersion: kustomize.config.k8s.io/v1beta1\n"
        "kind: Kustomization\n"
        "resources: []\n"
        "images:\n"
        "  - name: git.kalaj.org/decent-stuff/decent-cloud-api\n"
        "    newTag: 445a17d4\n"
        "  - name: git.kalaj.org/decent-stuff/decent-cloud-website\n"
        "    newTag: v0.5.5-hotfix.445a17d4\n"
    ))
    changed = deploy._update_stage_image_tag(k, "stage")
    assert changed is True
    out = k.read_text()
    assert "decent-cloud-api\n    newTag: stage\n" in out
    assert "newTag: v0.5.5-hotfix.445a17d4\n" in out  # website untouched


def test_updates_existing_newtag_column_zero_list(tmp_path):
    # kustomize also allows list items at column 0.
    k = _write_kustomization(tmp_path, (
        "images:\n"
        "- name: git.kalaj.org/decent-stuff/decent-cloud-api\n"
        "  newTag: oldtag\n"
        "namePrefix: stage-\n"
    ))
    changed = deploy._update_stage_image_tag(k, "stage")
    assert changed is True
    out = k.read_text()
    assert "- name: git.kalaj.org/decent-stuff/decent-cloud-api\n  newTag: stage\n" in out
    assert "namePrefix: stage-\n" in out  # block ended at the top-level key, not the list item


def test_inserts_newtag_when_absent(tmp_path):
    k = _write_kustomization(tmp_path, (
        "images:\n"
        "  - name: git.kalaj.org/decent-stuff/decent-cloud-api\n"
    ))
    changed = deploy._update_stage_image_tag(k, "stage")
    assert changed is True
    assert ("  - name: git.kalaj.org/decent-stuff/decent-cloud-api\n"
            "    newTag: stage\n") in k.read_text()


def test_idempotent_noop_returns_false(tmp_path):
    original = (
        "images:\n"
        "  - name: git.kalaj.org/decent-stuff/decent-cloud-api\n"
        "    newTag: stage\n"
    )
    k = _write_kustomization(tmp_path, original)
    changed = deploy._update_stage_image_tag(k, "stage")
    assert changed is False
    assert k.read_text() == original  # byte-for-byte unchanged


def test_does_not_match_website_image(tmp_path):
    # `decent-cloud-api` must NOT substring-match `decent-cloud-website`.
    k = _write_kustomization(tmp_path, (
        "images:\n"
        "  - name: git.kalaj.org/decent-stuff/decent-cloud-website\n"
        "    newTag: v0.5.5\n"
    ))
    try:
        deploy._update_stage_image_tag(k, "stage")
    except RuntimeError as e:
        assert "decent-cloud-api" in str(e)
    else:
        raise AssertionError("expected RuntimeError for missing api target")


def test_missing_file_raises(tmp_path):
    try:
        deploy._update_stage_image_tag(tmp_path / "nope.yaml", "stage")
    except RuntimeError as e:
        assert "not found" in str(e)
        assert "Track 1" in str(e)
    else:
        raise AssertionError("expected RuntimeError for missing file")


def test_missing_images_section_raises(tmp_path):
    k = _write_kustomization(tmp_path, (
        "apiVersion: kustomize.config.k8s.io/v1beta1\n"
        "kind: Kustomization\n"
    ))
    try:
        deploy._update_stage_image_tag(k, "stage")
    except RuntimeError as e:
        assert "images:" in str(e)
    else:
        raise AssertionError("expected RuntimeError for missing images section")


def test_preserves_rest_of_file_byte_for_byte(tmp_path):
    # Comments + sibling keys outside the api entry must survive untouched.
    text = (
        "# Managed overlay — do not edit the api tag by hand\n"
        "apiVersion: kustomize.config.k8s.io/v1beta1\n"
        "kind: Kustomization\n"
        "namespace: dc-stage\n"
        "resources:\n"
        "  - ../../base\n"
        "images:\n"
        "  - name: git.kalaj.org/decent-stuff/decent-cloud-api\n"
        "    newTag: 445a17d4\n"
        "  - name: git.kalaj.org/decent-stuff/decent-cloud-website\n"
        "    newName: git.kalaj.org/decent-stuff/decent-cloud-website\n"
        "    newTag: v0.5.5-hotfix.445a17d4\n"
        "patches:\n"
        "  - target:\n"
        "      kind: Deployment\n"
    )
    k = _write_kustomization(tmp_path, text)
    deploy._update_stage_image_tag(k, "stage")
    # Only the api newTag line differs; everything else identical.
    assert k.read_text() == text.replace("newTag: 445a17d4\n", "newTag: stage\n")
    out = k.read_text()
    assert "# Managed overlay" in out
    assert "patches:\n" in out
