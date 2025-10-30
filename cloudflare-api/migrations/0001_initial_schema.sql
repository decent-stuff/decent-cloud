-- Decent Cloud Core Schema
-- Initial migration for Decent Cloud CF Service D1 database

-- Users table for Decent Cloud accounts
CREATE TABLE IF NOT EXISTS dc_users (
  id TEXT PRIMARY KEY,                    -- User ID (typically principal or derived identifier)
  pubkey TEXT UNIQUE NOT NULL,            -- Public key (hex string)
  principal TEXT,                         -- ICP principal (if applicable)
  reputation INTEGER NOT NULL DEFAULT 0,  -- Reputation score
  balance_tokens INTEGER NOT NULL DEFAULT 0, -- DC token balance
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Provider profiles table
CREATE TABLE IF NOT EXISTS provider_profiles (
  pubkey TEXT PRIMARY KEY,               -- Provider's public key (foreign key to dc_users)
  profile_data BLOB NOT NULL,            -- Serialized profile data (original format)
  signature BLOB,                        -- Profile signature
  version INTEGER NOT NULL DEFAULT 1,    -- Profile version for migration tracking
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (pubkey) REFERENCES dc_users(pubkey) ON DELETE CASCADE
);

-- Provider offerings table
CREATE TABLE IF NOT EXISTS provider_offerings (
  id TEXT PRIMARY KEY,                   -- Offering ID (hash of content)
  provider_pubkey TEXT NOT NULL,         -- Provider's public key
  offering_data BLOB NOT NULL,           -- Serialized offering data (original format)
  signature BLOB,                        -- Offering signature
  version INTEGER NOT NULL DEFAULT 1,    -- Offering version for migration tracking
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (provider_pubkey) REFERENCES dc_users(pubkey) ON DELETE CASCADE
);

-- Ledger entries table (append-only pattern matching ICP)
CREATE TABLE IF NOT EXISTS ledger_entries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  label TEXT NOT NULL,                   -- Entry label (e.g., 'ProvProfile', 'ProvOffering')
  key TEXT NOT NULL,                     -- Entry key (typically public key or hash)
  value BLOB NOT NULL,                   -- Serialized value data
  block_offset INTEGER NOT NULL,         -- ICP block offset for reference
  operation TEXT NOT NULL,               -- 'INSERT', 'UPDATE', or 'DELETE'
  migrated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  icp_timestamp INTEGER NOT NULL         -- Original ICP timestamp (nanoseconds)
);

-- Reputation changes table
CREATE TABLE IF NOT EXISTS reputation_changes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  target_pubkey TEXT NOT NULL,           -- User whose reputation changed
  change_amount INTEGER NOT NULL,        -- Reputation change amount (can be negative)
  reason TEXT,                           -- Reason for change
  block_offset INTEGER NOT NULL,         -- Reference block
  changed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  icp_timestamp INTEGER NOT NULL,        -- Original ICP timestamp
  FOREIGN KEY (target_pubkey) REFERENCES dc_users(pubkey) ON DELETE CASCADE
);

-- Token transfers table
CREATE TABLE IF NOT EXISTS token_transfers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  from_pubkey TEXT NOT NULL,             -- Sender public key
  to_pubkey TEXT NOT NULL,               -- Recipient public key
  amount INTEGER NOT NULL,               -- Transfer amount in smallest units
  memo TEXT,                             -- Transfer memo
  block_offset INTEGER NOT NULL,         -- Reference block
  transferred_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  icp_timestamp INTEGER NOT NULL,        -- Original ICP timestamp
  FOREIGN KEY (from_pubkey) REFERENCES dc_users(pubkey),
  FOREIGN KEY (to_pubkey) REFERENCES dc_users(pubkey)
);

-- Contract signatures table
CREATE TABLE IF NOT EXISTS contract_signatures (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  contract_id TEXT NOT NULL,             -- Contract identifier
  requester_pubkey TEXT NOT NULL,        -- Requester public key
  provider_pubkey TEXT NOT NULL,         -- Provider public key
  contract_data BLOB NOT NULL,           -- Serialized contract data
  signature BLOB,                        -- Contract signature
  status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'signed', 'rejected', 'expired'
  block_offset INTEGER NOT NULL,         -- Reference block
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  icp_timestamp INTEGER NOT NULL,        -- Original ICP timestamp
  FOREIGN KEY (requester_pubkey) REFERENCES dc_users(pubkey),
  FOREIGN KEY (provider_pubkey) REFERENCES dc_users(pubkey)
);

-- Sync status table for tracking migration progress
CREATE TABLE IF NOT EXISTS sync_status (
  table_name TEXT PRIMARY KEY,
  last_synced_block_offset INTEGER NOT NULL DEFAULT 0,
  last_synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  total_records_synced INTEGER NOT NULL DEFAULT 0,
  sync_errors INTEGER NOT NULL DEFAULT 0,
  last_error TEXT
);

-- Performance indexes for queries
CREATE INDEX IF NOT EXISTS idx_dc_users_pubkey ON dc_users(pubkey);
CREATE INDEX IF NOT EXISTS idx_dc_users_principal ON dc_users(principal);
CREATE INDEX IF NOT EXISTS idx_dc_users_reputation ON dc_users(reputation DESC);

CREATE INDEX IF NOT EXISTS idx_provider_profiles_pubkey ON provider_profiles(pubkey);
CREATE INDEX IF NOT EXISTS idx_provider_profiles_updated ON provider_profiles(updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_provider_offerings_provider ON provider_offerings(provider_pubkey);
CREATE INDEX IF NOT EXISTS idx_provider_offerings_active ON provider_offerings(is_active, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_provider_offerings_id ON provider_offerings(id);

CREATE INDEX IF NOT EXISTS idx_ledger_entries_label_key ON ledger_entries(label, key);
CREATE INDEX IF NOT EXISTS idx_ledger_entries_block_offset ON ledger_entries(block_offset DESC);
CREATE INDEX IF NOT EXISTS idx_ledger_entries_migrated ON ledger_entries(migrated_at DESC);

CREATE INDEX IF NOT EXISTS idx_reputation_changes_target ON reputation_changes(target_pubkey);
CREATE INDEX IF NOT EXISTS idx_reputation_changes_block ON reputation_changes(block_offset DESC);

CREATE INDEX IF NOT EXISTS idx_token_transfers_from ON token_transfers(from_pubkey, transferred_at DESC);
CREATE INDEX IF NOT EXISTS idx_token_transfers_to ON token_transfers(to_pubkey, transferred_at DESC);
CREATE INDEX IF NOT EXISTS idx_token_transfers_block ON token_transfers(block_offset DESC);

CREATE INDEX IF NOT EXISTS idx_contract_signatures_contract ON contract_signatures(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_signatures_requester ON contract_signatures(requester_pubkey);
CREATE INDEX IF NOT EXISTS idx_contract_signatures_provider ON contract_signatures(provider_pubkey);
CREATE INDEX IF NOT EXISTS idx_contract_signatures_status ON contract_signatures(status, updated_at DESC);

-- Initialize sync status for core tables
INSERT OR IGNORE INTO sync_status (table_name, last_synced_block_offset) VALUES
('dc_users', 0),
('provider_profiles', 0),
('provider_offerings', 0),
('ledger_entries', 0),
('reputation_changes', 0),
('token_transfers', 0),
('contract_signatures', 0);