-- Complete database structure

-- Provider registrations
CREATE TABLE provider_registrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    pubkey_bytes BLOB NOT NULL,
    signature BLOB NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider check-ins
CREATE TABLE provider_check_ins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    memo TEXT NOT NULL,
    nonce_signature BLOB NOT NULL,
    block_timestamp_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider profiles (main table)
CREATE TABLE provider_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    website_url TEXT,
    logo_url TEXT,
    why_choose_us TEXT,
    api_version TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    updated_at_ns INTEGER NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider profile contacts (normalized table)
CREATE TABLE provider_profiles_contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_pubkey_hash BLOB NOT NULL,
    contact_type TEXT NOT NULL, -- email, phone, twitter, linkedin, etc.
    contact_value TEXT NOT NULL,
    FOREIGN KEY (provider_pubkey_hash) REFERENCES provider_profiles(pubkey_hash) ON DELETE CASCADE
);

-- Provider offerings (main table)
CREATE TABLE provider_offerings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    offering_id TEXT NOT NULL,
    offer_name TEXT NOT NULL,
    description TEXT,
    product_page_url TEXT,
    currency TEXT NOT NULL,
    monthly_price REAL NOT NULL,
    setup_fee REAL NOT NULL DEFAULT 0.0,
    visibility TEXT NOT NULL, -- public, private, limited
    product_type TEXT NOT NULL, -- dedicated_server, vps, cloud, etc.
    virtualization_type TEXT, -- kvm, xen, vmware, etc.
    billing_interval TEXT NOT NULL, -- monthly, yearly
    stock_status TEXT NOT NULL, -- in_stock, out_of_stock, limited
    processor_brand TEXT,
    processor_amount INTEGER,
    processor_cores INTEGER,
    processor_speed TEXT,
    processor_name TEXT,
    memory_error_correction TEXT, -- ecc, non_ecc
    memory_type TEXT,
    memory_amount TEXT,
    hdd_amount INTEGER DEFAULT 0,
    total_hdd_capacity TEXT,
    ssd_amount INTEGER DEFAULT 0,
    total_ssd_capacity TEXT,
    unmetered_bandwidth BOOLEAN DEFAULT FALSE,
    uplink_speed TEXT,
    traffic INTEGER, -- GB/TB per month
    datacenter_country TEXT NOT NULL,
    datacenter_city TEXT NOT NULL,
    datacenter_latitude REAL,
    datacenter_longitude REAL,
    control_panel TEXT,
    gpu_name TEXT,
    min_contract_hours INTEGER,
    max_contract_hours INTEGER,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    payment_methods TEXT,
    features TEXT,
    operating_systems TEXT,
    UNIQUE(pubkey_hash, offering_id)
);

-- User registrations
CREATE TABLE user_registrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    pubkey_bytes BLOB NOT NULL,
    signature BLOB NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Token transfers
CREATE TABLE token_transfers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_account TEXT NOT NULL,
    to_account TEXT NOT NULL,
    amount_e9s INTEGER NOT NULL,
    fee_e9s INTEGER NOT NULL DEFAULT 0,
    memo TEXT,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    block_hash BLOB,
    block_offset INTEGER
);

-- Token approvals
CREATE TABLE token_approvals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_account TEXT NOT NULL,
    spender_account TEXT NOT NULL,
    amount_e9s INTEGER NOT NULL,
    expires_at_ns INTEGER,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Contract sign requests
CREATE TABLE contract_sign_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL UNIQUE,
    requester_pubkey_hash BLOB NOT NULL,
    requester_ssh_pubkey TEXT NOT NULL,
    requester_contact TEXT NOT NULL,
    provider_pubkey_hash BLOB NOT NULL,
    offering_id TEXT NOT NULL,
    region_name TEXT,
    instance_config TEXT,
    payment_amount_e9s INTEGER NOT NULL,
    start_timestamp_ns INTEGER,
    end_timestamp_ns INTEGER,
    duration_hours INTEGER,
    original_duration_hours INTEGER,
    request_memo TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'pending', -- requested, pending, accepted, rejected, provisioning, provisioned, active, completed, cancelled
    status_updated_at_ns INTEGER,
    status_updated_by BLOB,
    provisioning_instance_details TEXT,
    provisioning_completed_at_ns INTEGER
);

-- Contract payment entries (properly normalized)
CREATE TABLE contract_payment_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    pricing_model TEXT NOT NULL, -- on_demand, reserved, spot
    time_period_unit TEXT NOT NULL, -- hour, day, month, year
    quantity INTEGER NOT NULL,
    amount_e9s INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE
);

-- Contract sign replies
CREATE TABLE contract_sign_replies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    provider_pubkey_hash BLOB NOT NULL,
    reply_status TEXT NOT NULL, -- accepted, rejected
    reply_memo TEXT,
    instance_details TEXT, -- connection details, IP addresses, etc.
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE
);

