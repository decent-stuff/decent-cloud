#[allow(unused_imports)]
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
use ic_cdk::println;

use serde::Serialize;
use std::mem::size_of;

use crate::platform_specific::{
    persistent_storage_grow, persistent_storage_read, persistent_storage_size_bytes,
    persistent_storage_write, PERSISTENT_STORAGE_PAGE_SIZE,
};
use crate::{debug, info};

pub const PARTITION_TABLE_START_OFFSET: u64 = 0;
pub const PARTITION_TABLE_MAX_ENTRIES: usize = 128;
const EXPECTED_MAGIC_BYTES: [u8; 8] = [0x4c, 0x65, 0x64, 0x67, 0x50, 0x61, 0x72, 0x74]; // "LedgPart"

#[derive(Serialize, Clone, Debug)]
pub struct PartitionTableHeader {
    pub magic_bytes: [u8; 8],
}

impl Default for PartitionTableHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl PartitionTableHeader {
    pub fn new() -> Self {
        PartitionTableHeader {
            magic_bytes: EXPECTED_MAGIC_BYTES,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != Self::size() {
            return Err(format!(
                "Header size mismatch: expected {}, got {}",
                Self::size(),
                bytes.len()
            ));
        }
        let mut header = PartitionTableHeader::new();
        header.magic_bytes.copy_from_slice(&bytes[0..8]);
        header.check_magic_bytes()?;
        Ok(header)
    }

    pub fn size() -> usize {
        size_of::<PartitionTableHeader>()
    }

    pub fn check_magic_bytes(&self) -> Result<(), String> {
        if self.magic_bytes != EXPECTED_MAGIC_BYTES {
            return Err(format!(
                "Header magic bytes mismatch: got {:?}, expected {:?}.",
                self.magic_bytes, EXPECTED_MAGIC_BYTES
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
pub struct PartitionTableEntry {
    pub name: [u8; 8],
    pub start_lba: u64,
}

impl PartitionTableEntry {
    pub fn new(name: &[u8], start_lba: u64) -> Self {
        let mut name_array = [0u8; 8];
        let len = name.len().min(8);
        name_array[..len].copy_from_slice(&name[..len]);

        PartitionTableEntry {
            name: name_array,
            start_lba,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != Self::size() {
            return Err(format!(
                "Entry size mismatch: expected {}, got {}",
                Self::size(),
                bytes.len()
            ));
        }
        Ok(PartitionTableEntry {
            name: bytes[0..8]
                .try_into()
                .map_err(|_| "Slice to array conversion failed for bytes 0..7")?,
            start_lba: u64::from_le_bytes(
                bytes[8..16]
                    .try_into()
                    .map_err(|_| "Slice to array conversion failed for bytes 8..16")?,
            ),
        })
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&self.name);
        bytes[8..16].copy_from_slice(&self.start_lba.to_le_bytes());
        bytes
    }

    pub fn size() -> usize {
        size_of::<PartitionTableEntry>()
    }

    pub fn is_used(&self) -> bool {
        self.start_lba != 0 || self.name != [0u8; 8]
    }
}

impl std::fmt::Display for PartitionTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, start_lba: {}",
            String::from_utf8_lossy(&self.name),
            self.start_lba
        )
    }
}

#[derive(Clone, Debug)]
pub struct PartitionTable {
    pub num_entries: u16,
    pub header: PartitionTableHeader,
    pub entries: Vec<PartitionTableEntry>,
}

impl Default for PartitionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PartitionTable {
    pub fn new() -> Self {
        let mut table = PartitionTable {
            num_entries: 0,
            header: PartitionTableHeader::new(),
            entries: Vec::with_capacity(PARTITION_TABLE_MAX_ENTRIES),
        };
        table
            .add_new_entry(PartitionTableEntry::new(
                b"PARTTABL",
                PartitionTableHeader::size() as u64,
            ))
            .unwrap();
        table
            .add_new_entry(PartitionTableEntry::new(b"DATA", 8 * 1024 * 1024))
            .unwrap();
        table
    }

    pub fn size() -> usize {
        PartitionTableHeader::size() + PARTITION_TABLE_MAX_ENTRIES * PartitionTableEntry::size()
    }

    pub fn required_size_bytes() -> u64 {
        Self::size() as u64
    }

    pub fn read_from_persistent_storage() -> Result<Self, String> {
        Self::ensure_enough_persistent_storage_allocated()?;

        if persistent_storage_size_bytes() < Self::required_size_bytes() {
            return Err("Not enough persistent storage allocated".to_string());
        }

        debug!(
            "Reading from persistent storage of size {} bytes",
            persistent_storage_size_bytes()
        );

        let mut buf = vec![0; Self::size()];
        persistent_storage_read(PARTITION_TABLE_START_OFFSET, &mut buf)?;

        let header = PartitionTableHeader::from_bytes(&buf[..PartitionTableHeader::size()])?;
        let mut entries = Vec::new();
        let mut num_entries = 0;

        for i in 0..PARTITION_TABLE_MAX_ENTRIES {
            let entry_offset = PartitionTableHeader::size() + i * PartitionTableEntry::size();
            let entry = PartitionTableEntry::from_bytes(
                &buf[entry_offset..entry_offset + PartitionTableEntry::size()],
            )?;

            if entries.is_empty() || entry.is_used() {
                entries.push(entry);
                num_entries += 1;
            } else {
                break;
            }
        }

        Ok(PartitionTable {
            num_entries,
            header,
            entries,
        })
    }

    pub fn persist(&self) -> Result<(), String> {
        if self.num_entries == 0 {
            return Err("Partition table is empty".to_string());
        }
        Self::ensure_enough_persistent_storage_allocated()?;

        let mut buf = vec![0; Self::size()];
        buf[..PartitionTableHeader::size()].copy_from_slice(&self.header.magic_bytes);
        for (i, entry) in self.entries.iter().enumerate() {
            let offset = PartitionTableHeader::size() + i * PartitionTableEntry::size();
            buf[offset..offset + PartitionTableEntry::size()].copy_from_slice(&entry.to_bytes());
        }

        persistent_storage_write(PARTITION_TABLE_START_OFFSET, &buf);
        info!(
            "Wrote {} bytes of partition table to persistent storage at LBA {}",
            buf.len(),
            PARTITION_TABLE_START_OFFSET
        );
        Ok(())
    }

    pub fn add_new_entry(&mut self, entry: PartitionTableEntry) -> Result<(), String> {
        if self.num_entries as usize >= PARTITION_TABLE_MAX_ENTRIES {
            return Err("Partition table full".to_string());
        }
        self.entries.push(entry);
        self.num_entries += 1;
        Ok(())
    }

    pub fn ensure_enough_persistent_storage_allocated() -> Result<(), String> {
        let size_min = Self::required_size_bytes();
        let size_bytes = persistent_storage_size_bytes();
        if size_bytes >= size_min {
            return Ok(());
        }
        let new_pages = (size_min - size_bytes) / PERSISTENT_STORAGE_PAGE_SIZE + 1;

        if new_pages > 0 {
            persistent_storage_grow(new_pages).expect("Failed to grow persistent storage");
            let persistent_storage_bytes_after = persistent_storage_size_bytes();
            info!(
                "Persistent storage resized to bytes: {}",
                persistent_storage_bytes_after
            );
            let table = PartitionTable::new();
            table.persist().unwrap();
        } else {
            info!("Persistent storage is sufficiently large");
        }
        Ok(())
    }
}

impl std::fmt::Display for PartitionTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Partition table: num_entries: {}, header: {}",
            self.num_entries,
            if self.header.magic_bytes == EXPECTED_MAGIC_BYTES {
                "VALID"
            } else {
                "INVALID"
            }
        )?;
        for entry in &self.entries {
            if !entry.is_used() {
                break;
            }
            write!(f, "\n\t{}", entry)?;
        }
        Ok(())
    }
}

