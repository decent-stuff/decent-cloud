//! TOTP two-factor authentication (RFC 6238 / ticket #80)
//!
//! # Design
//!
//! TOTP secrets are encrypted at rest using the server-side AES-256-GCM key
//! (`CREDENTIAL_ENCRYPTION_KEY`). The enrollment flow is:
//!
//! 1. `setup_totp` — generate random secret, encrypt, store (not yet active),
//!    return base32 secret + otpauth:// URI.
//! 2. `enable_totp` — client verifies the first code; marks enabled; returns
//!    one-time plaintext backup codes (hashed on storage).
//! 3. Subsequent logins may call `verify_totp` (or `verify_backup_code`).
//! 4. `disable_totp` — verifies code or backup code, then clears the secret
//!    and all backup codes.
//!
//! # Multi-session
//!
//! The TOTP secret is static per account; multiple simultaneous sessions work
//! without any extra tracking because every session can independently verify
//! against the same time-based code.
//!
//! # Clock tolerance
//!
//! Accepts the current 30-second window plus ±1 adjacent window to handle
//! reasonable clock skew between authenticator apps and the server.

use crate::crypto::{decrypt_server_credential, encrypt_server_credential, ServerEncryptionKey};
use anyhow::{bail, Context, Result};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;
use sha2::{Digest, Sha256};

use super::types::Database;

// ── constants ──────────────────────────────────────────────────────────────

const TOTP_STEP: u64 = 30; // 30-second window
const TOTP_DIGITS: u32 = 6;
const TOTP_WINDOW: i64 = 1; // accept ±1 step
const SECRET_BYTES: usize = 20; // 160-bit secret → 32-char base32
const BACKUP_CODE_COUNT: usize = 8;
const BACKUP_CODE_BYTES: usize = 6; // 6 raw bytes → 10 hex chars per code

/// Service name shown in authenticator apps.
const ISSUER: &str = "DecentCloud";

// ── pure TOTP algorithm ────────────────────────────────────────────────────

/// Compute one TOTP code for the given secret bytes and counter.
fn hotp(secret: &[u8], counter: u64) -> u32 {
    let msg = counter.to_be_bytes();
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("HMAC-SHA1 accepts any key length");
    mac.update(&msg);
    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0x0f) as usize;
    let code = u32::from_be_bytes([
        result[offset] & 0x7f,
        result[offset + 1],
        result[offset + 2],
        result[offset + 3],
    ]);
    code % 10_u32.pow(TOTP_DIGITS)
}

/// Verify a 6-digit string against the current time window (±1 step).
pub fn verify_totp_code(secret_bytes: &[u8], code: &str) -> bool {
    let Ok(provided) = code.parse::<u32>() else {
        return false;
    };
    let t = current_time_step();
    for delta in -TOTP_WINDOW..=TOTP_WINDOW {
        let step = t.checked_add_signed(delta).unwrap_or(t);
        if hotp(secret_bytes, step) == provided {
            return true;
        }
    }
    false
}

/// Decode a base32 secret string to raw bytes (for HMAC).
fn decode_b32_secret(secret_b32: &str) -> Result<Vec<u8>> {
    base32::decode(base32::Alphabet::Rfc4648 { padding: false }, secret_b32)
        .ok_or_else(|| anyhow::anyhow!("Invalid base32 TOTP secret"))
}

fn current_time_step() -> u64 {
    let unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    unix_secs / TOTP_STEP
}

// ── backup code helpers ────────────────────────────────────────────────────

/// Generate 8 random backup codes.  Returns `(plaintext_vec, hash_vec)`.
fn generate_backup_codes() -> (Vec<String>, Vec<Vec<u8>>) {
    let mut rng = rand::rng();
    let mut plaintext = Vec::with_capacity(BACKUP_CODE_COUNT);
    let mut hashes = Vec::with_capacity(BACKUP_CODE_COUNT);
    for _ in 0..BACKUP_CODE_COUNT {
        let mut raw = [0u8; BACKUP_CODE_BYTES];
        rng.fill_bytes(&mut raw);
        let code = hex::encode(raw); // 12-char hex string
        let hash = Sha256::digest(code.as_bytes()).to_vec();
        plaintext.push(code);
        hashes.push(hash);
    }
    (plaintext, hashes)
}

fn hash_backup_code(code: &str) -> Vec<u8> {
    Sha256::digest(code.trim().to_lowercase().as_bytes()).to_vec()
}

// ── status type ───────────────────────────────────────────────────────────

pub struct TotpStatus {
    pub enabled: bool,
    pub has_backup_codes: bool,
}

// ── Database impl ─────────────────────────────────────────────────────────

