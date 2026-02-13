-- acme-dns accounts for per-provider TLS isolation.
-- Each provider gets a unique username/password pair used by Caddy's acmedns plugin
-- to POST TXT record updates to our central API, which proxies them to Cloudflare.

CREATE TABLE acme_dns_accounts (
    username UUID PRIMARY KEY,
    password_hash TEXT NOT NULL,
    dc_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One set of credentials per provider (dc_id). Re-registration overwrites.
CREATE UNIQUE INDEX idx_acme_dns_dc_id ON acme_dns_accounts(dc_id);
