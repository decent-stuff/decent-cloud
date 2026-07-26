//! Stripe dispute persistence + reversible pause/resume helpers.
//!
//! Phase 1 (this module): DB-layer primitives only -- idempotent dispute upsert,
//! pause/resume/terminate-on-lost helpers. Phase 2 wires these into webhook
//! handlers and the dc-agent polling loop (see follow-up ticket).
//!
//! Idempotency contract (re: Stripe replays):
//!  * `upsert_contract_dispute` keys on `stripe_dispute_id` (UNIQUE);
//!    second call updates mutable fields, leaves `created_at_ns` untouched.
//!  * `pause_contract` is a no-op when the contract is already paused with
//!    the same reason; mismatched reason returns an error (loud failure).
//!  * `resume_contract` is a no-op when the contract is not paused;
//!    otherwise it credits the elapsed pause interval to `total_paused_ns`
//!    and restores the operational status the dispute interrupted.
//!  * `terminate_contract_for_dispute_lost` short-circuits when the contract
//!    is already in a terminal state.

use super::Database;
use anyhow::{anyhow, Result};
use dcc_common::ContractStatus;

/// Idempotency-friendly input for `upsert_contract_dispute`.
///
/// Borrowed everywhere so callers (webhook handlers in Phase 2) avoid
/// allocations on the hot path.
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

/// Pre-pause snapshot returned by `resume_contract`. Phase 2 dc-agent code
/// uses the `resumed_to` value to decide whether to restart the VM
/// (Active/Provisioned) versus leave it down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeOutcome {
    /// `true` when this call actually flipped the contract out of `paused`.
    /// `false` means the contract was not paused (idempotent no-op).
    pub resumed: bool,
    /// Status the contract is now in (Active or Provisioned for a real resume,
    /// the contract's existing status for a no-op).
    pub status: ContractStatus,
    /// Nanoseconds added to `total_paused_ns` by this call (0 for a no-op).
    pub credited_pause_ns: i64,
}

/// Outcome of [`Database::replay_orphan_dispute_pause`] (#447). Each field is a
/// count of re-linked orphan disputes that fell into one of three buckets.
///
/// Money-path note: the `closed_lost_detected` count signals disputes whose
/// terminate+refund replay was INTENTIONALLY deferred (money-path decision).
/// The caller MUST page ops to review them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OrphanDisputeReplayOutcome {
    /// Open disputes whose pause was replayed (newly applied or re-confirmed
    /// idempotently on an already-paused contract).
    pub paused: u64,
    /// Disputes that closed as `lost` while orphaned. The terminate+refund
    /// replay is deferred (money-path); surfaced for operator review.
    pub closed_lost_detected: u64,
}