impl Database {
    /// Begin TOTP enrollment: generate secret, encrypt, store (unconfirmed).
    ///
    /// Returns `(secret_b32, otpauth_uri)`. The secret is stored immediately so
    /// a subsequent `enable_totp` call can read it. Until `totp_enabled = TRUE`
    /// this has no effect on authentication.
    pub async fn setup_totp(&self, account_id: &[u8], username: &str) -> Result<(String, String)> {
        let enc_key = ServerEncryptionKey::from_env()
            .context("TOTP setup requires CREDENTIAL_ENCRYPTION_KEY")?;

        // Generate random secret
        let mut secret_bytes = [0u8; SECRET_BYTES];
        rand::rng().fill_bytes(&mut secret_bytes);

        // Encode for display in QR / manual entry
        let secret_b32 =
            base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret_bytes);

        // Encrypt the base32 string for storage
        let encrypted = encrypt_server_credential(&secret_b32, &enc_key)?;

        // Store (pending; totp_enabled stays FALSE)
        sqlx::query!(
            "UPDATE accounts SET totp_secret = $1 WHERE id = $2",
            encrypted,
            account_id,
        )
        .execute(&self.pool)
        .await
        .context("Failed to store pending TOTP secret")?;

        // Build otpauth:// URI (RFC 3548 / Google Authenticator convention)
        let label = urlencoding::encode(&format!("{ISSUER}:{username}")).into_owned();
        let uri = format!(
            "otpauth://totp/{label}?secret={secret_b32}&issuer={ISSUER}&algorithm=SHA1&digits={TOTP_DIGITS}&period={TOTP_STEP}"
        );

