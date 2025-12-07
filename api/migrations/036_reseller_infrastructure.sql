-- Reseller relationships: who can resell which external provider's offerings
CREATE TABLE IF NOT EXISTS reseller_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reseller_pubkey BLOB NOT NULL,           -- Onboarded provider acting as reseller
    external_provider_pubkey BLOB NOT NULL,  -- External provider being resold
    -- Commission settings
    commission_percent INTEGER NOT NULL DEFAULT 10,  -- 0-50%, markup on base price
    -- Status
    status TEXT NOT NULL DEFAULT 'active',   -- 'active', 'suspended'
    created_at_ns INTEGER NOT NULL,
    updated_at_ns INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(reseller_pubkey, external_provider_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_reseller_relationships_reseller
    ON reseller_relationships(reseller_pubkey);
CREATE INDEX IF NOT EXISTS idx_reseller_relationships_status
    ON reseller_relationships(status);

-- Track reseller orders (contracts proxied through reseller)
CREATE TABLE IF NOT EXISTS reseller_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL UNIQUE,        -- FK to contract
    reseller_pubkey BLOB NOT NULL,
    external_provider_pubkey BLOB NOT NULL,
    offering_id INTEGER NOT NULL,            -- Which offering was ordered
    -- Financial breakdown
    base_price_e9s INTEGER NOT NULL,         -- Original price
    commission_e9s INTEGER NOT NULL,         -- Reseller commission
    total_paid_e9s INTEGER NOT NULL,         -- What user paid
    -- External order tracking
    external_order_id TEXT,                  -- Provider's order ID
    external_order_details TEXT,             -- JSON: instance details
    -- Status
    status TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'fulfilled', 'failed'
    created_at_ns INTEGER NOT NULL,
    fulfilled_at_ns INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reseller_orders_reseller
    ON reseller_orders(reseller_pubkey);
CREATE INDEX IF NOT EXISTS idx_reseller_orders_status
    ON reseller_orders(status);
