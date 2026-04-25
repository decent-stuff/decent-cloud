//! Stripe refund idempotency-key construction (issue #411).
//!
//! Single source of truth for the strings sent to Stripe in the
//! `Idempotency-Key` header AND stored in the `refund_audit` table. Same
//! inputs ALWAYS produce the same key; this is what lets a transient network
//! retry collapse onto one Stripe Refund record.

/// Build the deterministic idempotency key for a refund attempt.
///
/// Layout:
///   * `dispute`: `dispute:{stripe_dispute_id}` -- preserves the format that
///     Phase 2 of the Stripe-dispute work (commit `e50ea8e5`) wrote to Stripe
///     and asserted in tests; `stripe_dispute_id` is globally unique so
///     prepending the contract_id adds nothing and would break replay
///     idempotency for any in-flight dispute at the time this ships.
///   * everything else (`cancel`, `reject`, `manual`, ...):
///     `{reason}:{contract_id_hex}:{unique_token}`. Including contract_id_hex
///     scopes the key to one contract so two unrelated user-initiated cancels
///     cannot collide on a poorly-chosen unique_token.
pub fn refund_idempotency_key(reason: &str, contract_id: &[u8], unique_token: &str) -> String {
    if reason == "dispute" {
        return format!("dispute:{}", unique_token);
    }
    format!("{}:{}:{}", reason, hex::encode(contract_id), unique_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispute_key_preserves_phase2_format() {
        // Phase 2 (commit e50ea8e5) shipped this exact format and the
        // openapi/webhooks.rs test asserts the literal string. A break
        // here means in-flight dispute refunds get a fresh key on retry
        // and Stripe issues a duplicate refund.
        assert_eq!(
            refund_idempotency_key("dispute", b"\x01\x02\x03", "du_c4"),
            "dispute:du_c4"
        );
    }

    #[test]
    fn cancel_key_includes_contract_id_and_token() {
        let key = refund_idempotency_key("cancel", &[0xab, 0xcd], "cancel:1700000000000000000");
        assert_eq!(key, "cancel:abcd:cancel:1700000000000000000");
    }

    #[test]
    fn key_is_deterministic_across_calls() {
        let cid = b"\xde\xad\xbe\xef";
        let a = refund_idempotency_key("reject", cid, "reject:42");
        let b = refund_idempotency_key("reject", cid, "reject:42");
        let c = refund_idempotency_key("reject", cid, "reject:42");
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn distinct_inputs_yield_distinct_keys() {
        let cid_a = b"\x01";
        let cid_b = b"\x02";
        // Different contract -> different key.
        assert_ne!(
            refund_idempotency_key("cancel", cid_a, "cancel:1"),
            refund_idempotency_key("cancel", cid_b, "cancel:1"),
        );
        // Different token (e.g. second cancel attempt for a re-rented
        // contract) -> different key so retries do NOT collapse onto the
        // first refund.
        assert_ne!(
            refund_idempotency_key("cancel", cid_a, "cancel:1"),
            refund_idempotency_key("cancel", cid_a, "cancel:2"),
        );
        // Different reason -> different key.
        assert_ne!(
            refund_idempotency_key("cancel", cid_a, "x"),
            refund_idempotency_key("reject", cid_a, "x"),
        );
    }
}
