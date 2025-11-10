use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const DEFAULT_LEDGER_DIR_NAME: &str = "decent-cloud-ledger";
const LEDGER_FILE_NAME: &str = "main.bin";

pub fn ledger_dir_path() -> io::Result<PathBuf> {
    if let Ok(path) = env::var("LEDGER_DIR") {
        let path_buf = PathBuf::from(path);
        fs::create_dir_all(&path_buf)?;
        return Ok(path_buf);
    }

    let fallback = env::temp_dir().join(DEFAULT_LEDGER_DIR_NAME);
    fs::create_dir_all(&fallback)?;
    if let Some(as_str) = fallback.to_str() {
        env::set_var("LEDGER_DIR", as_str);
    }
    Ok(fallback)
}

pub fn ledger_file_path() -> io::Result<PathBuf> {
    Ok(ledger_dir_path()?.join(LEDGER_FILE_NAME))
}

#[cfg(test)]
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod tests {
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
}
