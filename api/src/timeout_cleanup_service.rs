//! Periodic timeout-cleanup tasks for stuck contracts (issues #409 + #410).
//!
//! Mirrors the structure of `cleanup_service.rs` (the existing 24-hour
//! retention/cleanup loop) at a tighter cadence: 5 minutes by default. Two
//! tasks run every cycle:
//!
//!  1. `cleanup_stale_requested` (#410): contracts whose Stripe checkout
//!     never completed (status = `requested`) and that have been sitting
//!     longer than `requested_timeout_secs` flip to `expired`. No refund is
//!     issued because no payment ever succeeded.
//!  2. `cleanup_failed_provisioning` (#409): contracts in `accepted` or
//!     `provisioning` longer than `provisioning_timeout_secs` flip to
//!     `provisioningfailed` and the full paid amount is auto-refunded via
//!     `issue_audited_refund` (idempotency key
//!     `provisioning_failed:{contract_id}:{provisioning_failed_at_ns}`).
//!
//! A failure on one row MUST NOT block the loop from processing the rest --
//! every per-row error is logged and execution continues. The cycle as a
//! whole returns Ok unless the database-level scan itself errors.

use crate::database::Database;
use crate::stripe_client::StripeClient;
use std::sync::Arc;
use std::time::Duration;

/// Background service that runs both timeout-driven cleanup tasks. Owns the
/// database handle, an optional Stripe client (passed through to
/// `mark_provisioning_failed` so the refund call hits the real Stripe API in
/// production and is bypassed cleanly in pure-DB tests), the cycle interval
/// and the two configurable timeout windows.
pub struct TimeoutCleanupService {
    database: Arc<Database>,
    stripe_client: Option<Arc<StripeClient>>,
    interval: Duration,
    requested_timeout_secs: u64,
    provisioning_timeout_secs: u64,
}

impl TimeoutCleanupService {
    pub fn new(
        database: Arc<Database>,
        stripe_client: Option<Arc<StripeClient>>,
        interval_secs: u64,
        requested_timeout_secs: u64,
        provisioning_timeout_secs: u64,
    ) -> Self {
        Self {
            database,
            stripe_client,
            interval: Duration::from_secs(interval_secs),
            requested_timeout_secs,
            provisioning_timeout_secs,
        }
    }

    /// Run the cleanup loop until the shared shutdown signal flips. Mirrors
    /// the cancel-aware select pattern used by `CleanupService::run` so both
    /// services respond to SIGTERM identically.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.interval);

