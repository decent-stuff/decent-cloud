/// This module contains functionalities specific to the x86_64/aarch64 architecture.
/// It includes implementations and optimizations tailored for this 64-bit environment,
/// allowing LedgerMap to run on x86_64/aarch64 platforms and share most of the code with
/// the wasm32 platform.
///
use std::io::{Read, Seek, SeekFrom, Write};

use fs_err::{File, OpenOptions};
pub use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::path::PathBuf;

pub struct BackingFile {
    file: File,
    file_path: PathBuf,
}

impl BackingFile {
    pub fn new(file_path: Option<PathBuf>) -> Result<Self, String> {
        let file_path = file_path.unwrap_or_else(default_file_path);
        fs_err::create_dir_all(file_path.parent().expect("Could not find parent directory"))
            .map_err(|e| format!("{:?}", e))?;

        debug!("Opening persistent storage {:?}", file_path);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_path)
            .map_err(|e| e.to_string())?;

        Ok(BackingFile { file, file_path })
    }

    pub fn metadata(&self) -> Result<std::fs::Metadata, String> {
        self.file
            .metadata()
            .map_err(|e| format!("Failed to retrieve metadata: {}", e))
    }

    pub fn read(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), String> {
        let file_size_bytes = self.metadata()?.len();
        debug!(
            "Reading from persistent storage {:?} @ 0x{:0x} .. 0x{:0x}",
            self.file_path,
            offset,
            offset + buf.len() as u64
        );

        if offset + buf.len() as u64 > file_size_bytes {
            return Err(
                "Failed to read from persistent storage: read beyond end of file.".to_string(),
            );
        }

        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        self.file.read_exact(buf).map_err(|e| e.to_string())?;
        debug!("Read bytes: {:?}", buf);
        Ok(())
    }

    pub fn write(&mut self, offset: u64, buf: &[u8]) -> Result<(), String> {
        let file_size_bytes = self.metadata()?.len();
        if file_size_bytes < offset + (buf.len() as u64).max(PERSISTENT_STORAGE_PAGE_SIZE) {
            let file_size_bytes_new = offset + (buf.len() as u64).max(PERSISTENT_STORAGE_PAGE_SIZE);
            self.file
                .set_len(file_size_bytes_new)
                .map_err(|e| e.to_string())?;
            // Fill new file space with zeros
            self.file
                .seek(SeekFrom::Start(file_size_bytes))
                .map_err(|e| e.to_string())?;
            self.file
                .write_all(&vec![0; (file_size_bytes_new - file_size_bytes) as usize])
                .map_err(|e| e.to_string())?;
            info!(
                "Growing persistent storage to {} bytes.",
                file_size_bytes_new
            );
        }

        debug!(
            "Writing {} bytes to persistent storage @offset 0x{:0x}",
            buf.len(),
            offset
        );

        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        self.file.write_all(buf).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn grow(&mut self, additional_pages: u64) -> Result<u64, String> {
        let previous_size_bytes = self.metadata()?.len();
        let new_size_bytes =
            previous_size_bytes + (additional_pages * PERSISTENT_STORAGE_PAGE_SIZE);
        println!(
            "Growing persistent storage from {} to {} bytes.",
            previous_size_bytes, new_size_bytes
        );
        // Ensure we are growing the file
        if new_size_bytes < previous_size_bytes {
            return Err(
                "New size is smaller than the current size. Cannot shrink file.".to_string(),
            );
        }
        // Attempt to set the new length of the file
        self.file.set_len(new_size_bytes).map_err(|e| {
            // Capture detailed error information
            let msg = match e.kind() {
                std::io::ErrorKind::InvalidInput => "Invalid input provided for file size.",
                std::io::ErrorKind::PermissionDenied => "Permission denied when resizing file.",
                std::io::ErrorKind::UnexpectedEof => "Unexpected end of file when resizing.",
                _ => "An unexpected error occurred.",
            };
            format!(
                "Failed to set file length: {} (err kind: {:?}, message: {})",
                e,
                e.kind(),
                msg
            )
        })?;

        Ok(previous_size_bytes)
    }
}

fn default_file_path() -> PathBuf {
    dirs::data_local_dir()
        .map(|path| path.join("ledger-map").join("data.bin"))
        .unwrap_or_else(|| PathBuf::from("data.bin"))
}

thread_local! {
    pub static BACKING_FILE: RefCell<Option<BackingFile>> = const { RefCell::new(None) };
}

pub fn set_backing_file(file_path: Option<PathBuf>) -> Result<(), String> {
    BACKING_FILE.with(|backing_file| {
        backing_file.replace(Some(BackingFile::new(file_path)?));
        Ok(())
    })
}

pub fn get_backing_file_path() -> Option<PathBuf> {
    BACKING_FILE.with(|backing_file| {
        backing_file
            .borrow()
            .as_ref()
            .map(|bf| bf.file_path.clone())
    })
}

pub fn get_or_create_backing_file() -> Result<BackingFile, String> {
    BACKING_FILE.with(|backing_file| {
        let mut binding = backing_file.borrow_mut();

        if binding.is_none() {
            // Initialize the backing file if it doesn't exist
            let new_file = BackingFile::new(None).map_err(|e| e.to_string())?;
            *binding = Some(new_file);
        }

        // Return a clone of the BackingFile reference
        binding
            .as_ref()
            .map(|bf| BackingFile {
                file: bf.file.try_clone().expect("Failed to clone file handle"),
                file_path: bf.file_path.clone(),
            })
            .ok_or_else(|| "Failed to access backing file".to_string())
    })
}

pub fn persistent_storage_size_bytes() -> u64 {
    BACKING_FILE.with(|backing_file| {
        backing_file
            .borrow()
            .as_ref()
            .and_then(|bf| bf.metadata().ok().map(|metadata| metadata.len()))
            .unwrap_or(0)
    })
}

pub fn persistent_storage_last_valid_offset() -> u64 {
    persistent_storage_size_bytes()
}

pub fn persistent_storage_read(offset: u64, buf: &mut [u8]) -> Result<(), String> {
    let mut backing_file = get_or_create_backing_file()?;
    backing_file.read(offset, buf)
}

pub fn persistent_storage_write(offset: u64, buf: &[u8]) {
    let mut backing_file = get_or_create_backing_file().expect("Backing file should exist");
    backing_file
        .write(offset, buf)
        .expect("Failed to write to persistent storage");
}

pub fn persistent_storage_grow(additional_pages: u64) -> Result<u64, String> {
    let mut backing_file = get_or_create_backing_file()?;
    backing_file.grow(additional_pages)
}

pub const PERSISTENT_STORAGE_PAGE_SIZE: u64 = 64 * 1024;

// These functions exist only for compatibility with the wasm32 implementation.
pub fn export_debug() -> Vec<String> {
    Vec::new()
}

pub fn export_info() -> Vec<String> {
    Vec::new()
}

pub fn export_warn() -> Vec<String> {
    Vec::new()
}

pub fn export_error() -> Vec<String> {
    Vec::new()
}

pub(crate) fn get_timestamp_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
