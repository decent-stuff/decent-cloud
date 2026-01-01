# Production Readiness Verification

**Status:** Draft
**Created:** 2026-01-01
**Author:** Claude Code

## Summary

Establish and enforce production readiness standards across the entire codebase. Ensure all code properly captures and displays stdout/stderr on errors, never silently ignores failures, follows idiomatic patterns, provides excellent UX, and maintains comprehensive test coverage without redundancy.

## Problem Statement

The codebase has accumulated technical debt in several areas:

1. **Silent failures**: Some code uses `let _ = ` or ignores `Result` types
2. **Lost output**: Command execution may not capture/show stderr on failure
3. **Weak error messages**: Errors lack sufficient detail for troubleshooting
4. **Poor UX**: Some workflows are confusing or lack clear feedback
5. **Test redundancy**: Some tests overlap or assert weak signals
6. **Idiomatic issues**: Non-idiomatic patterns in places (e.g., not using `match`)

This makes production debugging difficult and violates the project's fail-fast principles.

## Requirements

### Must-have

- [ ] All command stdout/stderr captured and displayed on error
- [ ] No silent failures (no `let _ = ` for Result types)
- [ ] Idiomatic Rust code throughout (use `match`, `?`, proper error handling)
- [ ] Clear error messages with context ("{:#?}" for complex errors)
- [ ] Loud warnings for misconfigurations (use `tracing::warn!`)
- [ ] Simple, obvious UX in all user-facing features
- [ ] Complete test coverage for all code paths
- [ ] No test overlap - each test asserts unique behavior
- [ ] Remove weak-signal tests
- [ ] Each commit represents a meaningful, self-contained unit of work

### Nice-to-have

- [ ] Automated linting in CI (cargo clippy --tests)
- [ ] Test coverage metrics
- [ ] Pre-commit hooks for formatting

## Verification Checklist

For each module in the codebase, verify:

### Error Handling

- [ ] No `let _ = ` ignoring Result types
- [ ] All errors are propagated with `?` or handled explicitly
- [ ] Error messages include context ("failed to X: {:#?}")
- [ ] Command execution captures both stdout and stderr
- [ ] Stderr displayed on command failure

Example of correct pattern:
```rust
let output = Command::new("some-command")
    .arg("--flag")
    .output()
    .context("Failed to execute some-command")?;

if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("some-command failed: {}", stderr);
}
```

### Idiomatic Code

- [ ] Use `match` for enum handling (not `if let` when you need all cases)
- [ ] Use `?` for error propagation
- [ ] Use `format!` or `anyhow::bail!` for errors with context
- [ ] No `unwrap()` or `expect()` except in tests
- [ ] Proper use of `Option` and `Result`

Example:
```rust
// Good
match result {
    Ok(value) => process(value),
    Err(e) => return Err(e).context("Failed to process"),
}

// Bad
if let Ok(value) = result {
    process(value)
}
// Error silently ignored!
```

### UX Standards

- [ ] CLI commands provide clear feedback
- [ ] Progress indicators for long operations
- [ ] Error messages are actionable
- [ ] Navigation is obvious (menus, sidebars updated)
- [ ] Configuration is validated with clear error messages
- [ ] Misconfigurations logged with `tracing::warn!` including what's broken and how to fix

Example warning:
```rust
if config.api_key.is_none() {
    tracing::warn!(
        "API_KEY not set - External integrations will NOT work! \
        Set API_KEY in .env or via --api-key flag to enable."
    );
}
```

### Test Standards

- [ ] All public functions have tests
- [ ] Tests assert meaningful behavior (not implementation details)
- [ ] No test overlap - each test covers unique scenario
- [ ] Remove tests that:
  - Only test internal implementation
  - Duplicate coverage from other tests
  - Assert weak signals (e.g., "returns Ok" without checking value)
- [ ] Both positive and negative paths tested

Example of good test:
```rust
#[test]
fn test_provision_vm_creates_record_in_db() {
    // Setup
    let db = TestDb::new();
    let params = default_params();

    // Execute
    let result = provision_vm(&db, &params);

    // Assert meaningful behavior
    assert!(result.is_ok());
    let vm = db.get_vm(result.unwrap().id).unwrap();
    assert_eq!(vm.status, VMStatus::Running);
}
```

Example of weak test to remove:
```rust
#[test]
fn test_provision_vm_returns_ok() {
    // Bad: Only checks return type, not behavior
    assert!(provision_vm(&db, &params).is_ok());
}
```

## Implementation Plan

### Phase 1: Audit Current State

1. **Find silent failures:**
   ```bash
   grep -r "let _ = " --include="*.rs" .
   ```

2. **Find unwrap/expect in production code:**
   ```bash
   grep -r "\.unwrap()" --include="*.rs" . | grep -v "tests/"
   grep -r "\.expect(" --include="*.rs" . | grep -v "tests/"
   ```

3. **Find non-idiomatic patterns:**
   ```bash
   grep -r "if let Ok(" --include="*.rs" .
   ```

4. **Check command execution patterns:**
   - Search for `std::process::Command`
   - Verify all usage captures stdout/stderr
   - Verify errors include stderr content

### Phase 2: Fix by Module

For each module that fails audit:

1. **Update error handling** (no silent failures)
2. **Improve error messages** (add context with `{:#?}`)
3. **Make idiomatic** (use `match`, `?`)
4. **Add/update tests** (meaningful assertions, no overlap)
5. **Remove weak tests** (duplicates, implementation tests)
6. **Improve UX** (clear feedback, progress indicators)

### Phase 3: Verify Standards

Run for each crate:
```bash
# Lint
cargo clippy --tests

# Tests
cargo nextest run

# Manual verification
# - Run the tool locally
# - Test error scenarios
# - Verify UX is clear
```

## Success Criteria

- Zero `let _ = ` ignoring Result in production code
- Zero `unwrap()`/`expect()` in production code
- All command execution captures stderr and shows on error
- All error messages include sufficient context
- All tests assert meaningful, unique behavior
- Zero test overlap (confirmed by coverage analysis)
- `cargo clippy --tests` passes with zero warnings
- `cargo nextest run` passes all tests
- All workflows have clear, obvious UX

## Notes

This is a **cross-cutting concern** affecting all crates in the monorepo. Work should be done incrementally, module by module, with commits for each self-contained unit of work.

Priority order:
1. Core services (api-server, dc-agent)
2. Provisioners (proxmox, etc.)
3. CLI tools and utilities
4. Integrations and peripheral services

Each module completion should be a separate commit with descriptive message like:
```
Fix production readiness issues in api-server

- Capture stderr in all Command executions
- Replace silent failures with proper error handling
- Improve error messages with context
- Remove weak/overlapping tests
- Add missing test coverage
```
