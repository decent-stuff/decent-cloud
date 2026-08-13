//! Integration test guarding the `dc-agent` clap CLI `--help` surface.
//!
//! After the #444 `main.rs` split (3681 -> 139 lines; all logic moved into
//! `dc_agent::*` lib modules), every subcommand's `--help` output must stay
//! byte-identical so the CLI surface never drifts silently. This test spawns
//! the built `dc-agent` binary for each subcommand, captures `--help` stdout,
//! and compares it (version-normalized) against a committed snapshot under
//! `tests/snapshots/`.
//!
//! ## When the CLI surface CHANGES intentionally
//! Rebuild the binary and regenerate every snapshot, then commit them:
//! ```text
//! cargo build -p dc-agent --release
//! BIN=./target/release/dc-agent
//! $BIN --help                 > dc-agent/tests/snapshots/root.txt
//! $BIN run --help             > dc-agent/tests/snapshots/run.txt
//! $BIN doctor --help          > dc-agent/tests/snapshots/doctor.txt
//! $BIN setup --help           > dc-agent/tests/snapshots/setup.txt
//! $BIN setup token --help     > dc-agent/tests/snapshots/setup_token.txt
//! $BIN test-provision --help  > dc-agent/tests/snapshots/test_provision.txt
//! $BIN upgrade --help         > dc-agent/tests/snapshots/upgrade.txt
//! $BIN reset-password --help  > dc-agent/tests/snapshots/reset_password.txt
//! ```
//! A version bump alone must NOT require regenerating -- the version token is
//! normalized out on both sides (see [`normalize_version`]).

use std::process::Command;

/// Strip the crate version out of help text so a Cargo.toml version bump does
/// not break this test.
///
/// clap derives the version from `CARGO_PKG_VERSION` and prints it as
/// `dc-agent <MAJOR>.<MINOR>.<PATCH>` (the `--version` line, and any help
/// header that ever embeds it). That token changes every release, so a
/// byte-exact snapshot would break spuriously. Replace it with a fixed
/// placeholder on BOTH the captured and committed text before byte-comparing.
/// Today this is a no-op (no `--help` output embeds the version), but it keeps
/// the snapshot stable the moment clap starts emitting the version in help.
fn normalize_version(output: &str) -> String {
    output.replace(
        &format!("dc-agent {}", env!("CARGO_PKG_VERSION")),
        "dc-agent VERSION",
    )
}

/// Spawn `dc-agent <args> --help`, capture stdout, and assert it equals the
/// committed snapshot (both version-normalized).
///
/// Prints the full expected/actual text on mismatch so a drift points straight
/// at the changed bytes.
fn assert_help_snapshot(args: &[&str], expected_raw: &str) {
    let bin = env!("CARGO_BIN_EXE_dc-agent");
    let label = if args.is_empty() {
        "dc-agent".to_string()
    } else {
        format!("dc-agent {}", args.join(" "))
    };

    let output = Command::new(bin)
        .args(args)
        .arg("--help")
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn `{label} --help`: {e}"));

    assert!(
        output.status.success(),
        "`{label} --help` exited {}\n--- stderr ---\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    let captured =
        String::from_utf8(output.stdout).expect("`--help` stdout was not valid UTF-8");
    let actual = normalize_version(&captured);
    let expected = normalize_version(expected_raw);

    assert_eq!(
        actual, expected,
        "\n`{label} --help` output drifted (version-normalized).\n\
         --- expected (snapshot) ---\n{expected}\n\
         --- actual (binary) ---\n{actual}",
    );
}

#[test]
fn help_root() {
    assert_help_snapshot(&[], include_str!("snapshots/root.txt"));
}

#[test]
fn help_run() {
    assert_help_snapshot(&["run"], include_str!("snapshots/run.txt"));
}

#[test]
fn help_doctor() {
    assert_help_snapshot(&["doctor"], include_str!("snapshots/doctor.txt"));
}

#[test]
fn help_setup() {
    assert_help_snapshot(&["setup"], include_str!("snapshots/setup.txt"));
}

#[test]
fn help_setup_token() {
    assert_help_snapshot(&["setup", "token"], include_str!("snapshots/setup_token.txt"));
}

#[test]
fn help_test_provision() {
    assert_help_snapshot(&["test-provision"], include_str!("snapshots/test_provision.txt"));
}

#[test]
fn help_upgrade() {
    assert_help_snapshot(&["upgrade"], include_str!("snapshots/upgrade.txt"));
}

#[test]
fn help_reset_password() {
    assert_help_snapshot(&["reset-password"], include_str!("snapshots/reset_password.txt"));
}
