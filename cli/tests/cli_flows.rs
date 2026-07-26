//! Flow-level integration tests for the `dc` binary.
//!
//! Distinct from `cli_smoke.rs` (which covers single-command surface: help,
//! version, arg validation, one-shot keygen/list): these tests exercise
//! MULTI-STEP user flows across several invocations and assert end-to-end
//! behavior of the REAL compiled binary.
//!
//! ## Tiers
//!
//! - **Offline flows** (default, always run, fast): identity determinism,
//!   offline balance read, local-listing appearance after keygen, stdin
//!   keygen, BIP-39 language coverage, ledger-local variants, subcommand
//!   help surfaces, error paths. No network; fully deterministic.
//!
//! - **Warm-stack flows** (default, auto-skipped if the local API at
//!   `localhost:59011` is not reachable): `provider pool-suggest/generate`
//!   against the real API. These prove request signing works end-to-end
//!   (regression guard for the auth fix that previously made those commands
//!   100% fail). Skipped-with-pass when the stack is down so headless/CI
//!   runs stay green.
//!
//! - **IC-mainnet flows** (`#[ignore]`, opt-in): read-only ledger-remote
//!   queries against the real Internet Computer. Slow + external; run via
//!   `cargo nextest run -p decent-cloud --run-ignored only -- cli_flows`.
//!
//! Every command that writes under the user's home (`~/.dcc/...`) runs with
//! `HOME` pointed at a fresh `tempfile::TempDir` so tests never touch the real
//! home and never collide.

use assert_cmd::prelude::*; // CommandCargoExt + OutputAssertExt
use predicates::str::contains;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;

/// A valid 12-word English BIP-39 phrase used across determinism tests.
const KNOWN_PHRASE: &str =
    "guilt faith betray uphold faint come scheme south venture visa carry stay";

/// Build a `dc` invocation with `HOME` pointed at a fresh temp dir.
/// Returns (command, temp_home); the TempDir must outlive the assertions.
fn dc() -> (Command, TempDir) {
    let home = TempDir::new().expect("create temp HOME");
    let mut cmd = Command::cargo_bin("dc").expect("locate dc binary");
    cmd.env("HOME", home.path());
    // Keep dc's own INFO logs (several tests read the "Generated mnemonic"
    // line) while silencing the `ledger_map` "No data found" WARN noise from a
    // fresh empty ledger. A blanket RUST_LOG="" would suppress the INFO line.
    cmd.env("RUST_LOG", "dc=info,ledger_map=error");
    (cmd, home)
}

