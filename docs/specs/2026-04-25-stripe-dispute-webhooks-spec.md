# Stripe charge.dispute.* Webhook Handlers -- Implementation Plan

- Issue: decent-stuff/decent-cloud#408
- Date: 2026-04-25
- Author: Backend API agent
- Status: Plan only. No code in this commit.

## 0. Goal and non-goals

Goal: stop silently dropping Stripe `charge.dispute.created`,
`charge.dispute.updated`, `charge.dispute.closed`, and
`charge.dispute.funds_withdrawn` events. Persist every dispute, transition
the related contract to a state that stops billable usage, and surface the
event in logs and ops alerts.

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
- `api/src/database/contracts/payment.rs:101-146` -- prorated refund math.
- `api/src/database/contracts/rental.rs:560-731` -- `cancel_contract`,
  including stripe refund call at `:594-643`.
- `api/src/database/contracts/rental.rs:367-537` -- `reject_contract`, full
  refund path at `:396-470`.
- `api/src/database/contracts/extensions.rs:210-237` -- `insert_contract_event`
  signature.
- `common/src/contract_status.rs:36-108` -- state machine. Cancellable from
  Requested..Active. Terminal: Rejected, Cancelled, Expired.
- `api/src/stripe_client.rs:130-147` -- `create_refund`. Note: it accepts a
  `payment_intent_id`, but disputes are issued against a `charge_id`, not a
  payment_intent. Stripe creates the refund against the underlying charge
  automatically when given the PI; for disputes we never need to call
  `create_refund` because the dispute itself withdraws the funds.
- `api/migrations_pg/024_contract_events.sql` -- `contract_events` schema
  pattern; new dispute table follows same conventions.

## 2. Event handling matrix

Each row maps a Stripe event to: contract status transition, side effects on
the resource, what gets logged, what alerts fire, what row is upserted in
`contract_disputes`. "PI" = `payment_intent`. "alert" = `tracing::error!`
plus Telegram pipeline bot ping (existing `notifications/telegram` client
already wired into webhooks; reused here without new code paths).

| Event                              | DB row action                                    | Contract state action                                                    | Resource action                                                       | Log level | Alert |
|------------------------------------|--------------------------------------------------|--------------------------------------------------------------------------|-----------------------------------------------------------------------|-----------|-------|
| `charge.dispute.created`           | INSERT or UPDATE on `stripe_dispute_id`          | If contract is in any cancellable status (Requested..Active): same path as `cancel_contract` BUT with `cancel_memo = "stripe_dispute:<id>"` and NO new refund call (Stripe already pulled the funds). For terminal contracts: log + audit only. | Reuse `mark_contract_resource_for_deletion` (rental.rs:722). VM/agent terminated. | warn      | yes   |
| `charge.dispute.updated`           | UPDATE row (status, evidence_due_by, raw_event)  | None unless status changed to `lost`/`won` (we still get `closed` for that, so usually no-op). | None                                                                  | info      | no    |
| `charge.dispute.closed` (won)      | UPDATE row (status='won', closed_at)             | Contract is already Cancelled by `created`. No revival. Just mark dispute won and emit `dispute_resolved` contract event. Operations decides whether to manually re-onboard the customer. | None (resource already gone) | info | no |
| `charge.dispute.closed` (lost)     | UPDATE row (status='lost', closed_at)            | No-op on contract (already Cancelled). Funds already withdrawn. Emit `dispute_lost` contract event with amount. | None                  | warn      | yes   |
| `charge.dispute.funds_withdrawn`   | UPDATE row (funds_withdrawn_at)                  | No-op on contract. Audit trail only.                                     | None                                                                  | warn      | yes   |

Justifications, with citations:

- "Use existing `cancel_contract` path with a system actor" not chosen, see
  5.e: that function takes `cancelled_by_pubkey` and is gated to the
  requester. Instead we add a thin sibling `cancel_contract_for_dispute`
  that reuses the same status transition and resource cleanup code path.
- We skip the Stripe refund API call (rental.rs:611) because for disputes
  Stripe pulls the funds via the dispute mechanism, not via refund. Calling
  `create_refund` on a disputed charge yields a Stripe API error
  `charge_disputed`. Confidence 9/10.
- Cancellable check before transitioning matches the pre-existing rule at
  rental.rs:581-586 -- a contract already terminal stays terminal; we
  record the dispute but do not try a forbidden state transition.

## 3. Schema change

Path: `api/migrations_pg/041_contract_disputes.sql` (next free number;
current max is 040).

