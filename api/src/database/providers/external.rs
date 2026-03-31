use super::*;
use crate::database::types::{Database, LedgerEntryData};
use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::BorshDeserialize;
use dcc_common::CheckInPayload;

impl Database {
    /// Get list of active validators (checked in recently, with or without profiles)
    pub async fn get_active_validators(&self, days: i64) -> Result<Vec<Validator>> {
        let cutoff_ns = crate::now_ns()? - days.max(1) * 24 * 3600 * 1_000_000_000;
        let now_ns = crate::now_ns()?;
        let cutoff_24h = now_ns - 24 * 3600 * 1_000_000_000;
        let cutoff_7d = now_ns - 7 * 24 * 3600 * 1_000_000_000;
        let cutoff_30d = now_ns - 30 * 24 * 3600 * 1_000_000_000;

        let validators = sqlx::query_as!(
            Validator,
            r#"SELECT
                lower(encode(r.pubkey, 'hex')) as "pubkey!: String",
                NULLIF(p.name, '') as "name: String",
                NULLIF(p.description, '') as "description: String",
                NULLIF(p.website_url, '') as "website_url: String",
                NULLIF(p.logo_url, '') as "logo_url: String",
                COUNT(DISTINCT c.block_timestamp_ns) as "total_check_ins!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > $1 THEN 1 ELSE 0 END), 0) as "check_ins_24h!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > $2 THEN 1 ELSE 0 END), 0) as "check_ins_7d!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > $3 THEN 1 ELSE 0 END), 0) as "check_ins_30d!: i64",
                MAX(c.block_timestamp_ns) as "last_check_in_ns!: i64",
                r.created_at_ns as "registered_at_ns!: i64"
             FROM provider_registrations r
             INNER JOIN provider_check_ins c ON r.pubkey = c.pubkey
             LEFT JOIN provider_profiles p ON r.pubkey = p.pubkey
             WHERE c.block_timestamp_ns > $4
             GROUP BY r.pubkey, r.created_at_ns, p.name, p.description, p.website_url, p.logo_url
             ORDER BY MAX(c.block_timestamp_ns) DESC"#,
            cutoff_24h,
            cutoff_7d,
            cutoff_30d,
            cutoff_ns
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(validators)
    }

    /// List external providers with offering counts
    pub async fn list_external_providers(&self) -> Result<Vec<ExternalProvider>> {
        let rows = sqlx::query!(
            r#"SELECT
                ep.pubkey,
                ep.name,
                ep.domain,
                ep.website_url,
                ep.logo_url,
                ep.data_source,
                ep.created_at_ns,
                CAST(COUNT(po.id) AS BIGINT) as "offerings_count!: i64"
            FROM external_providers ep
            LEFT JOIN provider_offerings po ON ep.pubkey = po.pubkey AND po.offering_source = 'seeded'
            GROUP BY ep.pubkey, ep.name, ep.domain, ep.website_url, ep.logo_url, ep.data_source, ep.created_at_ns
            ORDER BY ep.name"#
        )
        .fetch_all(&self.pool)
        .await?;

        let providers = rows
            .into_iter()
            .map(|row| ExternalProvider {
                pubkey: hex::encode(&row.pubkey),
                name: row.name,
                domain: row.domain,
                website_url: row.website_url,
                logo_url: row.logo_url,
                data_source: row.data_source,
                offerings_count: row.offerings_count,
                created_at_ns: row.created_at_ns,
            })
            .collect();

        Ok(providers)
    }

    /// Create or update an external provider.
    /// Used by: `api-cli scrape-provider` command
    #[allow(dead_code)] // Used by api-cli binary, not api-server
    pub async fn create_or_update_external_provider(
        &self,
        pubkey: &[u8],
        name: &str,
        domain: &str,
        website_url: &str,
        data_source: &str,
    ) -> Result<()> {
        let created_at_ns = crate::now_ns()?;

        sqlx::query!(
            r#"INSERT INTO external_providers (pubkey, name, domain, website_url, data_source, created_at_ns)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT(pubkey) DO UPDATE SET
                   name = excluded.name,
                   domain = excluded.domain,
                   website_url = excluded.website_url,
                   data_source = excluded.data_source"#,
            pubkey,
            name,
            domain,
            website_url,
            data_source,
            created_at_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Provider registrations
    pub(crate) async fn insert_provider_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // Store raw Ed25519 public key (32 bytes) and signature
            let timestamp_i64 = entry.block_timestamp_ns as i64;
            sqlx::query!(
                "INSERT INTO provider_registrations (pubkey, signature, created_at_ns) VALUES ($1, $2, $3) ON CONFLICT (pubkey) DO UPDATE SET signature = excluded.signature, created_at_ns = excluded.created_at_ns",
                &entry.key,
                &entry.value,
                timestamp_i64
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider check-ins
    pub(crate) async fn insert_provider_check_ins(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let check_in = match CheckInPayload::try_from_slice(&entry.value) {
                Ok(check_in) => check_in,
                Err(e) => {
                    if entry.value.len() == 64 {
                        // Earlier versions of the protocol stored the nonce signature directly
                        CheckInPayload::new(String::new(), entry.value.clone())
                    } else {
                        tracing::error!(
                            "Failed to parse check-in: {}. Payload: {} len {}",
                            e,
                            BASE64.encode(&entry.value),
                            entry.value.len()
                        );
                        continue;
                    }
                }
            };

            let timestamp_i64 = entry.block_timestamp_ns as i64;
            let memo = check_in.memo().to_string();
            let nonce_signature = check_in.nonce_signature();
            sqlx::query!(
                "INSERT INTO provider_check_ins (pubkey, memo, nonce_signature, block_timestamp_ns) VALUES ($1, $2, $3, $4)",
                &entry.key,
                memo,
                nonce_signature,
                timestamp_i64
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    /// Store Chatwoot resources created for a provider during onboarding.
    pub async fn set_provider_chatwoot_resources(
        &self,
        pubkey: &[u8],
        inbox_id: u32,
        team_id: u32,
        portal_slug: &str,
    ) -> Result<()> {
        let inbox_id = inbox_id as i64;
        let team_id = team_id as i64;
        sqlx::query!(
            r#"UPDATE provider_profiles
               SET chatwoot_inbox_id = $1, chatwoot_team_id = $2, chatwoot_portal_slug = $3
               WHERE pubkey = $4"#,
            inbox_id,
            team_id,
            portal_slug,
            pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get Chatwoot resources for a provider.
    /// Returns (inbox_id, team_id, portal_slug) if set.
    pub async fn get_provider_chatwoot_resources(
        &self,
        pubkey: &[u8],
    ) -> Result<Option<(u32, u32, String)>> {
        let row = sqlx::query!(
            r#"SELECT chatwoot_inbox_id, chatwoot_team_id, chatwoot_portal_slug
               FROM provider_profiles WHERE pubkey = $1"#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| {
            match (
                r.chatwoot_inbox_id,
                r.chatwoot_team_id,
                r.chatwoot_portal_slug,
            ) {
                (Some(inbox), Some(team), Some(slug)) => Some((inbox as u32, team as u32, slug)),
                _ => None,
            }
        }))
    }
}
