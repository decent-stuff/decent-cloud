# Gateway TLS Isolation: Per-Provider Wildcard Certs via API-Proxied acme-dns

**Status:** Implemented
**Created:** 2026-02-13
**Depends On:** [DC Gateway Spec](./2025-12-31-dc-gateway-spec.md)

## Problem Statement

The current gateway architecture uses a single shared wildcard cert `*.{gw_prefix}.{domain}` across all providers. This has two security problems:

1. **Wildcard cert scope** — any provider holding a valid cert for `*.gw.decent-cloud.org` can impersonate VMs on any other provider.
2. **Cloudflare token exposure** — each provider host has a `CF_API_TOKEN` with `Zone:DNS:Edit` permission on the entire `decent-cloud.org` zone. A compromised host can manipulate DNS records for any subdomain, enabling MITM attacks against other providers' VMs.

With a single trusted provider (current state), this is acceptable. With multiple untrusted providers, it is not.

## Solution

Isolate TLS trust boundaries per provider using per-provider wildcard certs. ACME DNS-01 challenges are proxied through the central API (which implements the acme-dns HTTP protocol) so providers never get Cloudflare credentials and no separate DNS server is needed.

**Three changes:**
1. Subdomain format: `{slug}.{dc_id}.{gw_prefix}.{domain}` (dot-separated, 4 levels)
2. Per-provider wildcard cert: `*.{dc_id}.{gw_prefix}.{domain}`
3. DNS-01 via central API acme-dns endpoint instead of direct Cloudflare access

## Architecture

```
Provider Host                    Central API                      Cloudflare
+------------------+            +------------------+             +-----------+
| Caddy            |            |                  |             |           |
| (acmedns plugin) |----------->| POST             | TXT upsert  |           |
|                  | TXT update | /api/v1/acme-dns/ |----------->| DNS zone  |
| Private key      |            | update           |             |           |
| stays here       |            |                  |             |           |
|                  |            |                  | A record     |           |
| dc-agent         |----------->| /api/v1/agents/  |----------->|           |
|                  | per-VM DNS | dns              |             |           |
+------------------+            +------------------+             +-----------+

Cert scope: *.{dc_id}.{gw_prefix}.{domain}  (only this provider's VMs)
```

**Key insight:** The Caddy acmedns plugin only ever calls `POST {server_url}/update` with `X-Api-User`/`X-Api-Key` headers. It never calls `/register`. Our central API implements this single endpoint and proxies TXT records to Cloudflare. No separate DNS server, no CNAME delegation, no new infrastructure.

**What each component has access to:**

| Component | Cloudflare token | acme-dns credentials | TLS private key |
|-----------|-----------------|---------------------|-----------------|
| Provider host | No | Scoped to own dc_id | Yes (local only) |
| Central API | Yes | Manages all (hashed) | No |

## Subdomain Format

**Before:** `{slug}-{dc_id}.{gw_prefix}.{domain}` (3 levels, shared wildcard)
**After:** `{slug}.{dc_id}.{gw_prefix}.{domain}` (4 levels, per-provider wildcard)

| Component | Format | Example |
|-----------|--------|---------|
| slug | 6-char `[a-z0-9]` | `k7m2p4` |
| dc_id | 2-20 char `[a-z0-9-]` | `dc-lk` |
| gw_prefix | `gw` (prod) / `dev-gw` (dev) | `dev-gw` |
| domain | zone domain | `decent-cloud.org` |

**Dev example:** `k7m2p4.dc-lk.dev-gw.decent-cloud.org`
**Prod example:** `k7m2p4.dc-lk.gw.decent-cloud.org`

**Wildcard cert per provider:** `*.dc-lk.dev-gw.decent-cloud.org`

## acme-dns Protocol (API-Proxied)

The central API implements the acme-dns HTTP update protocol at `POST /api/v1/acme-dns/update`:

```
POST /api/v1/acme-dns/update
Headers:
  X-Api-User: <uuid>
  X-Api-Key: <password>
  Content-Type: application/json
Body: {"subdomain": "<uuid>", "txt": "<acme-challenge-value>"}
Response: {"txt": "<acme-challenge-value>"}
```

**Flow:**
1. Validate `X-Api-User` (UUID) and `X-Api-Key` (password) against `acme_dns_accounts` table
2. Look up `dc_id` from the account row
3. Call Cloudflare API to upsert TXT record at `_acme-challenge.{dc_id}.{gw_prefix}.{domain}` (TTL 60s)
4. Return `{"txt": "<value>"}`

### Caddy integration