impl Database {
    /// Insert or refresh a dispute row keyed on `stripe_dispute_id`.
    ///
    /// Stripe replays webhooks indefinitely on non-2xx responses; this method
    /// MUST be safe to call repeatedly with the same payload. ON CONFLICT
    /// preserves the original `created_at_ns` and only refreshes mutable
    /// fields plus `updated_at_ns`. Optional timestamp fields use COALESCE so
    /// a later replay of the same event cannot blank out a value set by an
    /// earlier, more-informative event (e.g. `closed` should not erase
    /// `funds_withdrawn_at_ns` from a prior `funds_withdrawn` delivery).
    pub async fn upsert_contract_dispute(&self, input: ContractDisputeUpsert<'_>) -> Result<()> {
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
                 reason = COALESCE(EXCLUDED.reason, contract_disputes.reason),
                 stripe_payment_intent_id = COALESCE(EXCLUDED.stripe_payment_intent_id,
                                                    contract_disputes.stripe_payment_intent_id),
                 contract_id = COALESCE(EXCLUDED.contract_id, contract_disputes.contract_id),
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

    /// Pause an operational contract (Active or Provisioned).
    ///
    /// Records `paused_at_ns = now`, sets `pause_reason`, transitions status
    /// to `paused`, and writes a row each into `contract_status_history` and
    /// `contract_events`. Idempotent: re-calling with the same `reason` on an
    /// already-paused contract is a no-op (returns Ok without writes). A
    /// mismatched reason returns an error rather than silently overwriting --
    /// concurrent disputes on the same contract are an operator-level event.
    pub async fn pause_contract(&self, contract_id: &[u8], reason: &str) -> Result<()> {
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;
        let current: ContractStatus = contract.status.parse().map_err(|e| {
            anyhow!("Contract status invalid '{}': {}", contract.status, e)
        })?;

        if current == ContractStatus::Paused {
            // Idempotent replay: same reason -> no-op; conflicting reason -> loud failure.
            let existing = self.get_pause_reason(contract_id).await?;
            return match existing {
                Some(ref r) if r == reason => Ok(()),
                Some(other) => Err(anyhow!(
                    "Contract {} is already paused with reason '{}'; refusing to overwrite with '{}'",
                    hex::encode(contract_id), other, reason
                )),
                None => Err(anyhow!(
                    "Contract {} status is paused but pause_reason is NULL -- inconsistent row",
                    hex::encode(contract_id)
                )),
            };
        }

        if !current.can_transition_to(ContractStatus::Paused) {
            return Err(anyhow!(
                "Contract {} status {} cannot transition to paused",
                hex::encode(contract_id),
                contract.status
            ));
        }

        let now_ns = crate::now_ns()?;
        let paused_status = ContractStatus::Paused.to_string();
        let actor: &[u8] = b"system-stripe-dispute";

        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3, paused_at_ns = $2, pause_reason = $4 WHERE contract_id = $5",
            paused_status,
            now_ns,
            actor,
            reason,
            contract_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id,
            contract.status,
            paused_status,
            actor,
            now_ns,
            Some(reason)
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at) VALUES ($1, 'paused', $2, $3, 'system', $4, $5)",
            contract_id,
            contract.status,
            paused_status,
            Some(reason),
            now_ns
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Resume a paused contract. Credits the elapsed pause interval to
    /// `total_paused_ns` (used by the prorated-refund function) and restores
    /// the contract to `Active`. Idempotent: returns `resumed: false` when
    /// the contract is not currently paused.
    ///
    /// Note: Phase 1 always resumes to `Active`. Restoring the exact
    /// pre-pause state (Provisioned vs Active) requires capturing it at
    /// pause-time -- deferred to Phase 2 since the only operational status
    /// that pauses today is Active (Provisioned + dispute is rare; we accept
    /// `Provisioned -> Paused -> Active` as a benign promotion).
    pub async fn resume_contract(&self, contract_id: &[u8]) -> Result<ResumeOutcome> {
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;
        let current: ContractStatus = contract.status.parse().map_err(|e| {
            anyhow!("Contract status invalid '{}': {}", contract.status, e)
        })?;

        if current != ContractStatus::Paused {
            // Not paused -- idempotent no-op.
            return Ok(ResumeOutcome {
                resumed: false,
                status: current,
                credited_pause_ns: 0,
            });
        }

        let paused_at_ns = self.get_paused_at_ns(contract_id).await?.ok_or_else(|| {
            anyhow!(
                "Contract {} status is paused but paused_at_ns is NULL -- inconsistent row",
                hex::encode(contract_id)
            )
        })?;
        let now_ns = crate::now_ns()?;
        let credit = (now_ns - paused_at_ns).max(0);
        let resumed_status = ContractStatus::Active;
        let resumed_str = resumed_status.to_string();
        let actor: &[u8] = b"system-stripe-dispute";

        let history_memo = format!("resume after {} ns paused", credit);
        let event_details = format!("credited_pause_ns={}", credit);
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3, paused_at_ns = NULL, pause_reason = NULL, total_paused_ns = total_paused_ns + $4 WHERE contract_id = $5",
            resumed_str,
            now_ns,
            actor,
            credit,
            contract_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id,
            contract.status,
            resumed_str,
            actor,
            now_ns,
            Some(history_memo.as_str())
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at) VALUES ($1, 'resumed', $2, $3, 'system', $4, $5)",
            contract_id,
            contract.status,
            resumed_str,
            Some(event_details.as_str()),
            now_ns
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(ResumeOutcome {
            resumed: true,
            status: resumed_status,
            credited_pause_ns: credit,
        })
    }

