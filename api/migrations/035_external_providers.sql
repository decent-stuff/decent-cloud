-- External Providers and Seeded Offerings
-- Tracks providers we've seeded but haven't onboarded yet
-- Extends provider_offerings to differentiate seeded vs provider-submitted

-- External providers table - tracks providers not yet onboarded
CREATE TABLE IF NOT EXISTS external_providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey BLOB NOT NULL UNIQUE,           -- Deterministic pubkey from domain
    name TEXT NOT NULL,
    domain TEXT NOT NULL UNIQUE,
    website_url TEXT NOT NULL,
    logo_url TEXT,
    data_source TEXT NOT NULL,             -- "scraper", "manual_curation"
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_external_providers_domain ON external_providers(domain);

-- Add source tracking columns to provider_offerings
-- Values for offering_source: 'provider' (normal), 'seeded' (scraped/curated)
ALTER TABLE provider_offerings ADD COLUMN offering_source TEXT DEFAULT 'provider';

-- Direct link to provider's checkout page for seeded offerings
ALTER TABLE provider_offerings ADD COLUMN external_checkout_url TEXT;
