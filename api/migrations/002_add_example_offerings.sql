-- Migration for adding example offerings data

-- Create a special example provider using a distinctive hash
-- Using a distinctive 32-byte value that's clearly not a real pubkey
INSERT OR IGNORE INTO provider_registrations (
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
INSERT OR IGNORE INTO provider_profiles (
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
INSERT OR IGNORE INTO provider_offerings (
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
    1609459200000000000  -- 2021-01-01 00:00:00 UTC
);

-- Insert example offering 2: Premium Dedicated Server
INSERT OR IGNORE INTO provider_offerings (
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
    1609459200000000000  -- 2021-01-01 00:00:00 UTC
);

-- Add payment methods for example offering 1
INSERT OR IGNORE INTO provider_offerings_payment_methods (offering_id, payment_method)
SELECT id, 'Credit Card' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

INSERT OR IGNORE INTO provider_offerings_payment_methods (offering_id, payment_method)
SELECT id, 'PayPal' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

-- Add payment methods for example offering 2
INSERT OR IGNORE INTO provider_offerings_payment_methods (offering_id, payment_method)
SELECT id, 'BTC' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_payment_methods (offering_id, payment_method)
SELECT id, 'Bank Transfer' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_payment_methods (offering_id, payment_method)
SELECT id, 'Credit Card' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

-- Add features for example offering 1
INSERT OR IGNORE INTO provider_offerings_features (offering_id, feature)
SELECT id, 'Auto Backup' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

INSERT OR IGNORE INTO provider_offerings_features (offering_id, feature)
SELECT id, 'SSH Access' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

INSERT OR IGNORE INTO provider_offerings_features (offering_id, feature)
SELECT id, 'Root Access' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

-- Add features for example offering 2
INSERT OR IGNORE INTO provider_offerings_features (offering_id, feature)
SELECT id, 'RAID 1' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_features (offering_id, feature)
SELECT id, 'IPMI Access' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_features (offering_id, feature)
SELECT id, 'DDoS Protection' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_features (offering_id, feature)
SELECT id, '24/7 Support' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

-- Add operating systems for example offering 1
INSERT OR IGNORE INTO provider_offerings_operating_systems (offering_id, operating_system)
SELECT id, 'Ubuntu 22.04' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

INSERT OR IGNORE INTO provider_offerings_operating_systems (offering_id, operating_system)
SELECT id, 'Debian 11' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

INSERT OR IGNORE INTO provider_offerings_operating_systems (offering_id, operating_system)
SELECT id, 'CentOS 8' FROM provider_offerings WHERE offering_id = 'vm-basic-001';

-- Add operating systems for example offering 2
INSERT OR IGNORE INTO provider_offerings_operating_systems (offering_id, operating_system)
SELECT id, 'Ubuntu 22.04' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_operating_systems (offering_id, operating_system)
SELECT id, 'CentOS 8' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_operating_systems (offering_id, operating_system)
SELECT id, 'Windows Server 2022' FROM provider_offerings WHERE offering_id = 'ds-premium-002';

INSERT OR IGNORE INTO provider_offerings_operating_systems (offering_id, operating_system)
SELECT id, 'Debian 11' FROM provider_offerings WHERE offering_id = 'ds-premium-002';