```sql
-- Stripe charge.dispute.* webhook persistence
-- Idempotency: stripe_dispute_id is UNIQUE; webhook handler upserts.
CREATE TABLE contract_disputes (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA REFERENCES contract_sign_requests(contract_id),
    stripe_dispute_id TEXT NOT NULL UNIQUE,
    stripe_charge_id TEXT NOT NULL,
    stripe_payment_intent_id TEXT,
    reason TEXT NOT NULL,
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

CREATE INDEX idx_contract_disputes_contract ON contract_disputes(contract_id);
CREATE INDEX idx_contract_disputes_charge ON contract_disputes(stripe_charge_id);
CREATE INDEX idx_contract_disputes_status ON contract_disputes(status);
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

## 4. Code diffs

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
         // Note: payment_intent.succeeded and payment_intent.payment_failed webhooks are NOT used.
         // We use checkout.session.completed which already sets payment_status and has the contract_id.
         // Stripe Checkout generates its own PaymentIntent internally, but we link contracts by
         // checkout session ID, not payment intent ID.
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
        reason: &dispute.reason,
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
        if let Err(e) = db
            .cancel_contract_for_dispute(&cid, &dispute.id, dispute.amount)
            .await
        {
            tracing::error!(
                "Failed to cancel contract {} for dispute {}: {:#}",
                hex::encode(&cid), dispute.id, e
            );
            // Persist anyway -- alert and continue, do not 500 (Stripe will retry forever).
            // The dispute row is committed; an operator can finish the cancel manually.
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
        reason: &dispute.reason,
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
        reason: &dispute.reason,
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
        let event_type = if outcome == "won" { "dispute_resolved" } else { "dispute_lost" };
        let details = format!(
            "Stripe dispute {}: id={} amount={} {}",
            outcome, dispute.id, dispute.amount, dispute.currency
        );
        if let Err(e) = db
            .insert_contract_event(&cid, event_type, None, None, "system", Some(&details))
            .await
        {
            tracing::warn!("Failed to record dispute close event for contract {}: {:#}",
                hex::encode(&cid), e);
        }
        if outcome == "lost" {
            crate::notifications::telegram::send_ops_alert(&format!(
                "Stripe dispute LOST for contract {}: id={} amount={} {}",
                hex::encode(&cid), dispute.id, dispute.amount, dispute.currency
            )).await;
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
        reason: &dispute.reason,
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

Note: `crate::notifications::telegram::send_ops_alert` is a thin wrapper
that already exists per the rest of the codebase pattern (see imports at
the top of `webhooks.rs`). If only the typed `TelegramClient` exists, the
implementation step lifts the alert call into the same form used by other
handlers (`TelegramClient::from_env()?.send_message(...)`). Confidence on
exact symbol name: 6/10 -- verify before implementation. See risk 9.

### 4.d `api/src/database/contracts/mod.rs` -- `ContractDisputeUpsert` struct

Added next to `SubscriptionEventInput` (a similar input-struct pattern that
already exists in the same module). Public so the webhook handler can
construct it.

```rust
pub struct ContractDisputeUpsert<'a> {
    pub contract_id: Option<&'a [u8]>,
    pub stripe_dispute_id: &'a str,
    pub stripe_charge_id: &'a str,
    pub stripe_payment_intent_id: Option<&'a str>,
    pub reason: &'a str,
    pub status: &'a str,
    pub amount_cents: i64,
    pub currency: &'a str,
    pub evidence_due_by_ns: Option<i64>,
    pub funds_withdrawn_at_ns: Option<i64>,
    pub closed_at_ns: Option<i64>,
    pub raw_event: &'a serde_json::Value,
}
```

### 4.e `api/src/database/contracts/payment.rs` -- new methods

```rust
impl Database {
    /// Idempotent upsert keyed on stripe_dispute_id.
    /// Stripe replays webhooks; we MUST NOT insert duplicates. ON CONFLICT
    /// preserves the original created_at_ns and only refreshes mutable
    /// fields and raw_event.
    pub async fn upsert_contract_dispute(
        &self,
        input: ContractDisputeUpsert<'_>,
    ) -> Result<()> {
        let now_ns = crate::now_ns()?;
        sqlx::query!(
            r#"INSERT INTO contract_disputes
               (contract_id, stripe_dispute_id, stripe_charge_id, stripe_payment_intent_id,
                reason, status, amount_cents, currency, evidence_due_by_ns,
                funds_withdrawn_at_ns, closed_at_ns, raw_event,
                created_at_ns, updated_at_ns)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)
               ON CONFLICT (stripe_dispute_id) DO UPDATE SET
                 status = EXCLUDED.status,
                 reason = EXCLUDED.reason,
                 evidence_due_by_ns = COALESCE(EXCLUDED.evidence_due_by_ns,
                                               contract_disputes.evidence_due_by_ns),
                 funds_withdrawn_at_ns = COALESCE(EXCLUDED.funds_withdrawn_at_ns,
                                                  contract_disputes.funds_withdrawn_at_ns),
                 closed_at_ns = COALESCE(EXCLUDED.closed_at_ns, contract_disputes.closed_at_ns),
                 raw_event = EXCLUDED.raw_event,
                 updated_at_ns = EXCLUDED.updated_at_ns"#,
            input.contract_id,
            input.stripe_dispute_id,
            input.stripe_charge_id,
            input.stripe_payment_intent_id,
            input.reason,
            input.status,
            input.amount_cents,
            input.currency,
            input.evidence_due_by_ns,
            input.funds_withdrawn_at_ns,
            input.closed_at_ns,
            input.raw_event,
            now_ns,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Look up contract_id by stripe_payment_intent_id (fallback path
    /// when dispute metadata.contract_id is missing).
    pub async fn get_contract_id_by_stripe_payment_intent(
        &self,
        pi: &str,
    ) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query!(
            "SELECT contract_id FROM contract_sign_requests WHERE stripe_payment_intent_id = $1 LIMIT 1",
            pi
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.contract_id))
    }