        Ok((secret_b32, uri))
    }

    /// Confirm enrollment: verify first code, enable TOTP, return backup codes.
    ///
    /// The backup codes are returned **once** in plaintext; only their SHA-256
    /// hashes are stored.
    pub async fn enable_totp(&self, account_id: &[u8], code: &str) -> Result<Vec<String>> {
        let enc_key = ServerEncryptionKey::from_env()
            .context("TOTP enable requires CREDENTIAL_ENCRYPTION_KEY")?;

        let row = sqlx::query!(
            r#"SELECT totp_secret, totp_enabled FROM accounts WHERE id = $1"#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            bail!("Account not found");
        };
        if row.totp_enabled {
            bail!("TOTP is already enabled");
        }
        let Some(encrypted) = row.totp_secret else {
            bail!("TOTP setup not started — call /accounts/me/totp/setup first");
        };

        let secret_b32 = decrypt_server_credential(&encrypted, &enc_key)
            .context("Failed to decrypt TOTP secret")?;
        let secret_bytes = decode_b32_secret(&secret_b32)?;

        if !verify_totp_code(&secret_bytes, code) {
            bail!("Invalid TOTP code");
        }

        // Generate backup codes
        let (plaintext_codes, hashes) = generate_backup_codes();
        let now = crate::now_ns()?;

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            "UPDATE accounts SET totp_enabled = TRUE WHERE id = $1",
            account_id
        )
        .execute(&mut *tx)
        .await?;

        // Remove any existing backup codes (re-enable scenario)
        sqlx::query!(
            "DELETE FROM totp_backup_codes WHERE account_id = $1",
            account_id
        )
        .execute(&mut *tx)
        .await?;

        for hash in &hashes {
            sqlx::query!(
                "INSERT INTO totp_backup_codes (account_id, code_hash, created_at) VALUES ($1, $2, $3)",
                account_id,
                hash.as_slice(),
                now,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(plaintext_codes)
    }

    /// Disable TOTP.  Requires a valid TOTP code or backup code as proof.
    pub async fn disable_totp(&self, account_id: &[u8], code: &str) -> Result<()> {
        let enc_key = ServerEncryptionKey::from_env()
            .context("TOTP disable requires CREDENTIAL_ENCRYPTION_KEY")?;

        let row = sqlx::query!(
            r#"SELECT totp_secret, totp_enabled FROM accounts WHERE id = $1"#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            bail!("Account not found");
        };
        if !row.totp_enabled {
            bail!("TOTP is not enabled");
        }
        let Some(encrypted) = row.totp_secret else {
            bail!("TOTP secret missing — database inconsistency");
        };

        let secret_b32 = decrypt_server_credential(&encrypted, &enc_key)?;
        let secret_bytes = decode_b32_secret(&secret_b32)?;

        // Accept either a live TOTP code or a backup code
        let verified = if verify_totp_code(&secret_bytes, code) {
            true
        } else {
            self.consume_backup_code_inner(account_id, code).await?
        };

        if !verified {
            bail!("Invalid TOTP code or backup code");
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            "UPDATE accounts SET totp_secret = NULL, totp_enabled = FALSE WHERE id = $1",
            account_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM totp_backup_codes WHERE account_id = $1",
            account_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Regenerate backup codes.  Requires a valid TOTP code to authorise.
    pub async fn regenerate_backup_codes(
        &self,
        account_id: &[u8],
        code: &str,
    ) -> Result<Vec<String>> {
        let enc_key = ServerEncryptionKey::from_env()
            .context("Backup code regeneration requires CREDENTIAL_ENCRYPTION_KEY")?;

        let row = sqlx::query!(
            r#"SELECT totp_secret, totp_enabled FROM accounts WHERE id = $1"#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            bail!("Account not found");
        };
        if !row.totp_enabled {
            bail!("TOTP is not enabled");
        }
        let encrypted = row.totp_secret.context("TOTP secret missing")?;
        let secret_b32 = decrypt_server_credential(&encrypted, &enc_key)?;
        let secret_bytes = decode_b32_secret(&secret_b32)?;

        if !verify_totp_code(&secret_bytes, code) {
            bail!("Invalid TOTP code");
        }

        let (plaintext_codes, hashes) = generate_backup_codes();
        let now = crate::now_ns()?;

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            "DELETE FROM totp_backup_codes WHERE account_id = $1",
            account_id
        )
        .execute(&mut *tx)
        .await?;

        for hash in &hashes {
            sqlx::query!(
                "INSERT INTO totp_backup_codes (account_id, code_hash, created_at) VALUES ($1, $2, $3)",
                account_id,
                hash.as_slice(),
                now,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(plaintext_codes)
    }

    /// Get TOTP status for an account.
    pub async fn totp_status(&self, account_id: &[u8]) -> Result<TotpStatus> {
        let row = sqlx::query!(
            r#"SELECT totp_enabled FROM accounts WHERE id = $1"#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            bail!("Account not found");
        };

        let backup_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM totp_backup_codes WHERE account_id = $1 AND used_at IS NULL",
            account_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(TotpStatus {
            enabled: row.totp_enabled,
            has_backup_codes: backup_count > 0,
        })
    }

    /// Verify a TOTP code for an already-enabled account.
    /// Returns `Ok(true)` on match, `Ok(false)` on wrong code.
    /// Used by sensitive-operation enforcement (e.g. future login-gate, key rotation).
    #[allow(dead_code)]
    pub async fn verify_totp(&self, account_id: &[u8], code: &str) -> Result<bool> {
        let enc_key = ServerEncryptionKey::from_env()
            .context("TOTP verification requires CREDENTIAL_ENCRYPTION_KEY")?;

        let row = sqlx::query!(
            r#"SELECT totp_secret, totp_enabled FROM accounts WHERE id = $1"#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            bail!("Account not found");
        };
        if !row.totp_enabled {
            bail!("TOTP is not enabled for this account");
        }
        let encrypted = row.totp_secret.context("TOTP secret missing")?;
        let secret_b32 = decrypt_server_credential(&encrypted, &enc_key)?;
        let secret_bytes = decode_b32_secret(&secret_b32)?;
        Ok(verify_totp_code(&secret_bytes, code))
    }

    /// Verify and consume a backup code.
    /// Returns `Ok(true)` if the code was valid (and is now consumed).
    /// Exposed for future use by login-gate or sensitive-operation enforcement.
    #[allow(dead_code)]
    pub async fn verify_and_consume_backup_code(
        &self,
        account_id: &[u8],
        code: &str,
    ) -> Result<bool> {
        self.consume_backup_code_inner(account_id, code).await
    }

    async fn consume_backup_code_inner(&self, account_id: &[u8], code: &str) -> Result<bool> {
        let hash = hash_backup_code(code);
        let now = crate::now_ns()?;

        let result = sqlx::query!(
            r#"UPDATE totp_backup_codes
               SET used_at = $1
               WHERE account_id = $2
                 AND code_hash = $3
                 AND used_at IS NULL
               RETURNING id"#,
            now,
            account_id,
            hash.as_slice(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }
}

// ── unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-good TOTP vector from RFC 6238 appendix B (TOTP-SHA1, secret = "12345678901234567890").
    #[test]
    fn test_hotp_known_vector() {
        let secret = b"12345678901234567890";
        // T = 59 seconds → step = 1 (T/30 = 1)
        assert_eq!(hotp(secret, 1), 287082);
        // T = 1111111109 → step = 37037036
        assert_eq!(hotp(secret, 37037036), 81804);
    }

    #[test]
    fn test_verify_totp_code_valid() {
        let secret = b"12345678901234567890";
        let t = current_time_step();
        let code = format!("{:06}", hotp(secret, t));
        assert!(verify_totp_code(secret, &code));
    }

    #[test]
    fn test_verify_totp_code_wrong() {
        let secret = b"12345678901234567890";
        assert!(!verify_totp_code(secret, "000000"));
    }

    #[test]
    fn test_verify_totp_code_non_numeric() {
        let secret = b"12345678901234567890";
        assert!(!verify_totp_code(secret, "abc123"));
    }

    #[test]
    fn test_backup_code_generation() {
        let (plain, hashes) = generate_backup_codes();
        assert_eq!(plain.len(), BACKUP_CODE_COUNT);
        assert_eq!(hashes.len(), BACKUP_CODE_COUNT);
        // each code is unique
        let unique: std::collections::HashSet<_> = plain.iter().collect();
        assert_eq!(unique.len(), BACKUP_CODE_COUNT);
    }

    #[test]
    fn test_hash_backup_code_normalisation() {
        let code = "AbCdEf123456";
        let h1 = hash_backup_code(code);
        let h2 = hash_backup_code(" abcdef123456 ");
        assert_eq!(h1, h2);
    }
}