-- Contract extensions tracking
CREATE TABLE contract_extensions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    extended_by_pubkey BLOB NOT NULL,
    extension_hours INTEGER NOT NULL,
    extension_payment_e9s INTEGER NOT NULL,
    previous_end_timestamp_ns INTEGER NOT NULL,
    new_end_timestamp_ns INTEGER NOT NULL,
    extension_memo TEXT,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE
);

-- Reputation changes
CREATE TABLE reputation_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    change_amount INTEGER NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    block_timestamp_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Reputation aging records
CREATE TABLE reputation_aging (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_timestamp_ns INTEGER NOT NULL,
    aging_factor_ppm INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Reward distributions
CREATE TABLE reward_distributions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_timestamp_ns INTEGER NOT NULL,
    total_amount_e9s INTEGER NOT NULL,
    providers_count INTEGER NOT NULL,
    amount_per_provider_e9s INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Linked IC identities
CREATE TABLE linked_ic_ids (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    ic_principal TEXT NOT NULL,
    operation TEXT NOT NULL, -- add, remove
    linked_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Sync state tracking
CREATE TABLE sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_position INTEGER NOT NULL DEFAULT 0,
    last_sync_at DATETIME DEFAULT CURRENT_TIMESTAMP
);


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

-- Insert initial sync state
INSERT OR IGNORE INTO sync_state (id, last_position) VALUES (1, 0);

-- Add example offerings data

-- Create a special example provider using a distinctive hash
-- Using a distinctive 32-byte value that's clearly not a real pubkey
INSERT OR REPLACE INTO provider_registrations (
    pubkey_hash,
    pubkey_bytes,
    signature,
    created_at_ns
) VALUES (
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    1609459200000000000  -- 2021-01-01 00:00:00 UTC
);

-- Create example provider profile
INSERT OR REPLACE INTO provider_profiles (
    pubkey_hash,
    name,
    description,
    website_url,
    logo_url,
    why_choose_us,
    api_version,
    profile_version,
    updated_at_ns
) VALUES (
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'Example Provider',
    'Example provider for template and demonstration purposes',
    'https://example.com',
    'https://example.com/logo.png',
    'This is an example provider used to demonstrate the offerings system',
    '1.0.0',
    '1.0.0',
    1609459200000000000  -- 2021-01-01 00:00:00 UTC
);

-- Insert example offering 1: Basic VM
INSERT OR REPLACE INTO provider_offerings (
    pubkey_hash,
    offering_id,
    offer_name,
    description,
    product_page_url,
    currency,
    monthly_price,
    setup_fee,
    visibility,
    product_type,
    virtualization_type,
    billing_interval,
    stock_status,
    processor_brand,
    processor_amount,
    processor_cores,
    processor_speed,
    processor_name,
    memory_type,
    memory_amount,
    ssd_amount,
    total_ssd_capacity,
    unmetered_bandwidth,
    uplink_speed,
    datacenter_country,
    datacenter_city,
    datacenter_latitude,
    datacenter_longitude,
    control_panel,
    min_contract_hours,
    max_contract_hours,
    payment_methods,
    features,
    operating_systems,
    created_at_ns
) VALUES (
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'vm-basic-001',
    'Basic Virtual Machine',
    '2 vCPU 4GB RAM 80GB SSD - Perfect for small applications',
    'https://example.com/vm-basic',
    'USD',
    29.99,
    0.0,
    'example',
    'compute',
    'KVM',
    'monthly',
    'in_stock',
    'Intel',
    1,
    2,
    '2.4 GHz',
    'Intel Xeon E5-2680v4',
    'DDR4',
    '4GB',
    1,
    '80GB',
    TRUE,
    '1 Gbps',
    'US',
    'New York',
    40.7128,
    -74.0060,
    'cPanel',
    1,
    720,
    'Credit Card,PayPal',
    'Auto Backup,SSH Access,Root Access',
    'Ubuntu 22.04,Debian 11,CentOS 8',
    1609459200000000000  -- 2021-01-01 00:00:00 UTC
);

-- Insert example offering 2: Premium Dedicated Server
INSERT OR REPLACE INTO provider_offerings (
    pubkey_hash,
    offering_id,
    offer_name,
    description,
    product_page_url,
    currency,
    monthly_price,
    setup_fee,
    visibility,
    product_type,
    virtualization_type,
    billing_interval,
    stock_status,
    processor_brand,
    processor_amount,
    processor_cores,
    processor_speed,
    processor_name,
    memory_error_correction,
    memory_type,
    memory_amount,
    ssd_amount,
    total_ssd_capacity,
    unmetered_bandwidth,
    uplink_speed,
    datacenter_country,
    datacenter_city,
    datacenter_latitude,
    datacenter_longitude,
    control_panel,
    min_contract_hours,
    max_contract_hours,
    payment_methods,
    features,
    operating_systems,
    created_at_ns
) VALUES (
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'ds-premium-002',
    'Premium Dedicated Server',
    'Dual Xeon 128GB RAM 2x1TB NVMe - High performance dedicated hardware',
    'https://example.com/dedicated-premium',
    'USD',
    299.99,
    99.0,
    'example',
    'dedicated',
    'bare-metal',
    'monthly',
    'in_stock',
    'Intel',
    2,
    32,
    '3.2 GHz',
    'Intel Xeon Gold 6248R',
    'ECC',
    'DDR4 ECC',
    '128GB',
    2,
    '2TB',
    TRUE,
    '10 Gbps',
    'DE',
    'Frankfurt',
    50.1109,
    8.6821,
    'IPMI',
    24,
    720,
    'BTC,Bank Transfer,Credit Card',
    'RAID 1,IPMI Access,DDoS Protection,24/7 Support',
    'Ubuntu 22.04,CentOS 8,Windows Server 2022,Debian 11',
    1609459200000000000  -- 2021-01-01 00:00:00 UTC
);

-- Optimized indexes for efficient querying
CREATE INDEX idx_provider_registrations_pubkey_hash ON provider_registrations(pubkey_hash);
CREATE INDEX idx_provider_check_ins_pubkey_hash ON provider_check_ins(pubkey_hash);
CREATE INDEX idx_provider_check_ins_timestamp ON provider_check_ins(block_timestamp_ns);
CREATE INDEX idx_provider_profiles_pubkey_hash ON provider_profiles(pubkey_hash);
CREATE INDEX idx_provider_profiles_contacts_provider ON provider_profiles_contacts(provider_pubkey_hash);
CREATE INDEX idx_provider_profiles_contacts_type ON provider_profiles_contacts(contact_type);
CREATE INDEX idx_provider_offerings_pubkey_hash ON provider_offerings(pubkey_hash);
CREATE INDEX idx_provider_offerings_offering_id ON provider_offerings(offering_id);
CREATE INDEX idx_provider_offerings_visibility ON provider_offerings(visibility);
CREATE INDEX idx_provider_offerings_country ON provider_offerings(datacenter_country);
CREATE INDEX idx_provider_offerings_product_type ON provider_offerings(product_type);
CREATE INDEX idx_provider_offerings_stock ON provider_offerings(stock_status);
CREATE INDEX idx_token_transfers_from_account ON token_transfers(from_account);
CREATE INDEX idx_token_transfers_to_account ON token_transfers(to_account);
CREATE INDEX idx_token_transfers_timestamp ON token_transfers(created_at_ns);
CREATE INDEX idx_token_transfers_block_hash ON token_transfers(block_hash);
CREATE INDEX idx_token_approvals_owner_account ON token_approvals(owner_account);
CREATE INDEX idx_token_approvals_spender_account ON token_approvals(spender_account);
CREATE INDEX idx_contract_sign_requests_contract_id ON contract_sign_requests(contract_id);
CREATE INDEX idx_contract_sign_requests_requester_pubkey_hash ON contract_sign_requests(requester_pubkey_hash);
CREATE INDEX idx_contract_sign_requests_provider ON contract_sign_requests(provider_pubkey_hash);
CREATE INDEX idx_contract_sign_requests_status ON contract_sign_requests(status);
CREATE INDEX idx_contract_sign_requests_offering ON contract_sign_requests(offering_id);
CREATE INDEX idx_contract_payment_entries_contract_id ON contract_payment_entries(contract_id);
CREATE INDEX idx_contract_sign_replies_contract_id ON contract_sign_replies(contract_id);
CREATE INDEX idx_contract_extensions_contract_id ON contract_extensions(contract_id);
CREATE INDEX idx_contract_extensions_extended_by ON contract_extensions(extended_by_pubkey);
CREATE INDEX idx_reputation_changes_pubkey_hash ON reputation_changes(pubkey_hash);
CREATE INDEX idx_reputation_changes_timestamp ON reputation_changes(block_timestamp_ns);
CREATE INDEX idx_linked_ic_ids_pubkey_hash ON linked_ic_ids(pubkey_hash);
CREATE INDEX idx_linked_ic_ids_principal ON linked_ic_ids(ic_principal);
CREATE INDEX idx_linked_ic_ids_operation ON linked_ic_ids(operation);
CREATE INDEX idx_user_profiles_pubkey_hash ON user_profiles(pubkey_hash);
CREATE INDEX idx_user_contacts_pubkey_hash ON user_contacts(user_pubkey_hash);
CREATE INDEX idx_user_contacts_type ON user_contacts(contact_type);
CREATE INDEX idx_user_socials_pubkey_hash ON user_socials(user_pubkey_hash);
CREATE INDEX idx_user_socials_platform ON user_socials(platform);
CREATE INDEX idx_user_public_keys_pubkey_hash ON user_public_keys(user_pubkey_hash);
CREATE INDEX idx_user_public_keys_type ON user_public_keys(key_type);
CREATE INDEX idx_user_public_keys_fingerprint ON user_public_keys(key_fingerprint);