    /// Cancel a contract because Stripe issued a chargeback. Skips the
    /// refund API call (Stripe already pulled funds) and skips the
    /// caller-pubkey check (the system is the actor).
    /// Idempotent: if the contract is already in a terminal status, only
    /// emits an audit event.
    pub async fn cancel_contract_for_dispute(
        &self,
        contract_id: &[u8],
        stripe_dispute_id: &str,
        amount_cents: i64,
    ) -> Result<()> {
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow::anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;
        let cancel_memo = format!(
            "Stripe chargeback {}: amount={} cents", stripe_dispute_id, amount_cents
        );
        let now_ns = crate::now_ns()?;
        let cancelled = ContractStatus::Cancelled.to_string();

        let current: ContractStatus = contract.status.parse().map_err(|e| {
            anyhow::anyhow!("Contract status invalid '{}': {}", contract.status, e)
        })?;
        if current.is_terminal() {
            // Already terminal -- record the dispute event but do not transition.
            self.insert_contract_event(
                contract_id, "dispute_opened", Some(&contract.status), None,
                "system", Some(&cancel_memo)
            ).await?;
            return Ok(());
        }
        if !current.can_transition_to(ContractStatus::Cancelled) {
            return Err(anyhow::anyhow!(
                "Contract status {} cannot transition to Cancelled (chargeback)", contract.status
            ));
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, payment_status = 'disputed' WHERE contract_id = $3",
            cancelled, now_ns, contract_id
        ).execute(&mut *tx).await?;
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id, contract.status, cancelled,
            &b"system-stripe-dispute"[..], now_ns, Some(cancel_memo.as_str())
        ).execute(&mut *tx).await?;
        sqlx::query!(
            "INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at) VALUES ($1, 'dispute_opened', $2, $3, 'system', $4, $5)",
            contract_id, contract.status, cancelled,
            Some(cancel_memo.as_str()), now_ns
        ).execute(&mut *tx).await?;
        tx.commit().await?;

