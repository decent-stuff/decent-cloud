# Ticket 348: Pre-built Docker image with openssh-server

## PoC Result

**PASS** — Docker image builds and runs sshd successfully.

### Verification evidence

```
$ bash dc-agent/container/build.sh dc-agent-ssh latest
Building dc-agent-ssh:latest ...
[... build output ...]
Verifying sshd starts ...
OK: sshd is running inside <container-id>
Container stopped.
Image size:
dc-agent-ssh:latest  86.7MB
```

### Files added

| File | Purpose |
|------|---------|
| `dc-agent/container/Dockerfile` | ubuntu:22.04 + openssh-server + PermitRootLogin yes + ENTRYPOINT sshd |
| `dc-agent/container/build.sh` | Build + verify script (builds image, runs container, checks sshd) |
| `dc-agent/container/publish.sh` | Tag + push to ghcr.io |
| `dc-agent/container/README.md` | This file: build/publish workflow + dev-stage code change plan |

### Image details

- Base: `ubuntu:22.04`
- Installed: `openssh-server`, `ca-certificates`
- Config: `/run/sshd` created, `PermitRootLogin yes`, `PermitEmptyPasswords no`
- Entrypoint: `/usr/sbin/sshd -D -e`
- Size: ~87MB
- Exposed port: 22

### Build & publish workflow

```bash
# Build locally
bash dc-agent/container/build.sh dc-agent-ssh latest

# Publish to GHCR
bash dc-agent/container/publish.sh ghcr.io decent-stuff
# Result: ghcr.io/decent-stuff/dc-agent-ssh:latest
```

For CI, add a GitHub Actions workflow that runs `build.sh` + `publish.sh` on pushes to `main` that touch `dc-agent/container/`.

---

## Dev-stage code change plan

### 1. Change default image — `config.rs:593-594`

```rust
// Before:
fn default_docker_image() -> String {
    "ubuntu:22.04".to_string()
}

// After:
fn default_docker_image() -> String {
    "ghcr.io/decent-stuff/dc-agent-ssh:latest".to_string()
}
```

Also update the test at `config.rs:1810`:
```rust
assert_eq!(docker.default_image, "ghcr.io/decent-stuff/dc-agent-ssh:latest");
```

And the test helper structs at `config.rs:1819,1830` that use `"ubuntu:22.04"` — keep those as-is since they're testing explicit config, not defaults.

### 2. Simplify CMD — `docker.rs:187-198`

The pre-built image already has openssh-server installed. The CMD only needs to:
1. Inject the SSH public key into `/root/.ssh/authorized_keys`
2. Exec sshd

```rust
// Before (7-line inline bash with apt-get):
let cmd = Some(vec![
    "/bin/bash".to_string(),
    "-c".to_string(),
    concat!(
        "set -e; ",
        "apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq openssh-server; ",
        "mkdir -p /root/.ssh && chmod 700 /root/.ssh; ",
        r#"[ -n "$SSH_PUBLIC_KEY" ] && printf '%s\n' "$SSH_PUBLIC_KEY" > /root/.ssh/authorized_keys && chmod 600 /root/.ssh/authorized_keys; "#,
        "mkdir -p /run/sshd; ",
        "exec /usr/sbin/sshd -D -e"
    ).to_string(),
]);

// After (SSH key injection only, no apt-get):
let cmd = Some(vec![
    "/bin/bash".to_string(),
    "-c".to_string(),
    concat!(
        "set -e; ",
        "mkdir -p /root/.ssh && chmod 700 /root/.ssh; ",
        r#"[ -n "$SSH_PUBLIC_KEY" ] && printf '%s\n' "$SSH_PUBLIC_KEY" > /root/.ssh/authorized_keys && chmod 600 /root/.ssh/authorized_keys; "#,
        "exec /usr/sbin/sshd -D -e"
    ).to_string(),
]);
```

### 3. Remove ubuntu:22.04 warning — `docker.rs:611-618`

```rust
// Remove this block from verify_setup():
if self.config.default_image == "ubuntu:22.04" {
    result.warnings.push(
        "Default image 'ubuntu:22.04' is used. SSH server will be installed on first boot \
         via apt-get, which can be slow. Consider using a pre-built image with openssh-server \
         already installed."
            .to_string(),
    );
}
```

### 4. Update tests — `docker_tests.rs`

- `test_build_container_config_has_cmd` (line 334): Remove assertion for `openssh-server` in cmd; keep assertions for `authorized_keys` and `sshd -D`.
- `test_verify_setup_image_found` (line 412): Change mock image list to include `ghcr.io/decent-stuff/dc-agent-ssh:latest` if testing with new default; or keep `ubuntu:22.04` if testing with explicit image override.
- `default_config()` (line 5): Update to use the new default image name, or keep `ubuntu:22.04` for backward compat tests.

### 5. Update `dc-agent.toml.example`

Change the commented-out `default_image` line to reference the new image:
```toml
# default_image = "ghcr.io/decent-stuff/dc-agent-ssh:latest"
```

### 6. Backward compatibility note

Existing deployments with `default_image = "ubuntu:22.04"` in their config will still work — the CMD just does redundant apt-get. The dev stage may want to keep the old CMD as a fallback when running against a bare ubuntu image, OR just document the breaking change. Recommend: just switch to the new image and require the pre-built image going forward.
