//! Real subprocess smoke tests for the `dc` binary.
//!
//! Unlike string-literal unit tests, these drive the actual compiled binary via
//! `assert_cmd::cargo_bin("dc")` and assert on real exit codes, stdout, stderr,
//! and filesystem side effects. They cover the local-only CLI surface that does
//! not require a live IC canister or network:
//!   - `--help` / `--version` / `-V` (clap surface)
//!   - clap arg validation (missing required arg, conflicting flags, unknown subcommand,
//!     no-args-help)
//!   - `keygen --generate` and `keygen --mnemonic <phrase>` (write identity to disk)
//!   - `ledger-local --list-accounts` (local ledger load + refresh path in main)
//!   - `--network <invalid>` error dispatch
//!
//! Every command that writes under the user's home (`~/.dcc/...`) runs with `HOME`
//! pointed at a fresh `tempfile::TempDir` so tests never touch the real home and never
//! collide with each other.

use assert_cmd::prelude::*; // CommandCargoExt (cargo_bin) + OutputAssertExt (assert)
use predicates::str::contains;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Build a `dc` invocation with `HOME` pointed at a fresh temp dir.
///
/// Returns the command plus the temp dir (which must be kept alive for the test's
/// duration so it is not cleaned up before assertions read from it).
fn dc_with_isolated_home() -> (Command, TempDir) {
    let home = TempDir::new().expect("create temp HOME");
    let mut cmd = Command::cargo_bin("dc").expect("locate dc binary");
    cmd.env("HOME", home.path());
    (cmd, home)
}

/// Confirm the identity directory was written with both key files.
fn assert_identity_written(home: &Path, name: &str) {
    let dir = home.join(".dcc").join("identity").join(name);
    assert!(
        dir.join("public.pem").exists(),
        "public.pem missing at {}",
        dir.display()
    );
    assert!(
        dir.join("private.pem").exists(),
        "private.pem missing at {}",
        dir.display()
    );
}

#[test]
fn help_lists_all_subcommands() {
    // `--help` exits 0 and lists every top-level subcommand on stdout.
    Command::cargo_bin("dc")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Decent Cloud CLI"))
        .stdout(contains("keygen"))
        .stdout(contains("account"))
        .stdout(contains("provider"))
        .stdout(contains("user"))
        .stdout(contains("ledger-local"))
        .stdout(contains("ledger-remote"));
}

#[test]
fn version_short_and_long_flags_match_pkg_version() {
    // Both `-V` and `--version` print `dcc <CARGO_PKG_VERSION>` and exit 0.
    let expected = format!("dcc {}", env!("CARGO_PKG_VERSION"));

    let long = Command::cargo_bin("dc").unwrap().arg("--version").output().unwrap();
    assert!(long.status.success(), "--version should exit 0");
    assert_eq!(
        String::from_utf8_lossy(&long.stdout).trim(),
        expected,
        "--version output"
    );

    let short = Command::cargo_bin("dc").unwrap().arg("-V").output().unwrap();
    assert!(short.status.success(), "-V should exit 0");
    assert_eq!(
        String::from_utf8_lossy(&short.stdout).trim(),
        expected,
        "-V output"
    );
}

#[test]
fn keygen_generate_writes_identity_and_logs_12_word_mnemonic() {
    // `keygen --generate --identity <name>`:
    //   - exits 0
    //   - logs "Generated mnemonic" to stderr (the real keygen path uses `info!`)
    //   - emits exactly 12 space-separated words in that mnemonic line
    //   - writes public.pem + private.pem into ~/.dcc/identity/<name>
    let (mut cmd, home) = dc_with_isolated_home();
    cmd.args(["keygen", "--generate", "--identity", "smoke-gen"]);

    let assert = cmd.assert().success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let mnemonic_line = stderr
        .lines()
        .find(|line| line.contains("Generated mnemonic"))
        .expect("stderr should contain a 'Generated mnemonic' line");

    // The mnemonic follows the last ": " on that log line.
    let mnemonic = mnemonic_line
        .rsplit_once(": ")
        .map(|(_, words)| words.trim())
        .expect("mnemonic line should have ': ' separator");
    let word_count = mnemonic.split_whitespace().count();
    assert_eq!(
        word_count, 12,
        "generated mnemonic must be 12 words, got: {mnemonic}"
    );

    assert_identity_written(home.path(), "smoke-gen");
}

#[test]
fn keygen_from_mnemonic_phrase_writes_identity() {
    // Importing a known-valid 12-word phrase (no network) writes the identity and
    // exits 0. Distinct from --generate: exercises the deterministic import path,
    // not the random generator.
    const PHRASE: &str =
        "guilt faith betray uphold faint come scheme south venture visa carry stay";

    let (mut cmd, home) = dc_with_isolated_home();
    cmd.args(["keygen", "--mnemonic", PHRASE, "--identity", "smoke-import"]);

    cmd.assert()
        .success()
        .stderr(contains("Generated identity"));

    assert_identity_written(home.path(), "smoke-import");
}