        if let Err(e) = self.mark_contract_resource_for_deletion(contract_id).await {
            tracing::warn!(
                "Failed to mark cloud resource for deletion for disputed contract {}: {}",
                hex::encode(contract_id), e
            );
        }
        Ok(())
    }
}
```

## 5. State machine analysis

Existing state machine: see `common/src/contract_status.rs:36-108`.

- All non-terminal statuses (`Requested`, `Pending`, `Accepted`,
  `Provisioning`, `Provisioned`, `Active`) can transition to `Cancelled`
  (`is_cancellable() == true` at lines 79-89).
- Terminal states (`Rejected`, `Cancelled`, `Expired`) cannot transition
  further (line 64).

Decision: NO new state added. `Cancelled` already covers the chargeback
outcome. Justification:
- The product semantics of "user disputed the charge" map cleanly onto
  "contract is dead, resource is gone, no further billing." That is what
  `Cancelled` already means everywhere else (`is_cancellable=true ->
  Cancelled` is reached today via tenant action; for disputes it is reached
  via a system actor).
- A new `Disputed` state would force every consumer of `ContractStatus`
  (`api/src/database/contracts/extensions.rs`, `api/src/openapi/contracts.rs`,
  every test, the SvelteKit UI) to handle a fourth terminal value. That
  is cost without benefit -- the dispute itself is recorded in
  `contract_disputes` and `contract_events`, which is the right place for
  the financial-event detail. Confidence 9/10.
- An "open dispute" is not a long-running contract state because we MUST
  stop the resource immediately on `dispute.created`. There is no
  reversible "frozen" state in the existing pause-vs-terminate model:
  rental.rs:722 `mark_contract_resource_for_deletion` is the only
  termination path and it is destructive. A reversible pause would be a
  separate, larger ticket.

## 6. Test plan

Test file: `api/src/database/contracts/tests.rs` (extend) and a new
`api/src/openapi/webhooks_dispute_tests.rs` (or a new sub-module in
webhooks.rs `#[cfg(test)] mod dispute_tests`). Final placement chosen at
implementation time to match the convention in the file (mod tests already
exists at line 1351 of webhooks.rs -- prefer extending it).

Test cases:

1. `test_dispute_created_on_active_contract_cancels_and_inserts_row`
   Assertions:
   - Pre: contract status = `active`, payment_status = `succeeded`.
   - After `upsert_contract_dispute` + `cancel_contract_for_dispute`:
     - `contract_disputes` has exactly 1 row, `status=needs_response`,
       `stripe_dispute_id=du_test_1`.
     - `contract_sign_requests.status = 'cancelled'`.
     - `contract_sign_requests.payment_status = 'disputed'`.
     - `contract_events` has a row with `event_type='dispute_opened'`,
       `actor='system'`.

2. `test_dispute_created_replay_is_idempotent`
   Assertions:
   - Run upsert + cancel twice with the same payload.
   - Exactly 1 row in `contract_disputes` after both calls.
   - Exactly 1 status_history row going `active -> cancelled` (the second
     call must short-circuit on `is_terminal()` and NOT insert another
     history row).
   - Exactly 1 `dispute_opened` event in `contract_events` from the first
     call AND exactly 1 from the second -- no, wait: the second call DOES
     emit an audit event (per spec section 4.e). So assert 2 events,
     never 3. Document this on the test.

3. `test_dispute_closed_won_emits_resolved_event_no_state_change`
   Pre: contract is already `cancelled` after a prior dispute_created.
   Action: handle_dispute_closed with status="won".
   Assertions:
   - `contract_disputes.status = 'won'`, `closed_at_ns IS NOT NULL`.
   - `contract_events` gains a `dispute_resolved` row.
   - `contract_sign_requests.status` remains `cancelled`.

4. `test_dispute_closed_lost_emits_lost_event`
   Same as 3 but status="lost":
   - `contract_disputes.status = 'lost'`.
   - `contract_events` gains a `dispute_lost` row with the amount in
     `details`.
   - Telegram alert helper invoked (assert via injected mock or capture
     the formatted string -- see risk 9).

5. `test_dispute_signature_mismatch_returns_401`
   At HTTP layer: post a `charge.dispute.created` body with a bogus
   `stripe-signature` header.
   Assert: response is 401, no DB row inserted.

6. `test_dispute_with_unknown_charge_persists_orphan_row`
   The dispute's metadata has no `contract_id` and PI lookup returns None.
   Assert:
   - `contract_disputes` row inserted with `contract_id IS NULL`.
   - No `contract_events` row (no contract to attach to).
   - Telegram alert invoked with "NO matching contract".

7. `test_dispute_funds_withdrawn_sets_timestamp`
   Assert:
   - Row exists with `funds_withdrawn_at_ns IS NOT NULL`.
   - Contract state untouched (the cancel happened on `created`).

### Test skeleton 1 (simplest -- positive path)

