-- Initial schema for ledger data synchronization
-- All ledger entry types are stored in a single table for simplicity

CREATE TABLE IF NOT EXISTS ledger_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL,
    key BLOB NOT NULL,
    value BLOB NOT NULL,
    block_hash BLOB,
    block_offset INTEGER,
    timestamp_ns INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(label, key)
);

CREATE INDEX idx_ledger_entries_label ON ledger_entries(label);
CREATE INDEX idx_ledger_entries_block_offset ON ledger_entries(block_offset);
CREATE INDEX idx_ledger_entries_timestamp ON ledger_entries(timestamp_ns);

-- Track sync cursor position
CREATE TABLE IF NOT EXISTS sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_position INTEGER NOT NULL DEFAULT 0,
    last_sync_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert initial sync state
INSERT OR IGNORE INTO sync_state (id, last_position) VALUES (1, 0);