        // Run an initial pass on startup so a freshly-deployed server does
        // not wait one full cycle to clear backlog from a prior outage.
        if let Err(e) = self.cleanup_once().await {
            tracing::error!("Initial timeout cleanup failed: {:#}", e);
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    tracing::info!("Timeout cleanup service shutting down gracefully");
                    return;
                }
            }
            if let Err(e) = self.cleanup_once().await {
                tracing::error!("Timeout cleanup failed: {:#}", e);
            }
        }
    }

    /// One pass of both cleanup tasks. Errors on individual rows are logged
    /// and swallowed; only a database-level scan failure surfaces here.
    async fn cleanup_once(&self) -> anyhow::Result<()> {
        self.cleanup_stale_requested().await?;
        self.cleanup_failed_provisioning().await?;
        Ok(())
    }

    /// Issue #410: expire contracts whose Stripe checkout never completed.
    async fn cleanup_stale_requested(&self) -> anyhow::Result<()> {
        let cutoff_ns = self.cutoff_ns(self.requested_timeout_secs)?;
        let stale = self.database.find_stale_requested(cutoff_ns).await?;
        if stale.is_empty() {
            tracing::debug!("No stale `requested` contracts to expire");
            return Ok(());
        }

        let total = stale.len();
        let mut expired = 0u64;
        for row in stale {
            match self.database.expire_requested(&row.contract_id).await {
                Ok(true) => {
                    expired += 1;
                    tracing::info!(
                        contract_id = %hex::encode(&row.contract_id),
                        "Expired stale `requested` contract (pre-payment timeout)"
                    );
                }
                Ok(false) => {
                    // Another worker handled the row between scan and
                    // transition; benign no-op.
                    tracing::debug!(
                        contract_id = %hex::encode(&row.contract_id),
                        "Stale `requested` contract already transitioned"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        contract_id = %hex::encode(&row.contract_id),
                        error = %format!("{:#}", e),
                        "Failed to expire stale `requested` contract"
                    );
                }
            }
        }
        tracing::info!(
            "Stale `requested` cleanup pass: scanned={} expired={}",
            total, expired
        );
        Ok(())
    }

    /// Issue #409: mark stuck provisioning contracts as failed and refund.
    async fn cleanup_failed_provisioning(&self) -> anyhow::Result<()> {
        let cutoff_ns = self.cutoff_ns(self.provisioning_timeout_secs)?;
        let stale = self.database.find_failed_provisioning(cutoff_ns).await?;
        if stale.is_empty() {
            tracing::debug!("No stuck provisioning contracts to fail");
            return Ok(());
        }

        let total = stale.len();
        let mut failed = 0u64;
        let stripe_ref = self.stripe_client.as_deref();
        for row in stale {
            let reason = format!(
                "agent did not reach provisioned within {} seconds",
                self.provisioning_timeout_secs
            );
            match self
                .database
                .mark_provisioning_failed(&row.contract_id, &reason, stripe_ref)
                .await
            {
                Ok(Some(_)) => {
                    failed += 1;
                    tracing::warn!(
                        contract_id = %hex::encode(&row.contract_id),
                        prior_status = %row.status,
                        "Marked stuck contract as provisioning_failed and triggered refund"
                    );
                }
                Ok(None) => {
                    tracing::debug!(
                        contract_id = %hex::encode(&row.contract_id),
                        "Stuck provisioning contract already transitioned"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        contract_id = %hex::encode(&row.contract_id),
                        error = %format!("{:#}", e),
                        "Failed to mark stuck provisioning contract"
                    );
                }
            }
        }
        tracing::info!(
            "Stuck provisioning cleanup pass: scanned={} failed={}",
            total, failed
        );
        Ok(())
    }

    fn cutoff_ns(&self, timeout_secs: u64) -> anyhow::Result<i64> {
        let now_ns = crate::now_ns()?;
        let timeout_ns = i64::try_from(timeout_secs)?.checked_mul(1_000_000_000)
            .ok_or_else(|| anyhow::anyhow!("timeout_secs overflow: {}", timeout_secs))?;
        Ok(now_ns.saturating_sub(timeout_ns))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    /// End-to-end loop test: a single `cleanup_once` pass must transition a
    /// stale `requested` contract AND a stuck `provisioning` contract in
    /// the same cycle, and must NOT touch fresh rows. This is the only test
    /// that exercises both cleanup tasks together; the per-task logic is
    /// covered by the unit tests in `database::contracts::timeouts::tests`.
    #[tokio::test]
    async fn test_cleanup_once_transitions_both_stale_classes_in_one_pass() {
        let db = Arc::new(setup_test_db().await);

        let provider: &[u8] = &[0xCC; 32];
        let requester: &[u8] = &[0xDD; 32];

        // Three contracts: stale requested, stale provisioning, fresh
        // provisioning. We use very short timeouts (1 second) so the rows
        // we insert with `status_updated_at_ns = 0` are well past the
        // cutoff while a row stamped at `now()` is well within it.
        let stale_req = vec![0xA0; 32];
        let stale_prov = vec![0xA1; 32];
        let fresh_prov = vec![0xA2; 32];

        for (cid, status, ts) in [
            (&stale_req, "requested", 0_i64),
            (&stale_prov, "provisioning", 0_i64),
            (&fresh_prov, "provisioning", crate::now_ns().unwrap()),
        ] {
            sqlx::query(
                r#"INSERT INTO contract_sign_requests (
                    contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                    provider_pubkey, offering_id, payment_amount_e9s, request_memo,
                    created_at_ns, status, status_updated_at_ns, payment_method,
                    payment_status, currency
                ) VALUES ($1, $2, '', '', $3, 'off-loop', 0, '',
                          $4, $5, $4, 'icpay', 'succeeded', 'usd')"#,
            )
            .bind(cid.as_slice())
            .bind(requester)
            .bind(provider)
            .bind(ts)
            .bind(status)
            .execute(&db.pool)
            .await
            .unwrap();
        }

        let service = TimeoutCleanupService::new(db.clone(), None, 60, 1, 1);
        service.cleanup_once().await.expect("cleanup pass must succeed");

        let status_of = |cid: &[u8]| {
            let pool = db.pool.clone();
            let cid = cid.to_vec();
            async move {
                sqlx::query_scalar::<_, String>(
                    "SELECT status FROM contract_sign_requests WHERE contract_id = $1",
                )
                .bind(cid)
                .fetch_one(&pool)
                .await
                .unwrap()
            }
        };

        assert_eq!(status_of(&stale_req).await, "expired");
        assert_eq!(status_of(&stale_prov).await, "provisioningfailed");
        assert_eq!(
            status_of(&fresh_prov).await,
            "provisioning",
            "fresh row must NOT be touched -- timeout boundary respected end-to-end"
        );
    }

    #[tokio::test]
    async fn test_cleanup_once_with_no_stale_rows_is_silent_noop() {
        // Defensive: a healthy production system spends most of its time in
        // this branch. A single empty cycle must not error and must not
        // write any contract events.
        let db = Arc::new(setup_test_db().await);
        let service = TimeoutCleanupService::new(db.clone(), None, 60, 60, 60);
        service.cleanup_once().await.expect("empty cycle must succeed");
        let event_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM contract_events")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(event_count, 0, "no contract events on empty cycle");
    }
}