```rust
#[tokio::test]
async fn test_dispute_created_on_active_contract_cancels_and_inserts_row() {
    let db = setup_test_db().await;
    let contract_id = vec![42u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    insert_contract_request(&db, &contract_id, &requester_pk, &provider_pk,
        "off-disp", 0, "active").await;

    let raw = serde_json::json!({
        "id": "du_test_1", "charge": "ch_test_1",
        "payment_intent": "pi_test_1", "amount": 500,
        "currency": "usd", "reason": "fraudulent",
        "status": "needs_response", "evidence_details": {"due_by": 1_700_000_000}
    });

    db.upsert_contract_dispute(ContractDisputeUpsert {
        contract_id: Some(&contract_id),
        stripe_dispute_id: "du_test_1",
        stripe_charge_id: "ch_test_1",
        stripe_payment_intent_id: Some("pi_test_1"),
        reason: "fraudulent",
        status: "needs_response",
        amount_cents: 500,
        currency: "usd",
        evidence_due_by_ns: Some(1_700_000_000_000_000_000),
        funds_withdrawn_at_ns: None,
        closed_at_ns: None,
        raw_event: &raw,
    }).await.unwrap();

    db.cancel_contract_for_dispute(&contract_id, "du_test_1", 500).await.unwrap();

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_disputes WHERE stripe_dispute_id = 'du_test_1'"
    ).fetch_one(&db.pool).await.unwrap();
    assert_eq!(count, 1);

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    assert_eq!(contract.payment_status, "disputed");

    let events = db.get_contract_events(&contract_id).await.unwrap();
    assert!(events.iter().any(|e|
        e.event_type == "dispute_opened" && e.actor == "system"
    ));
}
```

### Test skeleton 2 (trickiest -- idempotent replay)

```rust
#[tokio::test]
async fn test_dispute_created_replay_is_idempotent() {
    let db = setup_test_db().await;
    let contract_id = vec![43u8; 32];
    insert_contract_request(&db, &contract_id, &[1u8; 32], &[2u8; 32],
        "off-disp-2", 0, "active").await;

    let raw = serde_json::json!({
        "id": "du_replay", "charge": "ch_replay", "payment_intent": null,
        "amount": 1000, "currency": "usd", "reason": "duplicate",
        "status": "needs_response"
    });
    let upsert = || ContractDisputeUpsert {
        contract_id: Some(&contract_id),
        stripe_dispute_id: "du_replay",
        stripe_charge_id: "ch_replay",
        stripe_payment_intent_id: None,
        reason: "duplicate", status: "needs_response",
        amount_cents: 1000, currency: "usd",
        evidence_due_by_ns: None, funds_withdrawn_at_ns: None, closed_at_ns: None,
        raw_event: &raw,
    };

    // First delivery
    db.upsert_contract_dispute(upsert()).await.unwrap();
    db.cancel_contract_for_dispute(&contract_id, "du_replay", 1000).await.unwrap();

    // Second delivery (Stripe replay)
    db.upsert_contract_dispute(upsert()).await.unwrap();
    db.cancel_contract_for_dispute(&contract_id, "du_replay", 1000).await.unwrap();

    let dispute_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_disputes WHERE stripe_dispute_id = 'du_replay'"
    ).fetch_one(&db.pool).await.unwrap();
    assert_eq!(dispute_count, 1, "ON CONFLICT must keep a single row");

    let history_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_status_history WHERE contract_id = $1 AND new_status = 'cancelled'"
    ).bind(&contract_id).fetch_one(&db.pool).await.unwrap();
    assert_eq!(history_count, 1, "second call must short-circuit on is_terminal()");

    let opened_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_events WHERE contract_id = $1 AND event_type = 'dispute_opened'"
    ).bind(&contract_id).fetch_one(&db.pool).await.unwrap();
    assert_eq!(opened_events, 2, "first transitions, second is audit-only -- both emit dispute_opened");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
}
```

## 7. Acceptance checklist

Mapped to the four bullets in issue #408.

- [ ] Webhook routes for all four events
  - file: `api/src/openapi/webhooks.rs:~672-680` (new match arms, see 4.b)
- [ ] Persist dispute records linked to the originating contract/subscription
  - file: `api/migrations_pg/041_contract_disputes.sql` (table)
  - file: `api/src/database/contracts/payment.rs:~upsert_contract_dispute`
- [ ] On dispute.created: terminate/suspend the related Decent Agent, mark contract
  - file: `api/src/database/contracts/payment.rs:~cancel_contract_for_dispute`
  - file: `api/src/openapi/webhooks.rs:~handle_dispute_created` (calls cancel + alert)
  - resource teardown: reuses `mark_contract_resource_for_deletion`
    (rental.rs:722).
- [ ] On dispute.closed (won/lost): update state and any financial adjustments
  - file: `api/src/openapi/webhooks.rs:~handle_dispute_closed`
  - financial adjustment: NONE on our side; Stripe already moved funds.
    Documented at section 2.
