-- Seed data for example provider and offerings
-- This data helps with template generation and marketplace demonstration

-- Example provider pubkey (hex: "example-offering-provider-identifier")
-- This is a distinctive 32-byte value that's clearly not a real pubkey

-- Register example provider
INSERT INTO provider_registrations (pubkey, signature, created_at_ns)
VALUES (
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    1609459200000000000
) ON CONFLICT (pubkey) DO NOTHING;

-- Create example provider profile
INSERT INTO provider_profiles (pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns)
VALUES (
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'Example Provider',
    'Example provider for template and demonstration purposes',
    'https://example.com',
    'https://example.com/logo.png',
    'This is an example provider used to demonstrate the offerings system',
    '1.0.0',
    '1.0.0',
    1609459200000000000
) ON CONFLICT (pubkey) DO NOTHING;

-- Create agent pools for example provider
INSERT INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns)
VALUES
    ('example-na', E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', 'Example NA Pool', 'na', 'manual', 0),
    ('example-europe', E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', 'Example Europe Pool', 'europe', 'manual', 0),
    ('example-apac', E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', 'Example APAC Pool', 'apac', 'manual', 0)
ON CONFLICT (pool_id) DO NOTHING;

-- Create agent delegations for each pool
INSERT INTO provider_agent_delegations (provider_pubkey, agent_pubkey, permissions, expires_at_ns, label, signature, created_at_ns, pool_id)
VALUES
    (E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', E'\\x6578616d706c652d6167656e742d6e612d0000000000000000000000000000000000', '[]', NULL, 'Example NA Agent', E'\\x00', 0, 'example-na'),
    (E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', E'\\x6578616d706c652d6167656e742d65752d0000000000000000000000000000000000', '[]', NULL, 'Example EU Agent', E'\\x00', 0, 'example-europe'),
    (E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572', E'\\x6578616d706c652d6167656e742d61702d0000000000000000000000000000000000', '[]', NULL, 'Example APAC Agent', E'\\x00', 0, 'example-apac')
ON CONFLICT (agent_pubkey) DO NOTHING;

-- Mark example provider as online
INSERT INTO provider_agent_status (provider_pubkey, online, last_heartbeat_ns, updated_at_ns)
VALUES (
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    TRUE,
    (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
) ON CONFLICT (provider_pubkey) DO UPDATE SET
    online = excluded.online,
    last_heartbeat_ns = excluded.last_heartbeat_ns,
    updated_at_ns = excluded.updated_at_ns;

-- Example compute offerings
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee,
    visibility, product_type, virtualization_type, billing_interval, stock_status,
    processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
    memory_type, memory_amount, ssd_amount, total_ssd_capacity,
    unmetered_bandwidth, uplink_speed, traffic,
    datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
    control_panel, min_contract_hours, max_contract_hours,
    payment_methods, features, operating_systems, created_at_ns
) VALUES
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'compute-001', 'Basic VPS', 'Ideal for small websites and development environments', NULL,
    'ICP', 5.0, 0.0, 'public', 'compute', 'KVM', 'monthly', 'in_stock',
    'AMD', 1, 2, '3.5 GHz', 'EPYC 7763', 'DDR4', '4 GB', 1, '50 GB',
    FALSE, '1 Gbps', 1000, 'US', 'New York', 40.7128, -74.0060,
    'cPanel', 720, 8760, 'ICP,ckBTC', 'Daily Backups,DDoS Protection,Root Access', 'Ubuntu 22.04,Debian 12,Rocky Linux 9', 1700000000000000000
),
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'compute-002', 'Performance VPS', 'High-performance compute for demanding applications', NULL,
    'ICP', 15.0, 0.0, 'public', 'compute', 'KVM', 'monthly', 'in_stock',
    'Intel', 1, 4, '4.0 GHz', 'Xeon E-2388G', 'DDR4', '8 GB', 1, '100 GB',
    FALSE, '1 Gbps', 2000, 'DE', 'Frankfurt', 50.1109, 8.6821,
    'Plesk', 720, 8760, 'ICP,ckBTC,ckETH', 'NVMe Storage,Auto-scaling,99.9% Uptime SLA', 'Ubuntu 22.04,Windows Server 2022', 1700000000000000000
);

-- Example GPU offerings
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee,
    visibility, product_type, virtualization_type, billing_interval, stock_status,
    processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
    memory_type, memory_amount, ssd_amount, total_ssd_capacity,
    unmetered_bandwidth, uplink_speed, traffic,
    datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
    gpu_name, gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours,
    payment_methods, features, operating_systems, created_at_ns
) VALUES
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'gpu-001', 'AI Training - RTX 4090', 'Perfect for deep learning and AI model training', NULL,
    'ICP', 150.0, 10.0, 'public', 'gpu', 'Bare Metal', 'monthly', 'in_stock',
    'AMD', 1, 16, '3.7 GHz', 'Ryzen 9 7950X', 'DDR5', '64 GB', 1, '1 TB',
    FALSE, '10 Gbps', 10000, 'US', 'San Francisco', 37.7749, -122.4194,
    'NVIDIA RTX 4090', 1, 24, 168, 8760,
    'ICP,ckBTC', 'CUDA 12.0,PyTorch,TensorFlow Pre-installed,Jupyter Notebook', 'Ubuntu 22.04 LTS,Rocky Linux 9', 1700000000000000000
),
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'gpu-002', 'Multi-GPU Workstation - A100', 'Enterprise-grade AI infrastructure with multiple A100 GPUs', NULL,
    'ICP', 800.0, 50.0, 'public', 'gpu', 'Bare Metal', 'monthly', 'in_stock',
    'AMD', 2, 64, '2.9 GHz', 'EPYC 7763', 'DDR4', '512 GB', 2, '4 TB',
    TRUE, '25 Gbps', NULL, 'SG', 'Singapore', 1.3521, 103.8198,
    'NVIDIA A100', 4, 320, 720, 8760,
    'ICP,ckBTC', 'NVLink,InfiniBand,Professional Support,99.95% SLA', 'Ubuntu 22.04 LTS', 1700000000000000000
);

-- Example storage offerings
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee,
    visibility, product_type, billing_interval, stock_status,
    hdd_amount, total_hdd_capacity, ssd_amount, total_ssd_capacity,
    unmetered_bandwidth, uplink_speed, traffic,
    datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
    control_panel, min_contract_hours, max_contract_hours,
    payment_methods, features, created_at_ns
) VALUES
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'storage-001', 'Object Storage Standard', 'Scalable cloud storage for backups and archives', NULL,
    'ICP', 2.5, 0.0, 'public', 'storage', 'monthly', 'in_stock',
    4, '4 TB', NULL, NULL, FALSE, '1 Gbps', 500,
    'US', 'Dallas', 32.7767, -96.7970,
    'S3-Compatible API', 720, 8760,
    'ICP,ckBTC', 'S3 API,Versioning,Lifecycle Policies,Encryption at Rest', 1700000000000000000
),
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'storage-002', 'High-Performance NVMe Storage', 'Ultra-fast NVMe storage for database workloads', NULL,
    'ICP', 20.0, 0.0, 'public', 'storage', 'monthly', 'in_stock',
    NULL, NULL, 2, '2 TB', FALSE, '10 Gbps', 2000,
    'NL', 'Amsterdam', 52.3676, 4.9041,
    'Block Storage API', 720, 8760,
    'ICP,ckBTC,ckETH', 'NVMe Gen4,Snapshots,RAID 10,99.99% Availability', 1700000000000000000
);

