//! Shared HMAC-SHA256 webhook signature verification.
//!
//! Both the Stripe webhook handler and (when #414 lands) the GitHub
//! webhook handler need to recompute an HMAC-SHA256 over a signed
//! payload and compare it against an attacker-supplied hex string.
//! That comparison MUST run in constant time to remove the timing
//! side channel documented in #428.

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Compute `HMAC-SHA256(secret, signed_payload)` and compare it
/// constant-time against `expected_hex`.
///
/// Errors if the secret is malformed, the expected hex cannot be
/// decoded, or the signatures do not match.
pub fn verify_hmac_sha256_hex(
    signed_payload: &[u8],
    secret: &[u8],
    expected_hex: &str,
) -> Result<()> {
    let expected = hex::decode(expected_hex).context("Invalid signature: malformed hex")?;
    let mut mac = HmacSha256::new_from_slice(secret).context("Invalid webhook secret")?;
    mac.update(signed_payload);
    let computed = mac.finalize().into_bytes();
    if !bool::from(computed[..].ct_eq(&expected[..])) {
        return Err(anyhow::anyhow!("Invalid signature"));
    }
    Ok(())
}
