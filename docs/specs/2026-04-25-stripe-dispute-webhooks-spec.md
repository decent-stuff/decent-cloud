# Stripe charge.dispute.* Webhook Handlers -- Implementation Plan

- Issues: decent-stuff/decent-cloud#408 (handlers), #421 (pause-vs-terminate decision)
- Date: 2026-04-25 (revised 2026-04-25 to Option B per #421)
- Author: Backend API agent
- Status: Phase 1 (DB layer + migration + spec) IMPLEMENTED. Phase 2 (webhook
  handlers + dc-agent runtime change + ops-alert helper) PENDING -- tracked in
  a follow-up ticket. The diffs in section 4 remain authoritative as the
  specification for Phase 2; the `cancel_contract_for_dispute` references in
  the original spec are replaced by `pause_contract` / `resume_contract` /
  `terminate_contract_for_dispute_lost` (already shipped in Phase 1, see
  `api/src/database/contracts/dispute.rs`).

## 0. Goal and non-goals

Goal: stop silently dropping Stripe `charge.dispute.created`,
`charge.dispute.updated`, `charge.dispute.closed`, and
`charge.dispute.funds_withdrawn` events. Persist every dispute, transition
the related contract to a `paused` state that stops billable usage but
preserves customer state for reversal, and surface the event in logs and
ops alerts.

Non-goals (deferred):
- Automated evidence submission to Stripe (`/v1/disputes/{id}/close` with
  evidence) -- ticket noted under "follow-up" below.
- UI surfaces (admin dispute dashboard) -- separate ticket.
- Subscription-level dispute handling beyond contract-level. Subscriptions
  reuse the same `charge.dispute.*` events so the new code already covers
  both paths through the `contract_id` lookup; subscription-only disputes
  with no contract are logged and alerted but not state-mutated (see 4.h).

## 1. Source citations (read before changing anything)

- `api/src/openapi/webhooks.rs:127-685` -- existing Stripe webhook dispatch,
  signature verification at `:84-125`, idempotency-by-replay pattern (none
  exists today; `checkout.session.completed` is naturally idempotent because
  `update_checkout_session_payment` is an UPDATE, not an INSERT).
- `api/src/database/contracts/payment.rs:7-40` -- pattern for "update +
  emit `contract_events` row" inside one method.
- `api/src/database/contracts/payment.rs:107-160` -- prorated refund math.
  PHASE 1 EXTENDED: now takes `total_paused_ns`; paused intervals are
  excluded from billable use.
- `api/src/database/contracts/dispute.rs` (NEW in Phase 1) -- `pause_contract`,
  `resume_contract`, `terminate_contract_for_dispute_lost`,
  `upsert_contract_dispute`. All idempotent. Webhook handlers in Phase 2
  call these directly.
- `api/src/database/contracts/rental.rs:560-743` -- `cancel_contract`,
  including stripe refund call at `:594-643`. Pattern reused inside
  `terminate_contract_for_dispute_lost`.
- `api/src/database/contracts/rental.rs:367-537` -- `reject_contract`, full
  refund path at `:396-470`.
- `api/src/database/contracts/extensions.rs:210-237` -- `insert_contract_event`
  signature.
- `common/src/contract_status.rs:36-130` -- state machine. PHASE 1
  EXTENDED with `Paused`. Reversible: `Active <-> Paused`,
  `Provisioned <-> Paused`. Terminal-bypass: `Paused -> Cancelled`,
  `Paused -> Expired`. `is_cancellable=true`, `is_operational=false`,
  `is_terminal=false`.
- `api/src/stripe_client.rs:130-147` -- `create_refund`. Phase 2 calls this
  ONLY on `dispute.closed:lost` (we have to record the prorated refund
  remainder; Stripe's auto-withdrawal recovers the disputed amount, but
  we owe the customer the unused window minus the disputed amount).

## 2. Event handling matrix (Option B: pause-and-resume)

Each row maps a Stripe event to: contract status transition, side effects
on the resource, what gets logged, what alerts fire, what row is upserted
in `contract_disputes`. "PI" = `payment_intent`. "alert" = `tracing::warn!`
plus Telegram pipeline bot ping via `send_ops_alert` helper (Phase 2 adds
the helper to `notifications/telegram.rs`).

| Event                              | DB row action                                    | Contract state action                                                    | Resource action                                                       | Log level | Alert |
|------------------------------------|--------------------------------------------------|--------------------------------------------------------------------------|-----------------------------------------------------------------------|-----------|-------|
| `charge.dispute.created`           | INSERT or UPDATE on `stripe_dispute_id`          | `pause_contract(reason="stripe_dispute:<id>")`. Idempotent: replay = no-op. Records `paused_at_ns`. Operational contracts only -- terminal contracts get an audit event but no transition. | dc-agent polling loop (Phase 2) sees `status=paused` and STOPS the VM (does not destroy). | warn      | yes   |
| `charge.dispute.updated`           | UPDATE row (status, evidence_due_by, raw_event)  | None.                                                                    | None.                                                                 | info      | no    |
| `charge.dispute.closed` (won)      | UPDATE row (status='won', closed_at)             | `resume_contract`: clears `paused_at_ns`, credits elapsed pause to `total_paused_ns`, transitions back to `active`. Idempotent. Emits `dispute_resolved` event. | dc-agent restarts the VM on next poll. | info      | no    |
| `charge.dispute.closed` (lost)     | UPDATE row (status='lost', closed_at)            | `terminate_contract_for_dispute_lost`: pause -> cancelled, payment_status='disputed'. `mark_contract_resource_for_deletion` invoked. Idempotent. | VM destroyed by existing termination loop. Refund recorded with prorated remainder (paused time credited). Idempotency key: `dispute:<stripe_dispute_id>`. | warn      | yes   |
| `charge.dispute.funds_withdrawn`   | UPDATE row (funds_withdrawn_at)                  | No-op on contract. Audit trail only.                                     | None.                                                                 | warn      | yes   |

Justifications, with citations:

- Pause is reversible (PHASE 1 implements it via `Paused` state; no VM
  destruction on `dispute.created`). Lost disputes terminate the contract
  AFTER the pause, ensuring `total_paused_ns` is non-zero and the prorated
  refund credits the customer for the dispute window.
- The Stripe refund API call (`create_refund`) IS issued on `dispute.closed`
  (lost) -- but only for the prorated remainder beyond the disputed amount.
  Stripe's auto-withdrawal handles the disputed amount; the refund flow
  records what we owe the customer for unused future time.
- `cancel_contract` (rental.rs:566) is NOT reused directly: it requires a
  `cancelled_by_pubkey` and gates on requester equality. Instead, the
  Phase 1 helpers in `dispute.rs` use a synthetic `system-stripe-dispute`
  actor and skip the auth check (the system is the actor).
- Cancellable check before transitioning matches `is_cancellable()` at
  contract_status.rs:79-90 -- `Paused` is cancellable, so
  `terminate_contract_for_dispute_lost` works on a paused contract.

## 3. Schema change (PHASE 1, IMPLEMENTED)

Path: `api/migrations_pg/043_dispute_pause_state.sql`.

```sql
CREATE TABLE contract_disputes (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    stripe_dispute_id TEXT NOT NULL UNIQUE,
    stripe_charge_id TEXT NOT NULL,
    stripe_payment_intent_id TEXT,
    reason TEXT,
    status TEXT NOT NULL,
    amount_cents BIGINT NOT NULL,
    currency TEXT NOT NULL,
    evidence_due_by_ns BIGINT,
    funds_withdrawn_at_ns BIGINT,
    closed_at_ns BIGINT,
    raw_event JSONB NOT NULL,
    created_at_ns BIGINT NOT NULL,
    updated_at_ns BIGINT NOT NULL
);
CREATE INDEX idx_contract_disputes_contract ON contract_disputes(contract_id)
    WHERE contract_id IS NOT NULL;
CREATE INDEX idx_contract_disputes_charge ON contract_disputes(stripe_charge_id);
CREATE INDEX idx_contract_disputes_status ON contract_disputes(status);

ALTER TABLE contract_sign_requests
    ADD COLUMN paused_at_ns BIGINT,
    ADD COLUMN total_paused_ns BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN pause_reason TEXT;
CREATE INDEX idx_contract_sign_requests_paused
    ON contract_sign_requests (paused_at_ns)
    WHERE paused_at_ns IS NOT NULL;
```

Notes:
- `contract_id` is nullable so dispute rows can be persisted even when the
  charge is for a subscription that has no marketplace contract attached
  yet (e.g. Decent Agents subscription billed via the new product).
- `amount_cents` matches Stripe's native unit. We deliberately do not
  convert to e9s on insert -- raw_event has the source of truth, and our
  reporting code converts on read.
- `raw_event JSONB` mirrors the pattern used for Stripe payloads we want
  to inspect later (currently we discard them; this is the fix).
- `evidence_due_by_ns` is nanoseconds since Unix epoch, matching the rest
  of the codebase (`crate::now_ns`). Conversion from Stripe seconds is
  `seconds * 1_000_000_000`.
- `paused_at_ns` is partial-indexed; the dc-agent poll (Phase 2) cheaply
  filters paused rows.

## 4. Code diffs (PHASE 2)

### 4.a `api/src/openapi/webhooks.rs` -- new Stripe types

Insert immediately after `StripeSubscriptionItems` block, before line 83.
This keeps deserialization types co-located with the rest of the Stripe
schema definitions.

```diff
@@ webhooks.rs:78-82 (existing) @@
 #[derive(Debug, Deserialize)]
 struct StripePrice {
     id: String,
 }
+
+// Dispute webhook types -- charge.dispute.{created,updated,closed,funds_withdrawn}
+#[derive(Debug, Deserialize)]
+struct StripeDispute {
+    id: String,
+    charge: String,
+    payment_intent: Option<String>,
+    amount: i64,
+    currency: String,
+    reason: String,
+    status: String,
+    evidence_details: Option<StripeDisputeEvidenceDetails>,
+    metadata: Option<serde_json::Value>,
+}
+
+#[derive(Debug, Deserialize)]
+struct StripeDisputeEvidenceDetails {
+    due_by: Option<i64>, // Unix seconds
+}
```

### 4.b `api/src/openapi/webhooks.rs` -- new dispatch arms

Inserted after the `invoice.payment_failed` arm (line 671) and before the
default arm (line 677). Five lines of context each side:

```diff
@@ webhooks.rs:667-682 @@
                 // The subscription status will be updated by customer.subscription.updated webhook
                 // which Stripe sends when a subscription enters past_due status
             }
         }

+        "charge.dispute.created" => {
+            handle_dispute_created(db.as_ref(), &event.data.object).await?;
+        }
+        "charge.dispute.updated" => {
+            handle_dispute_updated(db.as_ref(), &event.data.object).await?;
+        }
+        "charge.dispute.closed" => {
+            handle_dispute_closed(db.as_ref(), &event.data.object).await?;
+        }
+        "charge.dispute.funds_withdrawn" => {
+            handle_dispute_funds_withdrawn(db.as_ref(), &event.data.object).await?;
+        }
         _ => {
             tracing::debug!("Unhandled event type: {}", event.event_type);
         }
     }
```

### 4.c `api/src/openapi/webhooks.rs` -- handler functions

Inserted as private free functions just before the `chatwoot_webhook`
handler at line 712 so they sit inside the same module as the dispatch.
All four share a single helper for upsert + lookup. Error handling matches
the existing pattern: DB errors map to 500, parse errors to 400, but the
handler never silently swallows -- every branch logs.

```rust
async fn parse_dispute(object: &serde_json::Value) -> Result<StripeDispute, PoemError> {
    serde_json::from_value(object.clone()).map_err(|e| {
        tracing::error!("Failed to parse dispute payload: {:#}", e);
        PoemError::from_string(
            format!("Invalid dispute data: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })
}

async fn handle_dispute_created(
    db: &Arc<Database>,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object).await?;
    tracing::warn!(
        "Stripe dispute opened: id={} charge={} amount={} {} reason={} status={}",
        dispute.id, dispute.charge, dispute.amount, dispute.currency,
        dispute.reason, dispute.status
    );

    let contract_id = lookup_contract_for_charge(db, &dispute).await;
    let evidence_due_by_ns = dispute
        .evidence_details
        .as_ref()
        .and_then(|d| d.due_by)
        .map(|s| s * 1_000_000_000);

    db.upsert_contract_dispute(crate::database::ContractDisputeUpsert {
        contract_id: contract_id.as_deref(),
        stripe_dispute_id: &dispute.id,
        stripe_charge_id: &dispute.charge,
        stripe_payment_intent_id: dispute.payment_intent.as_deref(),
        reason: Some(&dispute.reason),
        status: &dispute.status,
        amount_cents: dispute.amount,
        currency: &dispute.currency,
        evidence_due_by_ns,
        funds_withdrawn_at_ns: None,
        closed_at_ns: None,
        raw_event: object,
    })
    .await
    .map_err(map_db_err)?;

    if let Some(cid) = contract_id {
        let pause_reason = format!("stripe_dispute:{}", dispute.id);
        if let Err(e) = db.pause_contract(&cid, &pause_reason).await {
            tracing::error!(
                "Failed to pause contract {} for dispute {}: {:#}",
                hex::encode(&cid), dispute.id, e
            );
            // Persist anyway -- alert and continue, do not 500 (Stripe will retry forever).
        }
        crate::notifications::telegram::send_ops_alert(&format!(
            "Stripe dispute OPENED for contract {}: id={} reason={} amount={} {}",
            hex::encode(&cid), dispute.id, dispute.reason, dispute.amount, dispute.currency
        )).await;
    } else {
        tracing::warn!(
            "Stripe dispute {} has no matching contract (charge={})",
            dispute.id, dispute.charge
        );
        crate::notifications::telegram::send_ops_alert(&format!(
            "Stripe dispute OPENED with NO matching contract: id={} charge={} reason={} amount={} {}",
            dispute.id, dispute.charge, dispute.reason, dispute.amount, dispute.currency
        )).await;
    }
    Ok(())
}

async fn handle_dispute_updated(
    db: &Arc<Database>,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object).await?;
    tracing::info!(
        "Stripe dispute updated: id={} status={}", dispute.id, dispute.status
    );
    let contract_id = lookup_contract_for_charge(db, &dispute).await;
    let evidence_due_by_ns = dispute
        .evidence_details
        .as_ref()
        .and_then(|d| d.due_by)
        .map(|s| s * 1_000_000_000);
    db.upsert_contract_dispute(crate::database::ContractDisputeUpsert {
        contract_id: contract_id.as_deref(),
        stripe_dispute_id: &dispute.id,
        stripe_charge_id: &dispute.charge,
        stripe_payment_intent_id: dispute.payment_intent.as_deref(),
        reason: Some(&dispute.reason),
        status: &dispute.status,
        amount_cents: dispute.amount,
        currency: &dispute.currency,
        evidence_due_by_ns,
        funds_withdrawn_at_ns: None,
        closed_at_ns: None,
        raw_event: object,
    })
    .await
    .map_err(map_db_err)?;
    Ok(())
}

async fn handle_dispute_closed(
    db: &Arc<Database>,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object).await?;
    let now_ns = crate::now_ns().map_err(map_db_err)?;
    let contract_id = lookup_contract_for_charge(db, &dispute).await;

    let outcome = dispute.status.as_str(); // "won" | "lost" | "warning_closed"
    match outcome {
        "won" => tracing::info!("Stripe dispute WON: id={}", dispute.id),
        "lost" => tracing::warn!(
            "Stripe dispute LOST: id={} amount={} {}",
            dispute.id, dispute.amount, dispute.currency
        ),
        other => tracing::info!(
            "Stripe dispute closed with status={}: id={}", other, dispute.id
        ),
    }

    db.upsert_contract_dispute(crate::database::ContractDisputeUpsert {
        contract_id: contract_id.as_deref(),
        stripe_dispute_id: &dispute.id,
        stripe_charge_id: &dispute.charge,
        stripe_payment_intent_id: dispute.payment_intent.as_deref(),
        reason: Some(&dispute.reason),
        status: outcome,
        amount_cents: dispute.amount,
        currency: &dispute.currency,
        evidence_due_by_ns: None,
        funds_withdrawn_at_ns: None,
        closed_at_ns: Some(now_ns),
        raw_event: object,
    })
    .await
    .map_err(map_db_err)?;

    if let Some(cid) = contract_id {
        match outcome {
            "won" => {
                // Resume the contract; dc-agent restarts the VM on next poll.
                if let Err(e) = db.resume_contract(&cid).await {
                    tracing::error!(
                        "Failed to resume contract {} for dispute {}: {:#}",
                        hex::encode(&cid), dispute.id, e
                    );
                }
            }
            "lost" => {
                if let Err(e) = db
                    .terminate_contract_for_dispute_lost(&cid, &dispute.id)
                    .await
                {
                    tracing::error!(
                        "Failed to terminate contract {} for dispute {}: {:#}",
                        hex::encode(&cid), dispute.id, e
                    );
                }
                // Issue prorated refund (paused time credited automatically by
                // calculate_prorated_refund). Idempotency key: dispute:<id>.
                // ... see spec section 4.f for refund flow ...
                crate::notifications::telegram::send_ops_alert(&format!(
                    "Stripe dispute LOST for contract {}: id={} amount={} {}",
                    hex::encode(&cid), dispute.id, dispute.amount, dispute.currency
                )).await;
            }
            _ => {} // warning_closed and other outcomes: no state change
        }
    }
    Ok(())
}

async fn handle_dispute_funds_withdrawn(
    db: &Arc<Database>,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object).await?;
    let now_ns = crate::now_ns().map_err(map_db_err)?;
    tracing::warn!(
        "Stripe dispute funds withdrawn: id={} amount={} {}",
        dispute.id, dispute.amount, dispute.currency
    );
    let contract_id = lookup_contract_for_charge(db, &dispute).await;
    db.upsert_contract_dispute(crate::database::ContractDisputeUpsert {
        contract_id: contract_id.as_deref(),
        stripe_dispute_id: &dispute.id,
        stripe_charge_id: &dispute.charge,
        stripe_payment_intent_id: dispute.payment_intent.as_deref(),
        reason: Some(&dispute.reason),
        status: &dispute.status,
        amount_cents: dispute.amount,
        currency: &dispute.currency,
        evidence_due_by_ns: None,
        funds_withdrawn_at_ns: Some(now_ns),
        closed_at_ns: None,
        raw_event: object,
    })
    .await
    .map_err(map_db_err)?;

    crate::notifications::telegram::send_ops_alert(&format!(
        "Stripe dispute FUNDS WITHDRAWN: id={} contract={:?} amount={} {}",
        dispute.id, contract_id.as_ref().map(hex::encode),
        dispute.amount, dispute.currency
    )).await;
    Ok(())
}

fn map_db_err(e: anyhow::Error) -> PoemError {
    tracing::error!("Database error in dispute handler: {:#}", e);
    PoemError::from_string(
        format!("Database error: {}", e),
        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
    )
}

/// Resolve `contract_id` from `dispute.metadata.contract_id`, falling back
/// to a lookup by `stripe_payment_intent_id`. Returns None if neither yields
/// a row -- subscription disputes for the new Decent Agents product land here.
async fn lookup_contract_for_charge(
    db: &Arc<Database>,
    dispute: &StripeDispute,
) -> Option<Vec<u8>> {
    if let Some(meta) = dispute.metadata.as_ref() {
        if let Some(hex_id) = meta.get("contract_id").and_then(|v| v.as_str()) {
            if let Ok(bytes) = hex::decode(hex_id) {
                return Some(bytes);
            }
        }
    }
    if let Some(pi) = dispute.payment_intent.as_deref() {
        if let Ok(Some(id)) = db.get_contract_id_by_stripe_payment_intent(pi).await {
            return Some(id);
        }
    }
    None
}
```

`crate::notifications::telegram::send_ops_alert` is a thin Phase 2 wrapper
around `TelegramClient::from_env()?.send_message(chat_id, msg)` that reads
`TELEGRAM_OPS_CHAT_ID` from env and `tracing::warn!`s (does not silently
no-op) when Telegram is not configured. Add it to
`api/src/notifications/telegram.rs` (the `notifications` module is split
across `mod.rs`, `sms.rs`, `telegram.rs`; the helper belongs with the
Telegram client).

### 4.d `api/src/database/contracts/mod.rs` -- `ContractDisputeUpsert` struct (PHASE 1, IMPLEMENTED)

Re-exported from `dispute.rs`:

```rust
pub use dispute::ContractDisputeUpsert;
// (Phase 2 will also re-export `ResumeOutcome`.)

pub struct ContractDisputeUpsert<'a> {
    pub contract_id: Option<&'a [u8]>,
    pub stripe_dispute_id: &'a str,
    pub stripe_charge_id: &'a str,
    pub stripe_payment_intent_id: Option<&'a str>,
    pub reason: Option<&'a str>,
    pub status: &'a str,
    pub amount_cents: i64,
    pub currency: &'a str,
    pub evidence_due_by_ns: Option<i64>,
    pub funds_withdrawn_at_ns: Option<i64>,
    pub closed_at_ns: Option<i64>,
    pub raw_event: &'a serde_json::Value,
}
```

### 4.e `api/src/database/contracts/dispute.rs` -- helpers (PHASE 1, IMPLEMENTED)

See the file for full implementations. Public surface:

- `pub async fn upsert_contract_dispute(input: ContractDisputeUpsert<'_>) -> Result<()>`
  -- idempotent on `stripe_dispute_id`. ON CONFLICT preserves
  `created_at_ns`, refreshes mutable fields with COALESCE so a later replay
  cannot blank out a value set by an earlier event.
- `pub async fn pause_contract(contract_id: &[u8], reason: &str) -> Result<()>`
  -- transitions Active/Provisioned -> Paused, sets `paused_at_ns`,
  `pause_reason`. Replay with same reason: no-op. Replay with different
  reason: loud failure.
- `pub async fn resume_contract(contract_id: &[u8]) -> Result<ResumeOutcome>`
  -- credits `now - paused_at_ns` to `total_paused_ns`, transitions to
  Active. Idempotent (no-op when not paused).
- `pub async fn terminate_contract_for_dispute_lost(contract_id: &[u8], stripe_dispute_id: &str) -> Result<()>`
  -- transitions to Cancelled with `payment_status='disputed'`,
  `mark_contract_resource_for_deletion`. Idempotent (terminal short-circuit
  emits an audit event but no second history row).
- `pub async fn get_contract_id_by_stripe_payment_intent(pi: &str) -> Result<Option<Vec<u8>>>`
  -- Phase 2 fallback lookup.

### 4.f `api/src/database/contracts/payment.rs` -- prorated refund credit (PHASE 1, IMPLEMENTED)

`calculate_prorated_refund` now takes `total_paused_ns: i64`. Paused
intervals are subtracted from billable time:

```rust
let billable_used_ns = elapsed_ns.saturating_sub(total_paused_ns.max(0));
```

Existing call sites in `cancel_contract` (rental.rs) and `process_icpay_refund`
(payment.rs) read `total_paused_ns` from the row via the new
`get_total_paused_ns(&[u8])` helper before calling the function. Six
existing test cases pass `0`; one new test
`test_calculate_prorated_refund_credits_paused_time` covers the credit math.

## 5. State machine analysis (PHASE 1, IMPLEMENTED)

`common/src/contract_status.rs`: `Paused` variant added. Transitions:

- `Active -> Paused`, `Provisioned -> Paused`
- `Paused -> Active`, `Paused -> Provisioned` (resume targets)
- `Paused -> Cancelled`, `Paused -> Expired` (terminal-bypass on lost
  dispute or contract end during pause)

Properties: `is_terminal=false`, `is_cancellable=true`,
`is_operational=false`. The "not operational" claim is load-bearing for
the dc-agent polling loop (Phase 2): a paused contract has its VM
stopped, so the loop must skip provisioning/maintenance for it.

Two new tests added: `test_valid_transitions_paused`,
`test_paused_is_cancellable_not_operational_not_terminal`. Existing
exhaustive-enumeration tests
(`test_terminal_states_cannot_transition`, `test_is_*`) extended to
cover the new variant.

## 6. Test plan

Phase 1 (IMPLEMENTED) tests in
`api/src/database/contracts/tests.rs`:

1. `test_upsert_dispute_idempotent` -- replay with refreshed payload keeps
   row count = 1; `created_at_ns` preserved; `status`/`raw_event` updated.
2. `test_pause_contract_idempotent` -- pause transitions to `paused`,
   sets columns, writes one history row + one event. Replay with same
   reason: no extra rows, `paused_at_ns` does NOT bump. Conflicting
   reason: loud failure.
3. `test_resume_contract_credits_paused_time` -- pause + sleep + resume
   yields `resumed=true`, `credited_pause_ns >= 10ms`,
   `total_paused_ns == credited_pause_ns`, columns cleared, `resumed`
   event recorded. Second resume: no-op.
4. `test_terminate_for_dispute_lost` -- pause + terminate yields
   `cancelled` + `payment_status='disputed'` + history row +
   `dispute_lost` event referencing the dispute id. Replay on terminal:
   no second history row, but a fresh audit event preserves replay
   visibility.
5. `test_calculate_prorated_refund_credits_paused_time` -- baseline (no
   pause) gives ~40% refund; with 30% paused window credited, gives ~70%
   refund; full pause gives full refund; negative input is sanitized.

Phase 2 tests (DEFERRED) at the HTTP layer:

6. `test_dispute_signature_mismatch_returns_400` -- bogus
   `stripe-signature` header, response is 400, no DB row inserted.
7. `test_dispute_with_unknown_charge_persists_orphan_row` -- metadata has
   no contract_id, PI lookup misses; `contract_id IS NULL` row inserted,
   no `contract_events` row, Telegram alert "NO matching contract".
8. `test_dispute_funds_withdrawn_sets_timestamp` -- row exists with
   `funds_withdrawn_at_ns IS NOT NULL`, contract state untouched.

## 7. Acceptance checklist

Mapped to the four bullets in issue #408 and the decision in #421.

- [x] Pause-vs-terminate decision recorded (#421 -> Option B)
  - file: this spec section 9, R3
- [x] Schema for dispute persistence
  - file: `api/migrations_pg/043_dispute_pause_state.sql`
- [x] DB helpers (idempotent) for pause/resume/terminate-lost + dispute upsert
  - file: `api/src/database/contracts/dispute.rs`
- [x] Refund credits paused time
  - file: `api/src/database/contracts/payment.rs`
- [x] State machine has `Paused` with reversible transitions
  - file: `common/src/contract_status.rs`
- [ ] Webhook routes for all four events (PHASE 2)
  - file: `api/src/openapi/webhooks.rs`
- [ ] dc-agent polling loop skips paused contracts (PHASE 2)
  - file: `dc-agent/src/main.rs` (or wherever the loop reads `status`)
- [ ] `send_ops_alert` helper (PHASE 2)
  - file: `api/src/notifications/telegram.rs`
- [ ] HTTP-layer tests (PHASE 2): signature mismatch, orphan dispute,
  funds_withdrawn

## 8. Follow-up issues to file

1. "Stripe dispute evidence submission" -- automated upload of order
   metadata, ToS acceptance, IP logs to `/v1/disputes/{id}/close`. Today
   we cede the dispute by default; basic evidence for non-fraudulent
   reasons would improve win rate.
2. "Admin UI: dispute dashboard" -- list `contract_disputes` rows with
   filters by status, contract, date.
3. "Subscription dispute coverage" -- once Decent Agents subscriptions
   ship, extend `lookup_contract_for_charge` to also resolve subscription
   IDs and pause the subscription (cancel-at-period-end semantics).

## 9. Risk and uncertainty

Each risk: confidence I am right, what I do not know, what to verify.

- R1 (conf 9/10): Stripe `charge.dispute.*` payloads use `charge` and
  optional `payment_intent` string fields. Verify the JSON shape against
  Stripe API docs for the version pinned in `Cargo.toml`. The struct
  `StripeDispute` may need additional `#[serde(default)]` or `Option`
  wrappers if Stripe omits fields in older API versions.
- R2 (conf 8/10): `send_ops_alert` is a Phase 2 helper; the rest of the
  codebase uses `TelegramClient::from_env()?.send_message(chat_id, msg)`.
  Phase 2 adds the wrapper to `api/src/notifications/telegram.rs` reading
  `TELEGRAM_OPS_CHAT_ID` from env. If the env var is absent, the helper
  emits `tracing::warn!` (per project rule "BE LOUD ABOUT
  MISCONFIGURATIONS"), it does NOT silently no-op.
- R3 (RESOLVED 2026-04-25 per #421): pause-and-resume chosen over
  terminate-on-dispute. Reversible state preserves customer state on
  won disputes (no manual re-onboarding). The `Paused` state in
  `contract_status.rs` and the helpers in `dispute.rs` implement this
  cleanly; the prorated refund credits paused time so the customer pays
  only for billable usage. Phase 2 wires it into webhooks and the
  dc-agent loop.
- R4 (conf 9/10): the `stripe_payment_intent_id` column WAS storing
  Checkout Session IDs (`cs_*`) instead of real PaymentIntent IDs
  (`pi_*`). Migration 042 renamed the old column to
  `stripe_checkout_session_id` and added a real `stripe_payment_intent_id`
  populated from `session.payment_intent` at checkout completion. Phase 2
  fallback lookup `get_contract_id_by_stripe_payment_intent` therefore
  matches new rows; legacy rows fall back to checkout-session lookup.
- R5 (conf 9/10): The `charge.dispute.closed` payload has dispute
  `status` of `won` or `lost`. Stripe's `warning_closed` is also a
  closing terminal status for warnings (early dispute alerts). The plan
  handles all three by passing `status` through verbatim into the DB
  column.
- R6 (conf 10/10): migration number 043 confirmed at Phase 1 commit time
  (next free after 042).
- R7 (conf 9/10): `tracing::warn!` is the right level for `created`,
  `lost`, `funds_withdrawn` (per existing `invoice.payment_failed`
  pattern at webhooks.rs:655). `info!` for `won` and `updated`.
- R8 (conf 8/10): Telegram is a notification side-effect; if the
  Telegram client is misconfigured, the handler must NOT 500 (Stripe
  will retry forever). Phase 2 ensures `send_ops_alert` errors are
  warn-logged and absorbed.
- R9 (conf 5/10): Subscription metadata propagation -- I do NOT know
  whether dispute webhooks for the new Decent Agents subscription
  product will carry `metadata.contract_id`. Phase 2 must verify by
  inspecting how the subscription product sets
  `subscription_data.metadata` (or `payment_intent_data.metadata`).
  If missing, file a follow-up to add `contract_id` (or equivalent
  stable key) to subscription metadata.
- R10 (conf 9/10): test infra (`setup_test_db`) creates a fresh DB per
  test from a template; Phase 1 added 043 to both the migration list
  and the migration_hash function in `test_helpers.rs` so the template
  invalidates correctly.

Overall confidence the plan, if implemented as written for Phase 2,
ships production-ready: 8/10. Phase 1 confidence: 9/10 -- 22 dispute
tests + 7 prorated-refund tests pass against ephemeral PostgreSQL.