-- Example network offerings
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee,
    visibility, product_type, billing_interval, stock_status,
    unmetered_bandwidth, uplink_speed, traffic,
    datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
    control_panel, min_contract_hours, max_contract_hours,
    payment_methods, features, created_at_ns
) VALUES
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'network-001', 'CDN Basic', 'Global content delivery network for fast website performance', NULL,
    'ICP', 10.0, 0.0, 'public', 'network', 'monthly', 'in_stock',
    FALSE, '10 Gbps', 5000, 'US', 'Multiple POPs', NULL, NULL,
    'REST API', 720, 8760,
    'ICP,ckBTC', '50+ Edge Locations,SSL/TLS,DDoS Protection,Real-time Analytics', 1700000000000000000
),
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'network-002', 'Dedicated Bandwidth', 'Guaranteed bandwidth for high-traffic applications', NULL,
    'ICP', 50.0, 25.0, 'public', 'network', 'monthly', 'in_stock',
    TRUE, '10 Gbps', NULL, 'JP', 'Tokyo', 35.6762, 139.6503,
    NULL, 720, 8760,
    'ICP,ckBTC', 'Burstable to 20 Gbps,Low Latency,BGP Sessions,99.9% Uptime', 1700000000000000000
);

-- Example dedicated server offerings
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee,
    visibility, product_type, virtualization_type, billing_interval, stock_status,
    processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
    memory_error_correction, memory_type, memory_amount,
    hdd_amount, total_hdd_capacity, ssd_amount, total_ssd_capacity,
    unmetered_bandwidth, uplink_speed, traffic,
    datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
    control_panel, min_contract_hours, max_contract_hours,
    payment_methods, features, operating_systems, created_at_ns
) VALUES
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'dedicated-001', 'Entry Dedicated Server', 'Affordable dedicated server for small businesses', NULL,
    'ICP', 75.0, 25.0, 'public', 'dedicated', 'Bare Metal', 'monthly', 'in_stock',
    'Intel', 1, 8, '3.4 GHz', 'Xeon E-2388G', 'ECC', 'DDR4', '32 GB',
    2, '2 TB', 2, '480 GB', FALSE, '1 Gbps', 20000,
    'FR', 'Paris', 48.8566, 2.3522,
    'IPMI', 720, 8760,
    'ICP,ckBTC', 'Full Root Access,RAID 1,Remote Reboot,24/7 Support', 'Ubuntu 22.04,Debian 12,CentOS 9,Windows Server 2022', 1700000000000000000
),
(
    E'\\x6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'dedicated-002', 'Enterprise Dedicated Server', 'High-performance dedicated server for enterprise workloads', NULL,
    'ICP', 250.0, 50.0, 'public', 'dedicated', 'Bare Metal', 'monthly', 'in_stock',
    'AMD', 2, 128, '2.45 GHz', 'EPYC 7763', 'ECC', 'DDR4', '512 GB',
    NULL, NULL, 4, '8 TB', TRUE, '10 Gbps', NULL,
    'CA', 'Toronto', 43.6532, -79.3832,
    'IPMI,ILO', 720, 8760,
    'ICP,ckBTC,ckETH', 'NVMe RAID 10,Redundant PSU,IPMI,Premium Support,99.99% SLA', 'Ubuntu 22.04,RHEL 9,Windows Server 2022', 1700000000000000000
);
