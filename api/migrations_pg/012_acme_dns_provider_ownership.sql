-- Bind dc_id ownership to provider identity.
-- Prevents one provider's agent from hijacking another provider's dc_id
-- (acme-dns credentials, DNS records, wildcard TLS cert).
-- Existing rows are cleared; agents re-register on next `dc-agent setup token`.

DELETE FROM acme_dns_accounts;
ALTER TABLE acme_dns_accounts ADD COLUMN provider_pubkey BYTEA NOT NULL;