- [ ] Idempotent replay handling (Stripe retries)
  - SQL: `UNIQUE(stripe_dispute_id)` + `ON CONFLICT DO UPDATE`
  - State: `cancel_contract_for_dispute` short-circuits on
    `is_terminal()`.
- [ ] Unit tests per event type using Stripe test fixtures
  - covered by tests 1-7 above.
- [ ] Simulated dispute events transition state correctly -> tests 1, 3, 4.
- [ ] Duplicate webhook deliveries don't double-apply -> test 2.
- [ ] Logs + ops alert on every dispute event -> assertions in tests 4 and 6;
  `tracing::warn!` calls in handlers; `send_ops_alert` in handlers.

## 8. Follow-up issues to file (after this plan, before implementation)

1. "Stripe dispute evidence submission" -- automated upload of order
   metadata, ToS acceptance, IP logs to `/v1/disputes/{id}/close`. Today
   we fully cede the dispute; we should at least submit basic evidence
   for non-fraudulent reasons.
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
  wrappers if Stripe omits fields in older API versions. (Was checked
  against current docs at plan-time but version pinning lives in the
  `stripe-rust` crate and may differ.)
- R2 (conf 6/10): `crate::notifications::telegram::send_ops_alert` symbol
  -- the rest of the codebase uses `TelegramClient::from_env()?
  .send_message(chat_id, msg)`. There is no `send_ops_alert` helper today.
  Implementation step must either add one (1 small wrapper in
  `notifications/telegram.rs` reading `TELEGRAM_OPS_CHAT_ID` from env) or
  call `send_message` directly. The plan above is written assuming the
  wrapper exists; first implementation PR should add it.
- R3 (conf 8/10): pause-agent path. There is no reversible pause for
  marketplace contracts; only `mark_contract_resource_for_deletion`
  (rental.rs:722). Before implementing, confirm with stakeholders that
  destroying the VM on `dispute.created` is acceptable. The alternative
  -- reversible freeze -- is a separate, larger feature.
- R4 (conf 7/10): `stripe_payment_intent_id` column today actually stores
  the checkout session ID `cs_*`, not the payment_intent ID `pi_*` (see
  payment.rs:17 -- the column is named `stripe_payment_intent_id` but
  written with `session.id`). The fallback lookup
  `get_contract_id_by_stripe_payment_intent` will therefore NOT match
  most existing rows. For dispute events specifically we expect to find
  the contract via `dispute.metadata.contract_id`, NOT via PI lookup, so
  this fallback is best-effort. Document at the call site. Pre-existing
  bug; out of scope to fix here -- file as a separate cleanup issue.
- R5 (conf 9/10): The `charge.dispute.closed` payload has dispute
  `status` of `won` or `lost`. Stripe's `warning_closed` is also a
  closing terminal status for warnings (early dispute alerts). The plan
  handles all three by passing `status` through verbatim into the DB
  column.
- R6 (conf 8/10): the migration number 041 is correct as of plan-time;
  re-check on implementation in case another PR lands first.
- R7 (conf 9/10): `tracing::warn!` is the right level for `created`,
  `lost`, `funds_withdrawn` (per existing `invoice.payment_failed`
  pattern at webhooks.rs:655). `info!` for `won` and `updated`.
- R8 (conf 7/10): Telegram is a notification side-effect; if the
  Telegram client is misconfigured, the handler must NOT 500 (Stripe
  will retry forever). The plan logs and continues. Verify in the
  `send_ops_alert` wrapper that errors are swallowed-with-log only.
- R9 (conf 5/10): I do NOT know whether dispute webhooks for the new
  Decent Agents subscription product will carry `metadata.contract_id`
  at all. If subscriptions create their own checkout sessions without
  per-contract metadata, every Decent Agents dispute will arrive as an
  orphan and we will fail to terminate the agent. Verify by inspecting
  how the new subscription product sets `subscription_data.metadata`
  (or `payment_intent_data.metadata`). If missing, file a follow-up to
  add `contract_id` (or equivalent stable key) to subscription metadata.
- R10 (conf 9/10): test infra (`setup_test_db` at
  `api/src/database/test_helpers.rs:968`) creates a fresh DB per test --
  the dispute tests can rely on an empty `contract_disputes` table at
  start. No cross-test pollution.

Overall confidence the plan, if implemented as written, ships
production-ready: 8/10. The two pinch points are R2 (alert wrapper
naming) and R9 (subscription metadata for the new product). Both are
verifiable in <30 min during implementation.
