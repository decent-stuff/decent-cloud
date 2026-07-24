use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

const DEFAULT_LEDGER_DIR_NAME: &str = "decent-cloud-ledger";

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

#[cfg(test)]
fn env_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests;