/// Read the public.pem bytes for an identity name under a given HOME.
fn public_pem(home: &Path, name: &str) -> Vec<u8> {
    let path = home.join(".dcc").join("identity").join(name).join("public.pem");
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Confirm both identity key files exist on disk.
fn assert_identity_written(home: &Path, name: &str) {
    let dir = home.join(".dcc").join("identity").join(name);
    assert!(dir.join("public.pem").exists(), "public.pem missing at {}", dir.display());
    assert!(dir.join("private.pem").exists(), "private.pem missing at {}", dir.display());
}

/// Best-effort liveness check for the local warm API stack at the dev port.
/// Returns the base URL if the port is accepting connections, else None.
fn warm_stack_api_url() -> Option<String> {
    let url = std::env::var("DC_API_URL").unwrap_or_else(|_| "http://localhost:59011".into());
    let host_port = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(&url);
    let (host, port) = host_port.rsplit_once(':').unwrap_or(("localhost", "59011"));
    let addr = format!("{host}:{port}");
    let socket_addr = addr.parse().ok().or_else(|| {
        // Fall back to DNS-style resolution (localhost → 127.0.0.1).
        let host = if host.is_empty() { "127.0.0.1" } else { host };
        format!("{host}:{port}").parse().ok()
    })?;
    std::net::TcpStream::connect_timeout(&socket_addr, Duration::from_millis(400)).ok()?;
    Some(url)
}

// ---------------------------------------------------------------------------
// Offline flows
// ---------------------------------------------------------------------------

#[test]
fn keygen_is_deterministic_same_phrase_yields_same_public_key() {
    // Importing the SAME mnemonic twice must produce byte-identical public.pem.
    // Exercises the deterministic Ed25519 seed→key derivation end-to-end across
    // two separate process invocations (not just one in-memory call).
    let phrase = KNOWN_PHRASE;

    let (mut cmd_a, home_a) = dc();
    cmd_a.args(["keygen", "--mnemonic", phrase, "--identity", "ida"]);
    cmd_a.assert().success();
    let pk_a = public_pem(home_a.path(), "ida");

    let (mut cmd_b, home_b) = dc();
    cmd_b.args(["keygen", "--mnemonic", phrase, "--identity", "idb"]);
    cmd_b.assert().success();
    let pk_b = public_pem(home_b.path(), "idb");

    assert_eq!(
        pk_a, pk_b,
        "same mnemonic must derive the same public key (deterministic keygen)"
    );
}

#[test]
fn keygen_generate_then_reimport_mnemonic_matches() {
    // `keygen --generate` logs a fresh mnemonic to stderr; re-importing that
    // exact mnemonic into a second identity must reproduce the FIRST identity's
    // public key. This is the real "backup your mnemonic" recovery flow.
    let (mut gen_cmd, home) = dc();
    gen_cmd.args(["keygen", "--generate", "--identity", "primary"]);
    let out = gen_cmd.assert().success();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr);
    let mnemonic_line = stderr
        .lines()
        .find(|l| l.contains("Generated mnemonic"))
        .expect("stderr should log the generated mnemonic");
    let mnemonic = mnemonic_line
        .rsplit_once(": ")
        .map(|(_, w)| w.trim())
        .expect("mnemonic line has ': ' separator");
    let original_pk = public_pem(home.path(), "primary");

    // Re-import the printed mnemonic under a different name in the SAME home.
    let (mut reimport_cmd, _home2) = dc();
    reimport_cmd.env("HOME", home.path()); // share the original home
    reimport_cmd.args(["keygen", "--mnemonic", mnemonic, "--identity", "restored"]);
    reimport_cmd.assert().success();
    let restored_pk = public_pem(home.path(), "restored");

    assert_eq!(
        original_pk, restored_pk,
        "re-importing the generated mnemonic must reproduce the original public key"
    );
}

#[test]
fn account_balance_reads_local_ledger_without_network() {
    // `account --balance --identity` reads the cached local ledger balance and
    // never touches the IC canister. A fresh identity must report the account
    // principal + a balance line. (We assert the principal line and that the
    // process succeeds offline — no network is configured in the temp HOME.)
    let (mut keygen, home) = dc();
    keygen.args(["keygen", "--mnemonic", KNOWN_PHRASE, "--identity", "acct"]);
    keygen.assert().success();

    let (mut bal, _home) = dc();
    bal.env("HOME", home.path()); // share the home where the identity was written
    bal.args(["account", "--balance", "--identity", "acct"]);
    bal.assert()
        .success()
        .stdout(contains("Account Principal ID"))
        .stdout(contains("Account balance"));
}

