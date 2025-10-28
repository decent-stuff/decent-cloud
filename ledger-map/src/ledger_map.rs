use crate::errors::LedgerError;
use crate::ledger_entry::{
    EntryKey, EntryValue, LedgerBlock, LedgerBlockHeader, LedgerEntry, Operation,
};
use crate::metadata::Metadata;
use crate::partition_table;
use crate::platform_specific::{
    persistent_storage_read, persistent_storage_size_bytes, persistent_storage_write,
};
use crate::{debug, info, warn};
use crate::{platform_specific, AHashSet};
use anyhow::Result;
use borsh::to_vec;
use indexmap::IndexMap;
use sha2::Digest;
use std::{cell::RefCell, mem::size_of};

// TODO: Make this configurable based on canister memory limits
const MAX_NEXT_BLOCK_SIZE_BYTES: usize = 2 * 1024 * 1024; // 2MB limit
const MAX_NEXT_BLOCK_ENTRIES: usize = 10000; // Safety limit for entry count

#[derive(Debug)]
pub struct LedgerMap {
    metadata: RefCell<Metadata>,
    labels_to_index: Option<AHashSet<String>>,
    entries: IndexMap<String, IndexMap<EntryKey, LedgerEntry>>,
    next_block_entries: IndexMap<String, IndexMap<EntryKey, LedgerEntry>>,
    current_timestamp_nanos: fn() -> u64,
}

impl Default for LedgerMap {
    fn default() -> Self {
        Self::new(None).unwrap()
    }
}

impl LedgerMap {
    /// Create a new LedgerMap instance.
    /// If `labels_to_index` is `None`, then all labels will be indexed.
    /// Note that iterating over non-indexed labels will not be possible through .iter()
    pub fn new(labels_to_index: Option<Vec<String>>) -> anyhow::Result<Self> {
        let mut result = LedgerMap {
            metadata: RefCell::new(Metadata::new()),
            labels_to_index: labels_to_index.map(AHashSet::from_iter),
            entries: IndexMap::new(),
            next_block_entries: IndexMap::new(),
            current_timestamp_nanos: platform_specific::get_timestamp_nanos,
        };
        result.refresh_ledger()?;
        Ok(result)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn new_with_path(
        labels_to_index: Option<Vec<String>>,
        path: Option<std::path::PathBuf>,
    ) -> anyhow::Result<Self> {
        platform_specific::set_backing_file(path).map_err(|e| anyhow::format_err!("{:?}", e))?;
        Self::new(labels_to_index)
    }

    #[cfg(all(target_arch = "wasm32", feature = "browser"))]
    pub fn new_with_path(
        labels_to_index: Option<Vec<String>>,
        _path: Option<std::path::PathBuf>,
    ) -> anyhow::Result<Self> {
        Self::new(labels_to_index)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn get_file_path(&self) -> Option<std::path::PathBuf> {
        platform_specific::get_backing_file_path()
    }

    #[cfg(all(target_arch = "wasm32", feature = "browser"))]
    pub fn get_file_path(&self) -> Option<std::path::PathBuf> {
        None
    }

    #[cfg(test)]
    fn with_timestamp_fn(self, get_timestamp_nanos: fn() -> u64) -> Self {
        LedgerMap {
            current_timestamp_nanos: get_timestamp_nanos,
            ..self
        }
    }

    pub fn begin_block(&mut self) -> anyhow::Result<()> {
        if !self.next_block_entries.is_empty() {
            return Err(anyhow::format_err!("There is already an open transaction."));
        }
        self.next_block_entries.clear();
        Ok(())
    }

    pub fn commit_block(&mut self) -> anyhow::Result<()> {
        if self.next_block_entries.is_empty() {
            // debug!("Commit of empty block invoked, skipping");
        } else {
            info!(
                "Commit non-empty block, with {} entries",
                self.next_block_entries.len()
            );
            let mut block_entries = Vec::new();
            for (label, values) in &self.next_block_entries {
                if match &self.labels_to_index {
                    Some(labels_to_index) => labels_to_index.contains(label),
                    None => true,
                } {
                    self.entries
                        .entry(label.clone())
                        .or_default()
                        .extend(values.clone())
                };
                for (_key, entry) in values {
                    block_entries.push(entry.clone());
                }
            }
            let block_timestamp = (self.current_timestamp_nanos)();
            let parent_hash = self.metadata.borrow().get_last_block_chain_hash().to_vec();
            let block = LedgerBlock::new(block_entries, block_timestamp, parent_hash);
            self._persist_block(block)?;
            self.next_block_entries.clear();
        }
        Ok(())
    }

    pub fn get<S: AsRef<str>>(&self, label: S, key: &[u8]) -> Result<EntryValue, LedgerError> {
        fn lookup<'a>(
            map: &'a IndexMap<String, IndexMap<EntryKey, LedgerEntry>>,
            label: &str,
            key: &[u8],
        ) -> Option<&'a LedgerEntry> {
            map.get(label)?.get(key)
        }

        let label_str = label.as_ref();
        for map in [&self.next_block_entries, &self.entries] {
            if let Some(entry) = lookup(map, label_str, key) {
                match entry.operation() {
                    Operation::Upsert => {
                        return Ok(entry.value().to_vec());
                    }
                    Operation::Delete => {
                        return Err(LedgerError::EntryNotFound);
                    }
                }
            }
        }

        Err(LedgerError::EntryNotFound)
    }

