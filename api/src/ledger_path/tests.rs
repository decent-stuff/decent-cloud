use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn uses_env_var_when_present() {
    let _guard = env_lock().lock().unwrap();
    let temp_dir = tempdir().unwrap();
    let custom_path = temp_dir.keep();

    env::set_var("LEDGER_DIR", custom_path.to_str().unwrap());
    let path = ledger_dir_path().unwrap();

    assert_eq!(path, custom_path);
    assert!(path.exists());

    env::remove_var("LEDGER_DIR");
    fs::remove_dir_all(path).ok();
}

#[test]
fn creates_fallback_dir_when_env_missing() {
    let _guard = env_lock().lock().unwrap();
    env::remove_var("LEDGER_DIR");

    let path = ledger_dir_path().unwrap();
    assert!(path.ends_with(DEFAULT_LEDGER_DIR_NAME));
    assert!(path.exists());

    env::remove_var("LEDGER_DIR");
    fs::remove_dir_all(&path).ok();
}