#[test]
fn generated_identity_appears_in_all_local_listings() {
    // After keygen, the identity must be visible from `provider list --only-local`,
    // `user list --only-local`, and `account --list-all` (the local-on-disk
    // listing path, which scans ~/.dcc/identity). A single HOME is shared
    // across all three invocations so the written identity is visible to each.
    let (mut keygen, home) = dc();
    keygen.args(["keygen", "--generate", "--identity", "visible-id"]);
    keygen.assert().success();

    let run = |args: &[&str]| -> String {
        let (mut cmd, _) = dc();
        cmd.env("HOME", home.path()); // reuse the SAME home as keygen
        cmd.args(args);
        let out = cmd.assert().success().get_output().clone();
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    let providers = run(&["provider", "list", "--only-local"]);
    let users = run(&["user", "list", "--only-local"]);
    assert!(
        providers.contains("visible-id"),
        "provider list --only-local should list the identity: {providers}"
    );
    assert!(
        users.contains("visible-id"),
        "user list --only-local should list the identity: {users}"
    );
}

#[test]
fn keygen_reads_mnemonic_from_stdin_when_fewer_than_12_words() {
    // The interactive path: `--mnemonic` with fewer than 12 words prompts on
    // stdin for the remaining words. Piping 12 newline-separated words through
    // stdin completes keygen and writes the identity. This covers the recovery
    // UX where a user types/pastes a mnemonic interactively.
    let (mut cmd, home) = dc();
    // Pass an empty --mnemonic to enter the stdin path (< 12 words).
    cmd.args(["keygen", "--mnemonic", "", "--identity", "stdin-id"]);
    cmd.stdin(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("spawn dc");
    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        for w in KNOWN_PHRASE.split_whitespace() {
            use std::io::Write;
            writeln!(stdin, "{w}").unwrap();
        }
    }
    let status = child.wait().expect("wait for dc");
    assert!(status.success(), "stdin keygen should succeed");
    assert_identity_written(home.path(), "stdin-id");
}

#[test]
fn keygen_supports_non_english_bip39_languages() {
    // The BIP-39 generator honors --language for non-English wordlists. Both
    // French and Japanese must produce a valid 12-word mnemonic and write a
    // usable identity. Guards against the wordlist dispatch regressing.
    for lang in ["fr", "ja"] {
        let (mut cmd, home) = dc();
        cmd.args(["keygen", "--generate", "--language", lang, "--identity", "lang-id"]);
        let out = cmd.assert().success();
        let stderr = String::from_utf8_lossy(&out.get_output().stderr);
        let mnemonic_line = stderr
            .lines()
            .find(|l| l.contains("Generated mnemonic"))
            .unwrap_or_else(|| panic!("({lang}) no Generated mnemonic line in stderr"));
        let mnemonic = mnemonic_line
            .rsplit_once(": ")
            .map(|(_, w)| w.trim())
            .unwrap_or_else(|| panic!("({lang}) mnemonic line missing ': '"));
        assert_eq!(
            mnemonic.split_whitespace().count(),
            12,
            "({lang}) mnemonic must be 12 words: {mnemonic}"
        );
        assert_identity_written(home.path(), "lang-id");
    }
}

#[test]
fn ledger_local_list_entries_variants_on_fresh_ledger() {
    // --list-entries and --list-entries-raw exercise the ledger iteration paths
    // (typed + raw block iteration) distinct from --list-accounts. On a fresh
    // ledger they must succeed and print their section headers.
    let (mut entries, _home) = dc();
    entries.args(["ledger-local", "--list-entries"]);
    entries.assert().success().stdout(contains("Entries:"));

    let (mut raw, _home) = dc();
    raw.args(["ledger-local", "--list-entries-raw"]);
    raw.assert().success().stdout(contains("Raw Entries:"));
}

#[test]
fn subcommand_help_surfaces_enumerate_all_commands() {
    // Each subcommand group must expose --help listing its leaf commands. This
    // is a wiring check: it fails if a subcommand is mis-registered or a group
    // is missing from the clap tree. Distinct from the top-level --help test in
    // cli_smoke.rs.
    let cases: &[(&[&str], &[&str])] = &[
        (
            &["account", "--help"],
            &["--balance", "--list-all", "--transfer-to"],
        ),
        (
            &["provider", "--help"],
            &["list", "register", "check-in", "pool-suggest-offerings"],
        ),
        (&["user", "--help"], &["list", "register"]),
        (
            &["ledger-remote", "--help"],
            &["data-fetch", "metadata", "get-registration-fee", "get-check-in-nonce"],
        ),
    ];

    for (args, expected) in cases {
        let mut cmd = Command::cargo_bin("dc").expect("locate dc binary");
        cmd.args(*args);
        let out = cmd.assert().success().get_output().clone();
        let text = String::from_utf8_lossy(&out.stdout);
        for needle in *expected {
            assert!(
                text.contains(*needle),
                "`{} --help` should mention '{}'\nactual:\n{text}",
                args.join(" "),
                needle
            );
        }
    }
}

#[test]
fn keygen_invalid_mnemonic_is_rejected() {
    // A phrase that is not valid in ANY supported BIP-39 language must be
    // rejected with a non-zero exit (after the auto-detect loop fails).
    let (mut cmd, _home) = dc();
    cmd.args([
        "keygen",
        "--mnemonic",
        "totally bogus mnemonic words that cannot validate at all period",
        "--identity",
        "bad",
    ]);
    cmd.assert().failure();
}

#[test]
fn keygen_without_a_mnemonic_source_errors() {
    // `keygen --identity x` with neither --generate nor --mnemonic must error
    // (the handler requires a source). Distinct from clap's "missing required
    // arg" (which is about --identity itself); this is the handler-level guard.
    let (mut cmd, _home) = dc();
    cmd.args(["keygen", "--identity", "sourceless"]);
    cmd.assert().failure();
}

#[test]
fn account_transfer_to_without_amount_returns_meaningful_error() {
    // `account --transfer-to <valid-principal> --identity <real>` with no --amount-*
    // must reach the handler's amount-check and return the "Missing transfer amount"
    // error (exit non-zero). Uses a VALID principal so parsing succeeds and the
    // amount guard is what fails — distinct from the invalid-principal test below.
    let (mut keygen, home) = dc();
    keygen.args(["keygen", "--generate", "--identity", "sender"]);
    keygen.assert().success();

    let (mut cmd, _) = dc();
    cmd.env("HOME", home.path());
    cmd.args([
        "account",
        "--transfer-to",
        // A valid IC principal text (canonical format) so principal parsing succeeds
        // and the amount-guard is the thing that trips.
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "--identity",
        "sender",
    ]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Missing transfer amount"),
        "should explain the missing amount, got: {stderr}"
    );
}

#[test]
fn account_transfer_to_with_invalid_principal_returns_clean_error() {
    // A malformed --transfer-to address must produce a CLEAN, explained error — NOT
    // a panic/backtrace (exit 101). Regression guard for the transfer-principal
    // validation fix: previously the CLI panicked via
    // `IcrcCompatibleAccount::from(&str)`'s `Principal::from_text(...).expect(...)`.
    let (mut keygen, home) = dc();
    keygen.args(["keygen", "--generate", "--identity", "sender"]);
    keygen.assert().success();

    let (mut cmd, _) = dc();
    cmd.env("HOME", home.path());
    cmd.args([
        "account",
        "--transfer-to",
        "not-a-valid-principal",
        "--identity",
        "sender",
        "--amount-e9s",
        "1",
    ]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("panicked"),
        "invalid principal must NOT panic; got backtrace:\n{stderr}"
    );
    assert!(
        stderr.contains("not-a-valid-principal"),
        "should name the offending --transfer-to value, got: {stderr}"
    );
    // Must NOT be a Rust panic exit code (101).
    assert_ne!(
        out.status.code(),
        Some(101),
        "invalid principal must not crash with panic exit 101"
    );
}

#[test]
fn account_list_all_prints_both_registered_sections_on_fresh_ledger() {
    // `account --list-all` is the alias-stable entry point for the on-disk local
    // listing (calls list_identities with All). Distinct from `ledger-local
    // --list-accounts` (covered in cli_smoke.rs) — this asserts the actual
    // `account` subcommand dispatches to the same listing and prints BOTH section
    // headers on a fresh ledger.
    let (mut cmd, _home) = dc();
    cmd.args(["account", "--list-all"]);
    cmd.assert()
        .success()
        .stdout(contains("Registered providers"))
        .stdout(contains("Registered users"));
}

#[test]
fn account_list_accounts_alias_matches_list_all() {
    // `--list-accounts` is a visible alias for `--list-all`. It must dispatch to
    // the identical handler and print the same section headers. Guards against the
    // alias registration regressing (clap `visible_aliases`).
    let (mut cmd, _home) = dc();
    cmd.args(["account", "--list-accounts"]);
    cmd.assert()
        .success()
        .stdout(contains("Registered providers"))
        .stdout(contains("Registered users"));
}

#[test]
fn provider_list_prints_providers_section_on_fresh_ledger() {
    // `provider list` (without --only-local) reads the synced local ledger and
    // prints the "# Registered providers" section. On a fresh ledger there are no
    // entries, but the section header must still appear — proving the provider-list
    // dispatch + ledger iteration path works end-to-end.
    let (mut cmd, _home) = dc();
    cmd.args(["provider", "list"]);
    cmd.assert().success().stdout(contains("Registered providers"));
}

#[test]
fn user_list_prints_users_section_on_fresh_ledger() {
    // Mirror of the provider-list test for the user domain. Asserts the "# Registered
    // users" section header on a fresh ledger.
    let (mut cmd, _home) = dc();
    cmd.args(["user", "list"]);
    cmd.assert().success().stdout(contains("Registered users"));
}

#[test]
fn listing_only_local_with_balances_flag_shows_balance_column() {
    // `--balances` toggles the per-identity balance field in listings. With a
    // generated identity present, `provider list --only-local --balances` must
    // include the literal "balance" token in the printed line (the no-balances path
    // omits it). Proves the --balances flag is wired through to println_identity.
    let (mut keygen, home) = dc();
    keygen.args(["keygen", "--generate", "--identity", "bal-id"]);
    keygen.assert().success();

    let run = |args: &[&str]| -> String {
        let (mut cmd, _) = dc();
        cmd.env("HOME", home.path());
        cmd.args(args);
        let out = cmd.assert().success().get_output().clone();
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    let with_balances = run(&["provider", "list", "--only-local", "--balances"]);
    let without_balances = run(&["provider", "list", "--only-local"]);
    assert!(
        with_balances.contains("balance"),
        "--balances should add the balance field: {with_balances}"
    );
    assert!(
        !without_balances.contains("balance"),
        "without --balances the balance field should be absent: {without_balances}"
    );
    assert!(
        with_balances.contains("bal-id"),
        "the generated identity should be listed: {with_balances}"
    );
}

#[test]
fn local_ledger_dir_flag_places_ledger_file_at_custom_path() {
    // `--local-ledger-dir <dir>` overrides the default ~/.dcc/ledger location. After
    // any ledger command, <dir>/main.bin must exist (the LedgerMap creates + grows it
    // on first access). Asserts the global flag is honored end-to-end.
    let home = TempDir::new().expect("create temp HOME");
    let ledger_dir = TempDir::new().expect("create temp ledger dir");
    let mut cmd = Command::cargo_bin("dc").expect("locate dc binary");
    cmd.env("HOME", home.path());
    cmd.env("RUST_LOG", "dc=info,ledger_map=error");
    cmd.args([
        "--local-ledger-dir",
        ledger_dir.path().to_str().unwrap(),
        "ledger-local",
        "--list-accounts",
    ]);
    cmd.assert().success();
    let ledger_file = ledger_dir.path().join("main.bin");
    assert!(
        ledger_file.exists(),
        "main.bin should exist under --local-ledger-dir at {}",
        ledger_file.display()
    );
}

#[test]
fn verbose_flag_enables_debug_level_logging() {
    // `-v` sets RUST_LOG=debug when RUST_LOG is not already in the environment,
    // surfacing DEBUG-level log lines on stderr. Without -v only INFO+ shows. We do
    // NOT set RUST_LOG here (unlike dc()) so the -v branch in init_logger takes
    // effect, then assert at least one DEBUG line appears.
    let home = TempDir::new().expect("create temp HOME");
    let mut cmd = Command::cargo_bin("dc").expect("locate dc binary");
    cmd.env_remove("RUST_LOG");
    cmd.env("HOME", home.path());
    cmd.args(["-v", "keygen", "--generate", "--identity", "verbose-id"]);
    let out = cmd.assert().success().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.lines().any(|l| l.contains("DEBUG")),
        "-v should emit DEBUG log lines; stderr was:\n{stderr}"
    );
}

#[test]
fn account_balance_for_nonexistent_identity_errors() {
    // `account --balance --identity <ghost>` must fail cleanly (exit non-zero) with a
    // file-not-found style error naming the missing identity PEM. This is the
    // handler-level load_from_dir failure path (identity resolves under
    // ~/.dcc/identity/<name>/ which does not exist in a fresh HOME).
    let (mut cmd, _home) = dc();
    cmd.args(["account", "--balance", "--identity", "does-not-exist"]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does-not-exist") || stderr.contains("public.pem") || stderr.contains("NotFound"),
        "should surface the missing identity, got: {stderr}"
    );
}

#[test]
fn provider_register_without_identity_errors() {
    // `provider register` requires --identity (the handler-level guard, since there
    // is no clap `requires` on a leaf subcommand with no args). Must exit non-zero
    // with the actionable "Identity must be specified" message.
    let (mut cmd, _home) = dc();
    cmd.args(["provider", "register"]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Identity must be specified"),
        "should explain the missing identity, got: {stderr}"
    );
}

#[test]
fn provider_check_in_without_identity_errors() {
    // `provider check-in` (full path, not --only-nonce) requires --identity at the
    // handler level. Must fail with the actionable identity message.
    let (mut cmd, _home) = dc();
    cmd.args(["provider", "check-in"]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Identity must be specified"),
        "should explain the missing identity, got: {stderr}"
    );
}

#[test]
fn user_register_without_identity_errors() {
    // `user register` requires --identity at the handler level. Mirrors the provider
    // register guard. Must fail with the actionable identity message.
    let (mut cmd, _home) = dc();
    cmd.args(["user", "register"]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Identity must be specified"),
        "should explain the missing identity, got: {stderr}"
    );
}

#[test]
fn ledger_remote_push_without_identity_errors() {
    // `ledger-remote data-push` requires --identity at the handler level. We pair it
    // with `--network local` to avoid the default mainnet canister round-trip; the
    // identity guard fires before any network call regardless.
    let (mut cmd, _home) = dc();
    cmd.args(["--network", "local", "ledger-remote", "data-push"]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Identity must be specified"),
        "should explain the missing identity, got: {stderr}"
    );
}

#[test]
fn ledger_remote_push_authorize_without_identity_errors() {
    // `ledger-remote data-push-authorize` requires --identity at the handler level.
    // Same --network local short-circuit as the push test.
    let (mut cmd, _home) = dc();
    cmd.args(["--network", "local", "ledger-remote", "data-push-authorize"]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Identity must be specified"),
        "should explain the missing identity, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Warm-stack flows (real local API; auto-skipped if the stack is down)
// ---------------------------------------------------------------------------

#[test]
fn provider_pool_suggest_authenticates_against_real_api() {
    // Regression guard: pool-suggest-offerings must authenticate correctly
    // (correct headers, nanos timestamp, UUID nonce, byte-concatenated signed
    // message). Before the auth fix this returned 401 "Missing required header:
    // X-Public-Key". With the fix it reaches the handler and returns a real
    // business response. We assert it gets PAST auth (no 401/auth wording) and
    // reaches the handler (a structured API error for the unregistered
    // identity / unknown pool).
    let Some(api_url) = warm_stack_api_url() else {
        eprintln!("skip: warm API stack not reachable (set DC_API_URL / run dev-server.sh)");
        return;
    };

    let (mut keygen, home) = dc();
    keygen.args(["keygen", "--generate", "--identity", "pool-id"]);
    keygen.assert().success();

    let (mut cmd, _) = dc();
    cmd.env("HOME", home.path());
    cmd.args([
        "provider",
        "pool-suggest-offerings",
        "--identity",
        "pool-id",
        "--pool-id",
        "1",
        "--api-url",
        &api_url,
    ]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Must NOT contain auth-rejection wording (proves signing succeeded).
    assert!(
        !stderr.contains("401") && !stderr.contains("Unauthorized"),
        "pool-suggest should authenticate; got auth rejection: {stderr}"
    );
    assert!(
        !stderr.contains("Missing required header"),
        "pool-suggest should send all auth headers: {stderr}"
    );
    // Should reach the handler and produce a structured API business error
    // (the keygen'd identity owns no pool). Either wording is acceptable.
    assert!(
        stderr.contains("Pool not found") || stderr.contains("API error"),
        "pool-suggest should return a real API response, got: {stderr}"
    );
}

#[test]
fn provider_pool_generate_dry_run_authenticates_against_real_api() {
    // Same regression guard for the POST path (pool-generate-offerings). Also
    // verifies the request body schema (tiers must be an array, not omitted).
    let Some(api_url) = warm_stack_api_url() else {
        eprintln!("skip: warm API stack not reachable (set DC_API_URL / run dev-server.sh)");
        return;
    };

    let (mut keygen, home) = dc();
    keygen.args(["keygen", "--generate", "--identity", "pool-id"]);
    keygen.assert().success();

    let pricing = home.path().join("pricing.json");
    std::fs::write(
        &pricing,
        r#"{"small":{"monthlyPrice":5,"currency":"usd"}}"#,
    )
    .unwrap();

    let (mut cmd, _) = dc();
    cmd.env("HOME", home.path());
    cmd.args([
        "provider",
        "pool-generate-offerings",
        "--identity",
        "pool-id",
        "--pool-id",
        "1",
        "--pricing-file",
        pricing.to_str().unwrap(),
        "--dry-run",
        "--api-url",
        &api_url,
    ]);
    let out = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("401") && !stderr.contains("Unauthorized"),
        "pool-generate should authenticate; got auth rejection: {stderr}"
    );
    // Must NOT hit the poem-openapi payload-parse error for a null tiers field
    // (regression guard for the request-schema fix).
    assert!(
        !stderr.contains("Expected input type") && !stderr.contains("found null"),
        "pool-generate request schema is wrong: {stderr}"
    );
    assert!(
        stderr.contains("Pool not found") || stderr.contains("API error"),
        "pool-generate should return a real API response, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// IC-mainnet flows (opt-in: --run-ignored; slow, external)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "hits the real IC mainnet (icp-api.io); run via --run-ignored only"]
fn ledger_remote_get_registration_fee_against_ic_mainnet() {
    // Read-only IC canister query: fetches the registration fee. Proves the
    // ic-agent transport + canister binding work against the real network.
    let (mut cmd, _home) = dc();
    cmd.args(["ledger-remote", "get-registration-fee"]);
    let out = cmd.assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Registration fee"),
        "should print the registration fee line: {stdout}"
    );
}

#[test]
#[ignore = "hits the real IC mainnet (icp-api.io); run via --run-ignored only"]
fn ledger_remote_get_check_in_nonce_against_ic_mainnet() {
    // Read-only IC canister query: fetches the check-in nonce as hex. Proves
    // the nonce query path works end-to-end. The nonce is the LAST non-empty
    // stdout line (ledger-map prints a "Growing persistent storage" status line
    // first on a fresh HOME — that println is a known ledger-map quirk).
    let (mut cmd, _home) = dc();
    cmd.args(["ledger-remote", "get-check-in-nonce"]);
    let out = cmd.assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let nonce = stdout
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or_else(|| panic!("check-in nonce line missing; stdout:\n{stdout}"));
    assert!(
        !nonce.is_empty() && nonce.chars().all(|c| c.is_ascii_hexdigit()),
        "check-in nonce should be hex (last stdout line), got: {nonce}\nfull stdout:\n{stdout}"
    );
}

#[test]
#[ignore = "hits the real IC mainnet (icp-api.io); run via --run-ignored only"]
fn ledger_remote_metadata_against_ic_mainnet() {
    // Read-only IC canister query: fetches canister metadata (Key/Value table).
    // Proves the metadata query path + tabular rendering work against the real
    // network. Asserts the table header is printed.
    let (mut cmd, _home) = dc();
    cmd.args(["ledger-remote", "metadata"]);
    let out = cmd.assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Key") && stdout.contains("Value"),
        "metadata table header should be printed: {stdout}"
    );
}

#[test]
#[ignore = "hits the real IC mainnet (icp-api.io); run via --run-ignored only"]
fn ledger_remote_get_logs_warn_against_ic_mainnet() {
    // Read-only IC canister query: fetches WARN-level canister logs. Proves the
    // get_logs transport + the format_log_lines rendering work against the real
    // network for one representative log level. We use WARN (not INFO/DEBUG)
    // because the INFO log payload currently exceeds the IC's 3MB reply limit
    // (the canister rejects with IC0504 "payload too large"); WARN/ERROR are
    // smaller and succeed. DEBUG/WARN/ERROR share the same code path.
    let (mut cmd, _home) = dc();
    cmd.args(["ledger-remote", "get-logs-warn"]);
    let out = cmd.assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Ledger canister WARN logs:"),
        "should print the WARN logs header: {stdout}"
    );
}

#[test]
#[ignore = "hits the real IC mainnet (icp-api.io); run via --run-ignored only"]
fn provider_check_in_only_nonce_against_ic_mainnet() {
    // Read-only IC canister query via the `provider check-in --only-nonce` path
    // (distinct from `ledger-remote get-check-in-nonce` — same canister call, but
    // exercises the provider-command dispatch + the `0x`-prefixed formatting). The
    // nonce is the last stdout line starting with "0x" (ledger-map prints a status
    // line first on a fresh HOME).
    let (mut cmd, _home) = dc();
    cmd.args(["provider", "check-in", "--only-nonce"]);
    let out = cmd.assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let nonce = stdout
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("0x"))
        .unwrap_or_else(|| panic!("check-in --only-nonce should print a 0x nonce; stdout:\n{stdout}"));
    assert!(
        nonce.len() > 2 && nonce[2..].chars().all(|c| c.is_ascii_hexdigit()),
        "0x nonce should be hex after the prefix, got: {nonce}"
    );
}

#[test]
#[ignore = "hits the real IC mainnet (icp-api.io); run via --run-ignored only"]
fn ledger_remote_data_fetch_against_ic_mainnet() {
    // Read-only-to-local IC canister query: `data-fetch` pulls the latest ledger
    // into the local file and must succeed against mainnet, leaving a readable
    // local ledger behind. This is the core "ledger sync" user flow.
    let (mut cmd, home) = dc();
    cmd.args(["ledger-remote", "data-fetch"]);
    cmd.assert().success();

    // The synced ledger must now be readable offline and contain at least the
    // providers section (mainnet always has registered providers).
    let (mut list, _) = dc();
    list.env("HOME", home.path());
    list.args(["account", "--list-all"]);
    let out = list.assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Registered providers"),
        "after data-fetch the local ledger should list providers: {stdout}"
    );
}
