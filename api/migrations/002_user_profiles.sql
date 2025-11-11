-- User profiles extension
-- Extends the basic user_registrations table with profile information

-- User profiles (main table for user display information)
CREATE TABLE user_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    updated_at_ns INTEGER NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES user_registrations(pubkey_hash) ON DELETE CASCADE
);

-- User contacts (email, phone, etc.)
CREATE TABLE user_contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_pubkey_hash BLOB NOT NULL,
    contact_type TEXT NOT NULL, -- email, phone, telegram, etc.
    contact_value TEXT NOT NULL,
    verified BOOLEAN DEFAULT FALSE,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_pubkey_hash) REFERENCES user_registrations(pubkey_hash) ON DELETE CASCADE
);

-- User social media accounts
CREATE TABLE user_socials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_pubkey_hash BLOB NOT NULL,
    platform TEXT NOT NULL, -- twitter, github, discord, linkedin, etc.
    username TEXT NOT NULL,
    profile_url TEXT,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_pubkey_hash) REFERENCES user_registrations(pubkey_hash) ON DELETE CASCADE
);

-- User additional public keys (SSH, GPG, etc.)
CREATE TABLE user_public_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_pubkey_hash BLOB NOT NULL,
    key_type TEXT NOT NULL, -- ssh-ed25519, ssh-rsa, gpg, secp256k1, etc.
    key_data TEXT NOT NULL, -- The actual public key
    key_fingerprint TEXT, -- Optional fingerprint for quick identification
    label TEXT, -- User-provided label for this key
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_pubkey_hash) REFERENCES user_registrations(pubkey_hash) ON DELETE CASCADE
);

-- Indexes for efficient querying
CREATE INDEX idx_user_profiles_pubkey_hash ON user_profiles(pubkey_hash);
CREATE INDEX idx_user_contacts_pubkey_hash ON user_contacts(user_pubkey_hash);
CREATE INDEX idx_user_contacts_type ON user_contacts(contact_type);
CREATE INDEX idx_user_socials_pubkey_hash ON user_socials(user_pubkey_hash);
CREATE INDEX idx_user_socials_platform ON user_socials(platform);
CREATE INDEX idx_user_public_keys_pubkey_hash ON user_public_keys(user_pubkey_hash);
CREATE INDEX idx_user_public_keys_type ON user_public_keys(key_type);
CREATE INDEX idx_user_public_keys_fingerprint ON user_public_keys(key_fingerprint);