    /// Terminate a contract because the dispute closed against us. Marks the
    /// cloud resource for deletion (so dc-agent destroys the VM) and
    /// transitions the contract to `cancelled`. Idempotent: a contract that
    /// is already terminal records an audit event and returns Ok.
    ///
    /// The Stripe refund call is intentionally NOT issued here -- losing a
    /// dispute means Stripe already moved the funds. The webhook handler in
    /// Phase 2 records the prorated remainder against `refund_amount_e9s`
    /// using `calculate_prorated_refund` (with the paused interval credited).
    pub async fn terminate_contract_for_dispute_lost(
        &self,
        contract_id: &[u8],
        stripe_dispute_id: &str,
    ) -> Result<()> {
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;
        let current: ContractStatus = contract.status.parse().map_err(|e| {
            anyhow!("Contract status invalid '{}': {}", contract.status, e)
        })?;
        let memo = format!("stripe_dispute_lost:{}", stripe_dispute_id);
        let now_ns = crate::now_ns()?;
        let actor: &[u8] = b"system-stripe-dispute";

        if current.is_terminal() {
            // Already terminal -- record the dispute outcome but do not transition.
            self.insert_contract_event(
                contract_id,
                "dispute_lost",
                Some(&contract.status),
                None,
                "system",
                Some(&memo),
            )
            .await?;
            return Ok(());
        }

        if !current.can_transition_to(ContractStatus::Cancelled) {
            return Err(anyhow!(
                "Contract {} status {} cannot transition to cancelled (dispute_lost)",
                hex::encode(contract_id),
                contract.status
            ));
        }

        let cancelled_str = ContractStatus::Cancelled.to_string();
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3, payment_status = 'disputed' WHERE contract_id = $4",
            cancelled_str,
            now_ns,
            actor,
            contract_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id,
            contract.status,
            cancelled_str,
            actor,
            now_ns,
            Some(memo.as_str())
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at) VALUES ($1, 'dispute_lost', $2, $3, 'system', $4, $5)",
            contract_id,
            contract.status,
            cancelled_str,
            Some(memo.as_str()),
            now_ns
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        // Best-effort: tell the deletion loop to tear down the VM. A failure here
        // does NOT roll back the status transition (the contract is already cancelled
        // logically; an operator can re-trigger cleanup).
        if let Err(e) = self.mark_contract_resource_for_deletion(contract_id).await {
            tracing::warn!(
                contract_id = %hex::encode(contract_id),
                stripe_dispute_id,
                error = %format!("{:#}", e),
                "Failed to mark cloud_resource for deletion after dispute_lost"
            );
        }
        Ok(())
    }

    /// Look up a contract's current `pause_reason`. Used by `pause_contract`
    /// to detect conflicting concurrent pauses, and by the reconcile endpoint
    /// to forward the reason to the dc-agent's pause action.
    pub async fn get_pause_reason(&self, contract_id: &[u8]) -> Result<Option<String>> {
        let row = sqlx::query!(
            "SELECT pause_reason FROM contract_sign_requests WHERE contract_id = $1",
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.pause_reason))
    }

    /// Look up a contract's `paused_at_ns`. Used by `resume_contract` to
    /// compute the credited pause interval.
    pub(super) async fn get_paused_at_ns(&self, contract_id: &[u8]) -> Result<Option<i64>> {
        let row = sqlx::query!(
            "SELECT paused_at_ns FROM contract_sign_requests WHERE contract_id = $1",
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.paused_at_ns))
    }

    /// Read a contract's `total_paused_ns`. Phase 2 webhook handler passes
    /// this to `calculate_prorated_refund` so the customer is credited for
    /// time the VM was unavailable.
    pub async fn get_total_paused_ns(&self, contract_id: &[u8]) -> Result<i64> {
        let row = sqlx::query!(
            "SELECT total_paused_ns FROM contract_sign_requests WHERE contract_id = $1",
            contract_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.total_paused_ns)
    }

    /// Look up a contract by its real Stripe PaymentIntent ID (`pi_*`).
    ///
    /// Phase 2 dispute handler resolves `event.data.object.payment_intent` to
    /// our internal contract via this lookup. Legacy rows (predating the
    /// `stripe_checkout_session_id` / `stripe_payment_intent_id` split) may
    /// have the PI ID stored in the checkout-session column; the second
    /// query covers that fallback in one round trip.
    pub async fn get_contract_id_by_stripe_payment_intent(
        &self,
        payment_intent_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query!(
            r#"SELECT contract_id FROM contract_sign_requests
               WHERE stripe_payment_intent_id = $1
                  OR stripe_checkout_session_id = $1
               ORDER BY created_at_ns DESC
               LIMIT 1"#,
            payment_intent_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.contract_id))
    }

    /// Reconcile orphan disputes after `checkout.session.completed` learns the
    /// real PaymentIntent for a contract.
    ///
    /// Stripe occasionally delivers `charge.dispute.created` BEFORE
    /// `checkout.session.completed` for the same payment. At dispute time the
    /// contract has no `stripe_payment_intent_id` yet, so the dispute is
    /// persisted as an orphan (`contract_id = NULL`; see
    /// `lookup_contract_for_charge` in `openapi::webhooks`). This method
    /// backfills the FK once the PI is known so the dispute is visible on the
    /// contract and the charge-based lookup fallback works for any later
    /// dispute event.
    ///
    /// Scope (intentionally narrow): this ONLY sets `contract_id`. It does NOT
    /// replay pause / terminate / refund actions even if the dispute has since
    /// closed -- that is the caller's job via [`Self::replay_orphan_dispute_pause`]
    /// (#447), which is kept separate so this method stays a money-safe FK
    /// backfill. The orphan ops alert already fired at dispute-created time, so
    /// an operator can act on still-open disputes; closing the FK link here
    /// just makes the row queryable by contract and unblocks future events.
    ///
    /// Idempotent + money-safe by construction: `WHERE contract_id IS NULL`
    /// means replays and concurrent invocations affect each orphan at most
    /// once, and no payment / status / refund column is touched.
    ///
    /// Returns the number of orphan rows linked (0 when there was nothing to
    /// reconcile -- the common case).
    pub async fn relink_orphan_disputes_for_payment_intent(
        &self,
        contract_id: &[u8],
        payment_intent_id: &str,
    ) -> Result<u64> {
        let now_ns = crate::now_ns()?;
        let result = sqlx::query(
            r#"UPDATE contract_disputes
               SET contract_id = $1, updated_at_ns = $2
               WHERE stripe_payment_intent_id = $3 AND contract_id IS NULL"#,
        )
        .bind(contract_id)
        .bind(now_ns)
        .bind(payment_intent_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Replay the missed dispute-lifecycle effect for orphans re-linked after
    /// late checkout completion (#447).
    ///
    /// When `charge.dispute.created` arrives BEFORE `checkout.session.completed`,
    /// the contract lookup fails and the dispute is persisted as an orphan
    /// (`contract_id = NULL`). The normal `handle_dispute_created` handler
    /// pauses the matched contract; an orphan misses that pause entirely. Once
    /// `relink_orphan_disputes_for_payment_intent` backfills the FK, this method
    /// replays the missed non-money effect: it pauses the contract for each
    /// re-linked OPEN dispute, exactly as the normal handler would have.
    ///
    /// Money-safety (non-negotiable):
    ///  * `pause_contract` is a status change only -- it touches `status`,
    ///    `paused_at_ns`, `pause_reason`, and audit rows, NEVER a money column.
    ///  * `pause_contract` is idempotent: the same `stripe_dispute:{id}` reason
    ///    is a no-op on an already-paused contract. Re-running this replay (or a
    ///    Stripe webhook replay) is safe.
    ///  * Closed-`lost` orphans are DETECTED and surfaced via
    ///    [`OrphanDisputeReplayOutcome::closed_lost_detected`] but their
    ///    terminate+refund replay is a MONEY-PATH decision and is intentionally
    ///    NOT auto-applied here (risk of double-refund / wrong lifecycle). The
    ///    caller MUST page ops to review those rows.
    ///  * Closed-`won` / `warning_closed` orphans need no action (nothing was
    ///    missed -- the dispute resolved without a loss).
    ///
    /// When the contract is not yet pausable at replay time (e.g. it is still
    /// `requested`/`pending` at checkout-completion, which is the realistic
    /// orphan state), the pause is attempted and fails gracefully (logged); the
    /// linked dispute row keeps the dispute visible until a later event or an
    /// operator acts. This mirrors exactly what the normal handler does when a
    /// dispute arrives on a pre-active contract.
    pub async fn replay_orphan_dispute_pause(
        &self,
        contract_id: &[u8],
        payment_intent_id: &str,
    ) -> Result<OrphanDisputeReplayOutcome> {
        // Fetch the disputes linked to this contract via this PaymentIntent --
        // the orphans just backfilled by `relink_orphan_disputes_for_payment_intent`
        // (plus any same-PI disputes linked normally; idempotent either way).
        // Runtime query (not `query!`) mirrors the sibling relink method: the
        // dispute table schema is stable and this avoids churning `.sqlx`.
        let disputes: Vec<(String, String)> = sqlx::query_as(
            r#"SELECT stripe_dispute_id, status
               FROM contract_disputes
               WHERE contract_id = $1 AND stripe_payment_intent_id = $2"#,
        )
        .bind(contract_id)
        .bind(payment_intent_id)
        .fetch_all(&self.pool)
        .await?;

        let mut outcome = OrphanDisputeReplayOutcome::default();
        for (stripe_dispute_id, status) in disputes {
            if status == "lost" {
                // Closed-lost while orphaned: terminate+refund is a money-path
                // decision -- surface for operator review, do NOT auto-refund.
                outcome.closed_lost_detected += 1;
                continue;
            }
            if !is_open_dispute_status(&status) {
                // 'won' / 'warning_closed' / other terminal: nothing to pause.
                continue;
            }
            let reason = format!("stripe_dispute:{}", stripe_dispute_id);
            match self.pause_contract(contract_id, &reason).await {
                Ok(()) => outcome.paused += 1,
                Err(e) => {
                    // Pause could not be applied (contract not pausable yet, or
                    // a conflicting concurrent pause). Money-safe no-op: the
                    // contract either cannot be paused at this lifecycle stage
                    // (nothing to pause) or is already paused. Log + continue.
                    tracing::warn!(
                        contract_id = %hex::encode(contract_id),
                        stripe_dispute_id = %stripe_dispute_id,
                        payment_intent = %payment_intent_id,
                        error = %format!("{:#}", e),
                        "Could not replay pause for re-linked orphan dispute; \
                         contract may not be pausable yet (money-safe no-op)"
                    );
                }
            }
        }
        Ok(outcome)
    }

    /// Look up a contract by Stripe charge ID. Used as a final fallback in the
    /// dispute-handler lookup chain when the dispute payload lacks a PI but
    /// carries a charge ID we have already seen via a previous dispute event
    /// (inserted into `contract_disputes.stripe_charge_id`).
    pub async fn get_contract_id_by_stripe_charge(
        &self,
        charge_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query!(
            r#"SELECT contract_id FROM contract_disputes
               WHERE stripe_charge_id = $1 AND contract_id IS NOT NULL
               ORDER BY created_at_ns DESC
               LIMIT 1"#,
            charge_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.contract_id))
    }

    /// Compute and (optionally) issue the prorated refund owed to the customer
    /// after a `charge.dispute.closed` event with status=lost.
    ///
    /// Stripe's auto-withdrawal already pulls the disputed amount; the refund
    /// here covers the unused billable window (paused time excluded). The
    /// idempotency key is fixed at `dispute:{stripe_dispute_id}` so webhook
    /// replays collapse onto a single Stripe Refund record on Stripe's side
    /// AND onto a single `refund_audit` row on ours.
    ///
    /// Returns `(refund_amount_e9s, stripe_refund_id)`. `(None, None)` means
    /// nothing was owed (no successful payment, refund<=0, etc). On Stripe
    /// API errors the audit row is marked `failed` and the error propagates;
    /// Stripe will replay the webhook and the same idempotency key collapses
    /// the retry onto the same refund attempt.
    pub async fn process_dispute_lost_refund(
        &self,
        contract_id: &[u8],
        stripe_dispute_id: &str,
        stripe_client: Option<&crate::stripe_client::StripeClient>,
    ) -> Result<(Option<i64>, Option<String>)> {
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;
        if contract.payment_status != "succeeded" && contract.payment_status != "disputed" {
            // No money to refund. (`disputed` is what terminate_contract_for_dispute_lost
            // sets; we accept both so this method can run before OR after the terminate.)
            return Ok((None, None));
        }
        if contract.payment_method != "stripe" {
            return Ok((None, None));
        }
        let stripe_id = contract
            .stripe_payment_intent_id
            .as_deref()
            .or(contract.stripe_checkout_session_id.as_deref());
        let Some(payment_intent_id) = stripe_id else {
            return Ok((None, None));
        };

        let now_ns = crate::now_ns()?;
        // Gross prorated refund for unused time. Under Stripe-only no funds are
        // ever pre-released to the provider, so calculate_net_refund_e9s returns
        // the full prorated remainder — sharing it with the cancel/reject paths
        // keeps refund policy consistent across all three flows.
        let refund_e9s = self.calculate_net_refund_e9s(&contract, now_ns).await?;
        if refund_e9s <= 0 {
            return Ok((None, None));
        }

        let requester_pubkey_bytes = hex::decode(&contract.requester_pubkey).unwrap_or_default();

        use crate::database::refund_requests::{GatedRefundInput, RefundGateOutcome};
        let stripe_refund_id = match self
            .process_gated_refund(GatedRefundInput {
                contract_id,
                requester_pubkey: &requester_pubkey_bytes,
                refund_e9s,
                reason: "dispute_lost",
                payment_intent_id,
                currency: &contract.currency,
                stripe_dispute_id: Some(stripe_dispute_id),
                stripe_client,
            })
            .await?
        {
            RefundGateOutcome::AutoIssued {
                stripe_refund_id, ..
            } => stripe_refund_id,
            RefundGateOutcome::PendingApproval { .. } => None,
            RefundGateOutcome::NoRefund => None,
        };

        if let Some(ref id) = &stripe_refund_id {
            tracing::info!(
                contract_id = %hex::encode(contract_id),
                stripe_dispute_id,
                stripe_refund_id = %id,
                "Stripe dispute-lost refund issued"
            );
        }

        // Persist refund accounting on the contract row regardless of whether
        // Stripe accepted the refund; an operator with the row contents can
        // reconcile manually.
        sqlx::query!(
            "UPDATE contract_sign_requests SET refund_amount_e9s = $1, stripe_refund_id = $2, refund_created_at_ns = $3 WHERE contract_id = $4",
            refund_e9s,
            stripe_refund_id,
            now_ns,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok((Some(refund_e9s), stripe_refund_id))
    }
}


/// Idempotency key used for the prorated refund issued after a lost dispute.
/// Exposed so callers can log it and tests can assert the exact value. Thin
/// wrapper over the generic `refund::refund_idempotency_key` so the dispute
/// path shares one source of truth with cancel/reject/manual paths.
pub fn dispute_refund_idempotency_key(stripe_dispute_id: &str) -> String {
    // contract_id is unused for dispute keys (stripe_dispute_id is globally
    // unique); pass an empty slice to satisfy the shared signature.
    crate::refund::refund_idempotency_key("dispute", &[], stripe_dispute_id)
}

/// Whether a Stripe dispute `status` means the dispute is still open and the
/// matched contract SHOULD be paused. Returns `false` for terminal outcomes
/// (`won`, `lost`, `warning_closed`) where a pause replay is wrong or moot:
///  * `won` -- resolved in our favor; nothing to pause.
///  * `lost` -- should terminate+refund (deferred money path); pause is wrong.
///  * `warning_closed` -- inquiry withdrawn; nothing to pause.
///
/// Any unrecognized status is treated as open: pausing for an active dispute is
/// the conservative, correct default. Stripe's documented statuses include
/// `needs_response`, `response_required`, `under_review`, `charging`, `won`,
/// `lost`, and `warning_closed`; unknowns are assumed active.
fn is_open_dispute_status(status: &str) -> bool {
    !matches!(status, "won" | "lost" | "warning_closed")
}