pub fn get_partition_table() -> PartitionTable {
    PartitionTable::read_from_persistent_storage().unwrap_or_default()
}

pub fn get_data_partition() -> PartitionTableEntry {
    let table = get_partition_table();
    *table
        .entries
        .get(PART_DATA)
        .expect("Data partition not found")
}

pub const PART_RESERVED: usize = 0;
pub const PART_DATA: usize = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_table_header_default() {
        let header = PartitionTableHeader::new();
        assert_eq!(header.magic_bytes, EXPECTED_MAGIC_BYTES);
    }

    #[test]
    fn test_partition_table_entry_serialization() {
        let entry = PartitionTableEntry::new(b"TESTPART", 0);
        let bytes = entry.to_bytes();
        let deserialized_entry = PartitionTableEntry::from_bytes(&bytes).unwrap();
        assert_eq!(entry, deserialized_entry);
    }

    #[test]
    fn test_persistent_storage_read_and_write() {
        let file_path = tempfile::tempdir()
            .unwrap()
            .into_path()
            .join("test_ledger_store.bin");
        crate::platform_specific::set_backing_file(Some(file_path)).unwrap();

        let mut table = PartitionTable::new();
        let entry = PartitionTableEntry::new(b"TESTPART", 0);
        table.add_new_entry(entry).unwrap();
        table.persist().unwrap();

        let read_table = PartitionTable::read_from_persistent_storage().unwrap();
        assert_eq!(table.header.magic_bytes, read_table.header.magic_bytes);
        assert_eq!(table.num_entries, read_table.num_entries);
        assert_eq!(table.entries, read_table.entries);
    }

    #[test]
    fn test_get_data_partition() {
        let entry = get_data_partition();
        let mut label = b"DATA".to_vec();
        label.resize(8, 0);
        assert_eq!(entry.name.to_vec(), label);
    }
}
