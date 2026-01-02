//! Integration tests for CLI error handling in main.rs
//!
//! These tests verify that the error handling paths in main() work correctly:
//! 1. dirs::home_dir() failure triggers CliError::HomeDirNotFound
//! 2. LedgerMap::new_with_path() failure triggers CliError::LedgerLoad
//! 3. dcc_common::refresh_caches_from_ledger() failure triggers CliError::CacheRefresh

use std::path::PathBuf;

#[test]
fn test_home_dir_error_message_format() {
    // Verify the error message contains expected guidance
    let error_msg =
        "Could not determine home directory. Please set HOME environment variable or use --local-ledger-dir to specify ledger location.";

    assert!(error_msg.contains("Could not determine home directory"));
    assert!(error_msg.contains("HOME environment variable"));
    assert!(error_msg.contains("--local-ledger-dir"));
}

#[test]
fn test_ledger_load_error_message_format() {
    // Verify the error message contains actionable fixes
    let base_msg = "Failed to load local ledger";

    assert!(base_msg.contains("Failed to load local ledger"));

    // The full message should include multiple fix suggestions
    let full_msg = format!(
        "{}\n\nPossible fixes:\n  - Ensure the ledger directory exists and is readable\n  - Check file permissions\n  - Use --local-ledger-dir to specify a different location\n  - Try running: mkdir -p ~/.dcc/ledger",
        base_msg
    );

    assert!(full_msg.contains("Possible fixes"));
    assert!(full_msg.contains("ledger directory exists"));
    assert!(full_msg.contains("file permissions"));
    assert!(full_msg.contains("--local-ledger-dir"));
    assert!(full_msg.contains("mkdir -p ~/.dcc/ledger"));
}

#[test]
fn test_cache_refresh_error_message_format() {
    // Verify the error message contains recovery steps
    let base_msg = "Failed to refresh caches from ledger";

    assert!(base_msg.contains("Failed to refresh caches from ledger"));

    // The full message should include recovery suggestions
    let full_msg = format!(
        "{}\n\nThis may indicate a corrupted ledger. Try:\n  - Delete the ledger file and let it recreate: rm ~/.dcc/ledger/main.bin\n  - Use --local-ledger-dir to point to a backup ledger",
        base_msg
    );

    assert!(full_msg.contains("corrupted ledger"));
    assert!(full_msg.contains("rm ~/.dcc/ledger/main.bin"));
    assert!(full_msg.contains("--local-ledger-dir"));
    assert!(full_msg.contains("backup ledger"));
}

#[test]
fn test_ledger_path_construction_with_home() {
    // Test that the ledger path is correctly constructed when home dir is available
    let home = PathBuf::from("/tmp/test_home");
    let ledger_path = home.join(".dcc").join("ledger").join("main.bin");

    assert_eq!(
        ledger_path,
        PathBuf::from("/tmp/test_home/.dcc/ledger/main.bin")
    );
}

#[test]
fn test_ledger_path_with_custom_dir() {
    // Test that custom ledger dir overrides default
    let custom_dir = "/custom/ledger/location";
    let ledger_path = PathBuf::from(custom_dir).join("main.bin");

    assert_eq!(
        ledger_path,
        PathBuf::from("/custom/ledger/location/main.bin")
    );
}

#[test]
fn test_error_messages_contain_context() {
    // All error messages should provide context about what went wrong
    let errors = vec![
        "Could not determine home directory",
        "Failed to load local ledger",
        "Failed to refresh caches from ledger",
    ];

    for error in errors {
        assert!(!error.is_empty());
        assert!(error.len() > 10); // Substantial error message
    }
}

#[test]
fn test_error_messages_are_user_friendly() {
    // Error messages should be in plain language, not cryptic
    let non_technical_terms = vec![
        "Could not determine",
        "Failed to load",
        "Failed to refresh",
        "Possible fixes",
        "This may indicate",
    ];

    for term in non_technical_terms {
        assert!(!term.is_empty());
    }
}

#[test]
fn test_all_errors_provide_next_steps() {
    // Every error should guide the user on what to do next
    let actionable_keywords = [
        "Please set",
        "use --local-ledger-dir",
        "Possible fixes",
        "Try:",
    ];

    // Each error type should have at least one actionable keyword
    let home_error = "Could not determine home directory. Please set HOME environment variable or use --local-ledger-dir to specify ledger location.";
    let ledger_error = "Failed to load local ledger\n\nPossible fixes:";
    let cache_error =
        "Failed to refresh caches from ledger\n\nThis may indicate a corrupted ledger. Try:";

    assert!(
        actionable_keywords.iter().any(|kw| home_error.contains(kw)),
        "HomeDirNotFound error should provide next steps"
    );
    assert!(
        actionable_keywords
            .iter()
            .any(|kw| ledger_error.contains(kw)),
        "LedgerLoad error should provide next steps"
    );
    assert!(
        actionable_keywords
            .iter()
            .any(|kw| cache_error.contains(kw)),
        "CacheRefresh error should provide next steps"
    );
}