#[test]
fn ledger_local_list_accounts_on_fresh_ledger() {
    // `ledger-local --list-accounts` is fully local: it exercises main()'s ledger
    // load + refresh path and the local ledger command, printing the provider/user
    // section headers on stdout. Fresh isolated HOME => empty ledger.
    let (mut cmd, _home) = dc_with_isolated_home();
    cmd.args(["ledger-local", "--list-accounts"]);

    cmd.assert()
        .success()
        .stdout(contains("Registered providers"))
        .stdout(contains("Registered users"));
}

#[test]
fn invalid_network_is_rejected_with_nonzero_exit() {
    // An unknown `--network` must be rejected. main() returns Err and the process
    // exits non-zero with the offending network name surfaced in stderr.
    let (mut cmd, _home) = dc_with_isolated_home();
    cmd.args(["--network", "bogus-net", "ledger-local", "--list-accounts"]);

    cmd.assert()
        .failure()
        .stderr(contains("bogus-net"));
}

#[test]
fn missing_required_identity_arg_is_a_clap_error() {
    // `keygen --generate` without `--identity` (declared `requires = "identity"`)
    // must fail fast with a clap usage error (exit code 2).
    Command::cargo_bin("dc")
        .unwrap()
        .args(["keygen", "--generate"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("required arguments were not provided"))
        .stderr(contains("--identity"));
}

#[test]
fn conflicting_keygen_flags_are_rejected() {
    // `--generate` and `--mnemonic` are mutually exclusive (conflicts_with). Supplying
    // both must be a clap error (exit 2), not silently accepted.
    Command::cargo_bin("dc")
        .unwrap()
        .args(["keygen", "--generate", "--mnemonic", "x", "--identity", "y"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn unknown_subcommand_is_a_clap_error() {
    // An unrecognized subcommand must be rejected by clap with exit code 2.
    Command::cargo_bin("dc")
        .unwrap()
        .arg("frobnicate")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn no_args_prints_help_and_exits_nonzero() {
    // With no arguments the CLI is configured `arg_required_else_help = true`, so it
    // prints help to stderr and exits non-zero (does not silently succeed). Unlike an
    // explicit `--help`, clap routes this error-triggered help to stderr with code 2.
    Command::cargo_bin("dc")
        .unwrap()
        .assert()
        .failure()
        .code(2)
        .stderr(contains("Decent Cloud CLI"));
}

#[test]
fn account_balance_without_identity_is_a_clap_error() {
    // `--balance` is declared `requires = "identity"`, so omitting --identity must
    // fail fast at the clap layer (exit 2) with a message naming --identity. This is
    // distinct from the handler-level identity guard tested in cli_flows.rs.
    Command::cargo_bin("dc")
        .unwrap()
        .args(["account", "--balance"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("required arguments were not provided"))
        .stderr(contains("--identity"));
}

#[test]
fn account_transfer_to_conflicting_amount_flags_is_a_clap_error() {
    // `--amount-e9s` and `--amount-dct` are declared `conflicts_with`. Supplying both
    // must be rejected by clap (exit 2), never silently picked between.
    let (mut cmd, _home) = dc_with_isolated_home();
    cmd.args([
        "account",
        "--transfer-to",
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "--identity",
        "x",
        "--amount-e9s",
        "1",
        "--amount-dct",
        "1",
    ]);
    cmd.assert().failure().code(2);
}

#[test]
fn ledger_remote_subcommand_aliases_resolve() {
    // The data-fetch/data-push/data-push-authorize commands expose visible aliases
    // (fetch/pull, push, push-authorize/push-auth). To prove clap ACCEPTS each alias
    // (rather than rejecting it as "unrecognized subcommand") WITHOUT touching the
    // network, we pair it with `--network bogus`: handle_command validates the
    // network BEFORE dispatching the subcommand handler, so a valid alias reaches
    // the InvalidNetwork error (exit 1, mentions "bogus"), while an unknown alias
    // is rejected by clap first (exit 2, "unrecognized subcommand").
    for alias in ["fetch", "pull", "push", "push-auth", "push-authorize"] {
        let (mut cmd, _home) = dc_with_isolated_home();
        cmd.args(["--network", "bogus", "ledger-remote", alias]);
        let out = cmd.assert().get_output().clone();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(1),
            "`ledger-remote {alias}` should resolve as an alias (exit 1 from InvalidNetwork), got {:?}",
            out.status.code()
        );
        assert!(
            stderr.contains("bogus") && !stderr.contains("unrecognized subcommand"),
            "`ledger-remote {alias}` should be accepted; stderr: {stderr}"
        );
    }
}