    pub fn count_entries_for_label<S: AsRef<str>>(&self, label: S) -> u64 {
        let label_str = label.as_ref();
        self.entries.get(label_str).map_or(0, |m| m.len() as u64)
            + self
                .next_block_entries
                .get(label_str)
                .map_or(0, |m| m.len() as u64)
    }

    pub fn upsert<S: AsRef<str>, K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &mut self,
        label: S,
        key: K,
        value: V,
    ) -> Result<(), LedgerError> {
        self._insert_entry_into_next_block(label, key, value, Operation::Upsert)
    }

    pub fn put<S: AsRef<str>, K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &mut self,
        label: S,
        key: K,
        value: V,
    ) -> Result<(), LedgerError> {
        self.upsert(label, key, value)
    }

    pub fn delete<S: AsRef<str>, K: AsRef<[u8]>>(
        &mut self,
        label: S,
        key: K,
    ) -> Result<(), LedgerError> {
        self._insert_entry_into_next_block(label, key, Vec::new(), Operation::Delete)
    }

    pub fn refresh_ledger(&mut self) -> anyhow::Result<()> {
        self.metadata.borrow_mut().clear();
        self.entries.clear();
        self.next_block_entries.clear();

        // If the backend is empty or non-existing, just return
        if persistent_storage_size_bytes() == 0 {
            warn!("Persistent storage is empty");
            return Ok(());
        }

        let data_part_entry = partition_table::get_data_partition();
        if persistent_storage_size_bytes() < data_part_entry.start_lba {
            warn!("No data found in persistent storage");
            return Ok(());
        }

        let mut expected_parent_hash = Vec::new();
        let mut updates = Vec::new();
        // Step 1: Read all Ledger Blocks
        for entry in self.iter_raw() {
            let (block_header, ledger_block) = entry?;

            if ledger_block.parent_hash() != expected_parent_hash {
                return Err(anyhow::format_err!(
                    "Hash mismatch: expected parent hash {:?}, got {:?}",
                    expected_parent_hash,
                    ledger_block.parent_hash()
                ));
            };

            let new_chain_hash = Self::_compute_block_chain_hash(
                ledger_block.parent_hash(),
                ledger_block.entries(),
                ledger_block.timestamp(),
            )?;

            let next_block_start_pos = self.metadata.borrow().next_block_start_pos()
                + block_header.jump_bytes_next_block() as u64;
            self.metadata.borrow_mut().update_from_appended_block(
                &new_chain_hash,
                ledger_block.timestamp(),
                next_block_start_pos,
            );
            expected_parent_hash = new_chain_hash;

            updates.push(ledger_block);
        }

        // Step 2: Add ledger entries into the index (self.entries) for quick search
        for ledger_block in updates {
            for ledger_entry in ledger_block.entries() {
                // Skip entries that are not in the labels_to_index
                if !match &self.labels_to_index {
                    Some(labels_to_index) => labels_to_index.contains(ledger_entry.label()),
                    None => true,
                } {
                    continue;
                }
                let entries = match self.entries.get_mut(ledger_entry.label()) {
                    Some(entries) => entries,
                    None => {
                        let new_map = IndexMap::new();
                        self.entries
                            .insert(ledger_entry.label().to_string(), new_map);
                        self.entries.get_mut(ledger_entry.label()).ok_or_else(|| {
                            anyhow::format_err!("Entry label {:?} not found", ledger_entry.label())
                        })?
                    }
                };

                match ledger_entry.operation() {
                    Operation::Upsert => {
                        entries.insert(ledger_entry.key().to_vec(), ledger_entry.clone());
                    }
                    Operation::Delete => {
                        entries.swap_remove(&ledger_entry.key().to_vec());
                    }
                }
            }
        }
        debug!("Ledger refreshed successfully");

        Ok(())
    }

    pub fn next_block_iter(&self, label: Option<&str>) -> impl Iterator<Item = &LedgerEntry> {
        match label {
            Some(label) => self
                .next_block_entries
                .get(label)
                .map(|entries| entries.values())
                .unwrap_or_default()
                .filter(|entry| entry.operation() == Operation::Upsert)
                .collect::<Vec<_>>()
                .into_iter(),
            None => self
                .next_block_entries
                .values()
                .flat_map(|entries| entries.values())
                .filter(|entry| entry.operation() == Operation::Upsert)
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }

    pub fn for_each<F>(&self, label: &str, mut f: F)
    where
        F: FnMut(&[u8], &[u8]),
    {
        if let Some(entries) = self.entries.get(label) {
            for (key, entry) in entries {
                f(key.as_slice(), entry.value());
            }
        }
        if let Some(entries) = self.next_block_entries.get(label) {
            for (key, entry) in entries {
                f(key.as_slice(), entry.value());
            }
        }
    }

    pub fn iter(&self, label: Option<&str>) -> impl Iterator<Item = &LedgerEntry> {
        match label {
            Some(label) => self
                .entries
                .get(label)
                .map(|entries| entries.values())
                .unwrap_or_default()
                .filter(|entry| entry.operation() == Operation::Upsert)
                .collect::<Vec<_>>()
                .into_iter(),
            None => self
                .entries
                .values()
                .flat_map(|entries| entries.values())
                .filter(|entry| entry.operation() == Operation::Upsert)
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }

    pub fn iter_raw(
        &self,
    ) -> impl Iterator<Item = anyhow::Result<(LedgerBlockHeader, LedgerBlock)>> + '_ {
        let data_start = partition_table::get_data_partition().start_lba;
        (0..).scan(data_start, |state, _| {
            let (block_header, ledger_block) = match self._persisted_block_read(*state) {
                Ok(decoded) => decoded,
                Err(LedgerError::BlockEmpty) => return None,
                Err(LedgerError::BlockCorrupted(err)) => {
                    return Some(Err(anyhow::format_err!(
                        "Failed to read Ledger block: {}",
                        err
                    )))
                }
                Err(err) => {
                    return Some(Err(anyhow::format_err!(
                        "Failed to read Ledger block: {}",
                        err
                    )))
                }
            };
            *state += block_header.jump_bytes_next_block() as u64;
            Some(Ok((block_header, ledger_block)))
        })
    }

    pub fn iter_raw_from_slice<'a>(
        &'a self,
        data: &'a [u8],
    ) -> impl Iterator<Item = anyhow::Result<(LedgerBlockHeader, LedgerBlock, Vec<u8>)>> + 'a {
        (0..).scan(0usize, move |offset, _| {
            // End iteration if the offset is at or past the end of the slice.
            if *offset >= data.len() {
                return None;
            }
            // Ensure there's at least enough bytes to read a header.
            if data.len() - *offset < LedgerBlockHeader::sizeof() {
                return None;
            }
            // Attempt to parse a block from the current offset.
            match self.get_block_from_slice(&data[*offset..]) {
                Ok((header, block, block_hash)) => {
                    let block_offset = *offset as u64;
                    let jump = header.jump_bytes_next_block() as usize;
                    // Avoid an infinite loop if jump is zero.
                    if jump == 0 {
                        return Some(Err(anyhow::format_err!("Block jump length is zero")));
                    }
                    *offset += jump;
                    Some(Ok((header, block.with_offset(block_offset), block_hash)))
                }
                Err(LedgerError::BlockEmpty) => {
                    // End iteration if a block is empty.
                    None
                }
                Err(err) => Some(Err(anyhow::format_err!(
                    "Failed to read Ledger block: {}",
                    err
                ))),
            }
        })
    }

    pub fn get_block_at_offset(
        &self,
        offset: u64,
    ) -> Result<(LedgerBlockHeader, LedgerBlock), LedgerError> {
        let offset = if offset < self.metadata.borrow().first_block_start_pos() {
            self.metadata.borrow().first_block_start_pos()
        } else {
            offset
        };
        self._persisted_block_read(offset)
    }

    pub fn get_block_from_slice(
        &self,
        data: &[u8],
    ) -> Result<(LedgerBlockHeader, LedgerBlock, Vec<u8>), LedgerError> {
        let header_size = LedgerBlockHeader::sizeof();
        if data.len() < header_size {
            return Err(LedgerError::BlockCorrupted("Block too short".to_string()));
        }
        let block_header = LedgerBlockHeader::deserialize(data)?;
        let end = block_header.jump_bytes_next_block() as usize;

        // Check if there's enough data for the block payload.
        if data.len() < end || end < header_size {
            return Err(LedgerError::BlockCorrupted("Block too short".to_string()));
        }

        let block =
            LedgerBlock::deserialize(&data[header_size..end], block_header.block_version())?;
        let block_hash = Self::_compute_block_chain_hash(
            block.parent_hash(),
            block.entries(),
            block.timestamp(),
        )
        .map_err(|e| LedgerError::BlockCorrupted(e.to_string()))?;
        Ok((block_header, block, block_hash))
    }

    pub fn get_blocks_count(&self) -> usize {
        self.metadata.borrow().num_blocks()
    }

    pub fn get_latest_block_start_pos(&self) -> u64 {
        self.metadata
            .borrow()
            .tip_block_start_pos()
            .unwrap_or_default()
    }

    pub fn get_latest_block_hash(&self) -> Vec<u8> {
        self.metadata.borrow().get_last_block_chain_hash().to_vec()
    }

    pub fn get_latest_block_timestamp_ns(&self) -> u64 {
        self.metadata.borrow().get_last_block_timestamp_ns()
    }

    pub fn get_next_block_start_pos(&self) -> u64 {
        self.metadata.borrow().next_block_start_pos()
    }

    pub fn get_next_block_entries_count(&self, label: Option<&str>) -> usize {
        match label {
            Some(label) => self.next_block_iter(Some(label)).count(),
            None => self.next_block_entries.values().map(|m| m.len()).sum(),
        }
    }

    /// Get pre-serialized data from the next block for fast sync
    pub fn get_next_block_serialized_data(&self) -> Vec<u8> {
        let mut serialized_data = Vec::new();
        for (_label, entries) in &self.next_block_entries {
            for (_key, entry) in entries {
                if let Some(serialized) = entry.get_serialized() {
                    serialized_data.extend_from_slice(&serialized);
                } else {
                    // Fallback: serialize on-demand if not already done
                    if let Ok(serialized) = entry.ensure_serialized() {
                        serialized_data.extend_from_slice(&serialized);
                    }
                }
            }
        }
        serialized_data
    }

    /// Calculate the current serialized size of all entries in the next block
    fn _get_next_block_serialized_size(&self) -> usize {
        self.next_block_entries
            .values()
            .flat_map(|entries| entries.values())
            .filter_map(|entry| entry.get_serialized())
            .map(|s| s.len())
            .sum()
    }

    /// Force commit current block (used during upgrades to prevent data loss)
    pub fn force_commit_block(&mut self) -> anyhow::Result<()> {
        if !self.next_block_entries.is_empty() {
            warn!("Force committing pending block during upgrade");
            self.commit_block()
        } else {
            Ok(())
        }
    }

    pub fn _compute_block_chain_hash(
        parent_block_hash: &[u8],
        block_entries: &[LedgerEntry],
        block_timestamp: u64,
    ) -> anyhow::Result<Vec<u8>> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(parent_block_hash);
        for entry in block_entries {
            hasher.update(to_vec(entry)?);
        }
        hasher.update(block_timestamp.to_le_bytes());
        Ok(hasher.finalize().to_vec())
    }

    fn _persist_block(&self, ledger_block: LedgerBlock) -> anyhow::Result<()> {
        let block_serialized_data = ledger_block.serialize()?;
        info!(
            "Appending block @timestamp {} with {} bytes data: {}",
            ledger_block.timestamp(),
            block_serialized_data.len(),
            ledger_block
        );
        // Prepare block header
        let jump_bytes_prev_block = (self
            .metadata
            .borrow()
            .tip_block_start_pos()
            .unwrap_or_default() as i64
            - self.metadata.borrow().next_block_start_pos() as i64)
            as i32;
        let jump_bytes_next_block =
            (block_serialized_data.len() + LedgerBlockHeader::sizeof()) as u32;
        let serialized_block_header =
            LedgerBlockHeader::new(jump_bytes_prev_block, jump_bytes_next_block).serialize()?;

        // First persist block header
        persistent_storage_write(
            self.metadata.borrow().next_block_start_pos(),
            &serialized_block_header,
        );

        // Then persist block data
        persistent_storage_write(
            self.metadata.borrow().next_block_start_pos() + LedgerBlockHeader::sizeof() as u64,
            &block_serialized_data,
        );

        let new_chain_hash = Self::_compute_block_chain_hash(
            ledger_block.parent_hash(),
            ledger_block.entries(),
            ledger_block.timestamp(),
        )?;
        let next_block_start_pos =
            self.metadata.borrow().next_block_start_pos() + jump_bytes_next_block as u64;
        self.metadata.borrow_mut().update_from_appended_block(
            &new_chain_hash,
            ledger_block.timestamp(),
            next_block_start_pos,
        );

        // Finally, persist LedgerBlockHeader number of bytes to mark the end of the block chain
        persistent_storage_write(
            self.metadata.borrow().next_block_start_pos(),
            &vec![0u8; size_of::<LedgerBlockHeader>()],
        );
        Ok(())
    }

    fn _persisted_block_read(
        &self,
        offset: u64,
    ) -> Result<(LedgerBlockHeader, LedgerBlock), LedgerError> {
        // Find out how many bytes we need to read ==> block len in bytes
        let mut buf = vec![0u8; size_of::<LedgerBlockHeader>()];
        persistent_storage_read(offset, &mut buf)
            .map_err(|e| LedgerError::BlockCorrupted(e.to_string()))?;

        let block_header = LedgerBlockHeader::deserialize(&buf)?;
        let block_len_bytes = block_header.jump_bytes_next_block();

        // Read the block as raw bytes
        let mut buf = vec![0u8; block_len_bytes as usize];
        persistent_storage_read(offset + LedgerBlockHeader::sizeof() as u64, &mut buf)
            .map_err(|e| LedgerError::Other(e.to_string()))?;

        let block = LedgerBlock::deserialize(&buf, block_header.block_version())
            .map_err(|err| LedgerError::BlockCorrupted(err.to_string()))?
            .with_offset(offset);

        Ok((block_header, block))
    }

    fn _insert_entry_into_next_block<S: AsRef<str>, K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &mut self,
        label: S,
        key: K,
        value: V,
        operation: Operation,
    ) -> Result<(), LedgerError> {
        let mut entry = LedgerEntry::new(label.as_ref(), key, value, operation);

        // Pre-serialize immediately to catch errors early
        let serialized =
            to_vec(&entry).map_err(|e| LedgerError::SerializationError(e.to_string()))?;

        // Check entry count limit
        let total_entries: usize = self.next_block_entries.values().map(|m| m.len()).sum();
        if total_entries >= MAX_NEXT_BLOCK_ENTRIES {
            return Err(LedgerError::TooManyEntriesInBlock(format!(
                "Block exceeds limit of {} entries",
                MAX_NEXT_BLOCK_ENTRIES
            )));
        }

        // Check serialized size limit
        let current_size = self._get_next_block_serialized_size();
        if current_size + serialized.len() > MAX_NEXT_BLOCK_SIZE_BYTES {
            return Err(LedgerError::BlockTooLarge(format!(
                "Block exceeds {} bytes limit",
                MAX_NEXT_BLOCK_SIZE_BYTES
            )));
        }

        // Store the serialized data in the entry for fast sync
        entry.set_serialized(serialized);

        // Update in-memory structures
        match self.next_block_entries.get_mut(entry.label()) {
            Some(entries) => {
                entries.insert(entry.key().to_vec(), entry);
            }
            None => {
                let mut new_map = IndexMap::new();
                new_map.insert(entry.key().to_vec(), entry);
                self.next_block_entries
                    .insert(label.as_ref().to_string(), new_map);
            }
        };

        Ok(())
    }
}

#[cfg(test)]
#[path = "ledger_map_tests.rs"]
mod ledger_map_tests;
