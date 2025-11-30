#!/bin/bash
# Seed E2E test data for Search DSL testing
# Run this from the website directory: ./tests/e2e/seed-e2e-test-data.sh

set -e

# Get the repo root (2 levels up from tests/e2e/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
DB_PATH="$REPO_ROOT/data/api-data-dev/ledger.db"

echo "🌱 Seeding E2E test data..."

if [ ! -f "$DB_PATH" ]; then
    echo "⚠️  Database not found at $DB_PATH"
    exit 1
fi

# E2E test provider pubkey (hex: "e2e-test-provider-for-dsl-search-testing")
PROVIDER_PUBKEY="6532652d746573742d70726f76696465722d666f722d64736c2d7365617263682d74657374696e67"

# Delete existing E2E test offerings (idempotent)
sqlite3 "$DB_PATH" "DELETE FROM provider_offerings WHERE pubkey = x'$PROVIDER_PUBKEY';"

# Insert test offerings
sqlite3 "$DB_PATH" <<EOF
-- Compute offerings
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
    x'$PROVIDER_PUBKEY',
    'e2e-compute-001', 'E2E Compute Low', 'E2E test offering - compute low price', NULL,
    'ICP', 25.0, 0.0, 'public', 'compute', 'KVM', 'monthly', 'in_stock',
    'AMD', 1, 2, '3.5 GHz', 'EPYC 7763', 'DDR4', '4 GB', 1, '50 GB',
    0, '1 Gbps', 1000, 'USA', 'New York', 40.7128, -74.0060,
    'cPanel', 720, 8760, 'ICP', 'E2E Test', 'Ubuntu 22.04', 1700000000000000000
),
(
    x'$PROVIDER_PUBKEY',
    'e2e-compute-002', 'E2E Compute High', 'E2E test offering - compute high price', NULL,
    'ICP', 75.0, 0.0, 'public', 'compute', 'KVM', 'monthly', 'in_stock',
    'Intel', 1, 4, '4.0 GHz', 'Xeon E-2388G', 'DDR4', '8 GB', 1, '100 GB',
    0, '1 Gbps', 2000, 'Germany', 'Frankfurt', 50.1109, 8.6821,
    'Plesk', 720, 8760, 'ICP', 'E2E Test', 'Ubuntu 22.04', 1700000000000000000
);

-- GPU offerings
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
    x'$PROVIDER_PUBKEY',
    'e2e-gpu-001', 'E2E GPU Mid', 'E2E test offering - gpu mid price', NULL,
    'ICP', 100.0, 0.0, 'public', 'gpu', 'Bare Metal', 'monthly', 'in_stock',
    'AMD', 1, 16, '3.7 GHz', 'Ryzen 9 7950X', 'DDR5', '64 GB', 1, '1 TB',
    0, '10 Gbps', 10000, 'USA', 'San Francisco', 37.7749, -122.4194,
    'NVIDIA RTX 4090', 1, 24, 168, 8760,
    'ICP', 'E2E Test', 'Ubuntu 22.04 LTS', 1700000000000000000
),
(
    x'$PROVIDER_PUBKEY',
    'e2e-gpu-002', 'E2E GPU High', 'E2E test offering - gpu high price', NULL,
    'ICP', 500.0, 0.0, 'public', 'gpu', 'Bare Metal', 'monthly', 'in_stock',
    'AMD', 2, 64, '2.9 GHz', 'EPYC 7763', 'DDR4', '512 GB', 2, '4 TB',
    1, '25 Gbps', NULL, 'Japan', 'Tokyo', 35.6762, 139.6503,
    'NVIDIA A100', 4, 320, 720, 8760,
    'ICP', 'E2E Test', 'Ubuntu 22.04 LTS', 1700000000000000000
);

-- Storage offerings
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
    x'$PROVIDER_PUBKEY',
    'e2e-storage-001', 'E2E Storage Low', 'E2E test offering - storage low price', NULL,
    'ICP', 10.0, 0.0, 'public', 'storage', 'monthly', 'in_stock',
    4, '4 TB', NULL, NULL, 0, '1 Gbps', 500,
    'USA', 'Dallas', 32.7767, -96.7970,
    'S3-Compatible API', 720, 8760,
    'ICP', 'E2E Test', 1700000000000000000
),
(
    x'$PROVIDER_PUBKEY',
    'e2e-storage-002', 'E2E Storage Mid', 'E2E test offering - storage mid price', NULL,
    'ICP', 50.0, 0.0, 'public', 'storage', 'monthly', 'in_stock',
    NULL, NULL, 2, '2 TB', 0, '10 Gbps', 2000,
    'Netherlands', 'Amsterdam', 52.3676, 4.9041,
    'Block Storage API', 720, 8760,
    'ICP', 'E2E Test', 1700000000000000000
);

-- Network offerings
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee,
    visibility, product_type, billing_interval, stock_status,
    unmetered_bandwidth, uplink_speed, traffic,
    datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
    control_panel, min_contract_hours, max_contract_hours,
    payment_methods, features, created_at_ns
) VALUES
(
    x'$PROVIDER_PUBKEY',
    'e2e-network-001', 'E2E Network Low', 'E2E test offering - network low price', NULL,
    'ICP', 15.0, 0.0, 'public', 'network', 'monthly', 'in_stock',
    0, '10 Gbps', 5000, 'USA', 'Seattle', 47.6062, -122.3321,
    'REST API', 720, 8760,
    'ICP', 'E2E Test', 1700000000000000000
),
(
    x'$PROVIDER_PUBKEY',
    'e2e-network-002', 'E2E Network High', 'E2E test offering - network high price', NULL,
    'ICP', 80.0, 0.0, 'public', 'network', 'monthly', 'in_stock',
    1, '10 Gbps', NULL, 'Japan', 'Tokyo', 35.6762, 139.6503,
    NULL, 720, 8760,
    'ICP', 'E2E Test', 1700000000000000000
);
EOF

INSERTED=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM provider_offerings WHERE pubkey = x'$PROVIDER_PUBKEY';")

echo "✅ Seeded $INSERTED E2E test offering(s)"
echo "   Provider pubkey: e2e-test-provider-for-dsl-search-testing"
echo "   Ready for E2E tests!"