The official [`caddy-dns/acmedns`](https://github.com/caddy-dns/acmedns) plugin works unmodified — it just POSTs to our API URL instead of a standalone acme-dns server.

**Caddyfile:**
```
*.{dc_id}.{gw_prefix}.{domain} {
    tls {
        dns acmedns {
            server_url {env.ACME_DNS_SERVER_URL}
            username {env.ACME_DNS_USERNAME}
            password {env.ACME_DNS_PASSWORD}
            subdomain {env.ACME_DNS_SUBDOMAIN}
        }
    }
    import /etc/caddy/sites/*.caddy
}
```

**Env file** (`/etc/caddy/env`, mode 600):
```
ACME_DNS_SERVER_URL=https://api.decent-cloud.org/api/v1/acme-dns
ACME_DNS_USERNAME=<uuid>
ACME_DNS_PASSWORD=<password>
ACME_DNS_SUBDOMAIN=<uuid>
```

### Per-VM Caddy config

```
# /etc/caddy/sites/k7m2p4.caddy
@k7m2p4 host k7m2p4.dc-lk.dev-gw.decent-cloud.org
handle @k7m2p4 {
    reverse_proxy 10.0.1.5:80
}
```

## DNS Records Required

### Per-provider (created automatically via API during ACME challenges)

```
_acme-challenge.dc-lk.dev-gw.decent-cloud.org.  TXT  "<acme-challenge-value>"
```

### Per-VM (existing, unchanged)

```
k7m2p4.dc-lk.dev-gw.decent-cloud.org.  A  203.0.113.1
```

## Provider Registration Flow

During `dc-agent setup token`:

```
1. Agent sends POST /api/v1/agents/gateway/register with {dc_id}
2. Central API:
   a. Generates UUID username + random password
   b. Stores (username, sha256(password), dc_id) in acme_dns_accounts table
   c. Returns {acme_dns_server_url, username, password, subdomain}
3. dc-agent:
   a. Writes credentials to /etc/caddy/env
   b. Downloads Caddy with acmedns plugin (from official Caddy download API)
   c. Writes Caddyfile with per-provider wildcard + acmedns TLS
   d. Starts Caddy -> requests cert -> Caddy POSTs to our API -> API sets TXT in Cloudflare
   e. Let's Encrypt queries Cloudflare directly for TXT verification
   f. Cert issued, private key stays local
```

## Database Schema

```sql
CREATE TABLE acme_dns_accounts (
    username UUID PRIMARY KEY,
    password_hash TEXT NOT NULL,
    dc_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_acme_dns_dc_id ON acme_dns_accounts(dc_id);
```

One set of credentials per provider. Re-registration (same dc_id) overwrites.

## Rate Limits

Let's Encrypt limits: 50 certificates per registered domain per week.

Each provider gets one wildcard cert (e.g., `*.dc-lk.gw.decent-cloud.org`). Certs last 90 days, Caddy renews at 30 days before expiry. This allows onboarding ~50 new providers per week, with renewals spread across 60-day windows. No practical constraint.

## Security Model

| Threat | Before | After |
|--------|--------|-------|
| Compromised provider impersonates other provider's VMs | Possible (shared wildcard) | Blocked (cert only covers own dc_id) |
| Compromised provider manipulates other DNS records | Possible (full zone edit token) | Blocked (no Cloudflare access) |
| Compromised provider's acme-dns credentials leak | N/A | Can only update own TXT record |
| TLS private key exfiltration | Key on provider host | Same (key stays local, but now scoped to own VMs only) |
| Central API compromised | Full Cloudflare access | Same (has CF token for A records and TXT proxying) |

## Alternatives Considered

### Self-hosted acme-dns server
- Deploy dedicated acme-dns DNS server on port 53 with NS delegation
- **Rejected:** requires infrastructure we don't need; the acmedns Caddy plugin only calls one HTTP endpoint (`POST /update`), which our API can implement directly

### Custom Caddy Go plugin calling Central API
- Requires writing Go, building and hosting custom Caddy binary
- **Rejected:** unnecessary when the standard acmedns plugin protocol is trivially proxied

### certbot + shell hook
- certbot runs DNS-01 with a hook script that calls Central API
- **Rejected:** more moving parts, loses Caddy's integrated cert management

### Shared wildcard with scoped CF tokens
- Each provider gets a CF token scoped to their records
- **Not possible:** Cloudflare API tokens cannot be scoped to specific record name prefixes, only to entire zones

## References

- [caddy-dns/acmedns](https://github.com/caddy-dns/acmedns) — Official Caddy acme-dns plugin (libdns/acmedns)
- [Let's Encrypt DNS-01](https://letsencrypt.org/docs/challenge-types/#dns-01-challenge)
- [Let's Encrypt Rate Limits](https://letsencrypt.org/docs/rate-limits/)
- [DC Gateway Spec](./2025-12-31-dc-gateway-spec.md) — Current architecture
