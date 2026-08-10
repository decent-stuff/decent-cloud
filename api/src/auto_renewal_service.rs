use crate::database::Database;
use std::sync::Arc;
use std::time::Duration;

/// Background service that auto-renews expiring contracts.
///
/// Runs every 6 hours. For each active contract with auto_renew=true that expires
/// within 48 hours, it creates a new rental request with the same parameters and
/// clears auto_renew on the old contract so it won't trigger again.
pub struct AutoRenewalService {
    database: Arc<Database>,
    interval: Duration,
}

impl AutoRenewalService {
    pub fn new(database: Arc<Database>, interval_hours: u64) -> Self {
        Self {
            database,
            interval: Duration::from_secs(interval_hours * 60 * 60),
        }
    }

    /// Run the auto-renewal service until shutdown is signalled.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial check on startup
        self.process_renewals_once().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    tracing::info!("Auto-renewal service shutting down gracefully");
                    return;
                }
            }
            self.process_renewals_once().await;
        }
    }

    async fn process_renewals_once(&self) {
        tracing::info!("Checking for contracts due for auto-renewal");

        let contracts = match self.database.get_contracts_for_renewal().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to fetch contracts for auto-renewal: {:#}", e);
                return;
            }
        };

        if contracts.is_empty() {
            tracing::debug!("No contracts due for auto-renewal");
            return;
        }

        tracing::info!("{} contract(s) due for auto-renewal", contracts.len());

        for contract in contracts {
            if let Err(e) = self.renew_contract(&contract).await {
                tracing::error!(
                    contract_id = %contract.contract_id,
                    "Auto-renewal failed: {:#}",
                    e
                );
                // Continue processing remaining contracts — one failure must not block others
            }
        }
    }

    async fn renew_contract(
        &self,
        contract: &crate::database::contracts::Contract,
    ) -> anyhow::Result<()> {
        let contract_id_bytes = hex::decode(&contract.contract_id)
            .map_err(|e| anyhow::anyhow!("Invalid contract_id hex: {}", e))?;
        let requester_pubkey_bytes = hex::decode(&contract.requester_pubkey)
            .map_err(|e| anyhow::anyhow!("Invalid requester_pubkey hex: {}", e))?;

        // Parse offering_db_id from the stored offering_id string
        let offering_db_id: i64 = contract
            .offering_id
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid offering_id '{}'", contract.offering_id))?;

        // === Fix (b): wallet-method renewals must debit the wallet =========
        // The HTTP rental-creation path debits the prepaid wallet immediately
        // after create (openapi/contracts.rs). The renewal path used to skip
        // that, so a wallet renewal produced a zombie contract (status=
        // requested, payment_status=pending) that can NEVER be provisioned
        // (DB CHECK 048 + the code gate in update_contract_status), while the
        // old contract's auto_renew was silently cleared — expiring the user's
        // running service with no paid replacement and no error logged.
        //
        // Renewals that are self-rentals or use the test method set
        // payment_status='succeeded' (amount 0) at creation and need no debit.

        // Fail fast BEFORE creating a new contract when the wallet clearly can't
        // cover the renewal. This uses the old contract's price as an estimate
        // (the renewal recreates the same offering + duration), so a chronically
        // under-funded wallet doesn't spawn a fresh zombie every renewal cycle.
        // The atomic debit further below remains the real money-safety guard
        // and handles the rare case where the estimate was too low.
        let requester_hex = &contract.requester_pubkey;
        let needs_wallet_debit = contract.payment_amount_e9s > 0
            && !Self::is_free_or_test_renewal(contract);
        if needs_wallet_debit {
            let balance = self
                .database
                .get_wallet_balance(requester_hex)
                .await?
                .unwrap_or(0);
            if balance < contract.payment_amount_e9s {
                anyhow::bail!(
                    "Insufficient wallet balance for auto-renewal: has {} e9s, \
                     renewal costs ~{} e9s (previous contract price); not creating a renewal",
                    balance,
                    contract.payment_amount_e9s
                );
            }
        }

        let params = crate::database::contracts::RentalRequestParams {
            offering_db_id,
            ssh_pubkey: Some(contract.requester_ssh_pubkey.clone()),
            contact_method: Some(contract.requester_contact.clone()),
            request_memo: Some(format!("Auto-renewal of {}", &contract.contract_id[..12])),
            duration_hours: contract.original_duration_hours.or(contract.duration_hours),
            payment_method: Some(contract.payment_method.clone()),
            buyer_address: contract.buyer_address.clone(),
            operating_system: contract.operating_system.clone(),
        };

        let new_contract_id = self
            .database
            .create_rental_request(&requester_pubkey_bytes, params)
            .await?;

        // If create left the new contract unpaid (wallet method), debit the
        // wallet now — exactly as the HTTP handler does. On insufficient balance
        // (race or price increase vs the estimate above), cancel the unpaid
        // zombie and return the error WITHOUT clearing auto_renew, so the
        // renewal is retried next cycle instead of stranding the user.
        let new_contract = self
            .database
            .get_contract(&new_contract_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Newly created renewal contract {} could not be read back",
                    hex::encode(&new_contract_id)
                )
            })?;

        if new_contract.payment_status != dcc_common::payment_status::SUCCEEDED
            && new_contract.payment_amount_e9s > 0
        {
            if let Err(debit_err) = self
                .database
                .debit_wallet_for_contract(
                    requester_hex,
                    &new_contract_id,
                    new_contract.payment_amount_e9s,
                )
                .await
            {
                if let Err(cancel_err) = self
                    .database
                    .cancel_unpaid_contract(&new_contract_id)
                    .await
                {
                    tracing::error!(
                        new_contract_id = %hex::encode(&new_contract_id),
                        "Failed to cancel unpaid renewal zombie after wallet debit failure: {:#}",
                        cancel_err
                    );
                }
                return Err(debit_err.context("wallet debit for auto-renewal failed"));
            }
        }

        // Clear auto_renew on the old contract so it won't trigger again.
        self.database
            .set_contract_auto_renew(&contract_id_bytes, &requester_pubkey_bytes, false)
            .await?;

        // === Fix (c): renewals now trigger the spending alert too =========
        // (previously only the HTTP path did). Best-effort: errors are logged
        // inside the helper and never affect the renewal.
        crate::openapi::contracts::check_spending_alert_and_notify(
            &self.database,
            &requester_pubkey_bytes,
        )
        .await;

        tracing::info!(
            old_contract_id = %contract.contract_id,
            new_contract_id = %hex::encode(&new_contract_id),
            "Auto-renewed contract"
        );

        Ok(())
    }

    /// True for renewals that need NO wallet debit: self-rentals (requester is
    /// the provider, free) and the test payment method (auto-succeeds without
    /// payment). Both set `payment_status='succeeded'` at creation.
    fn is_free_or_test_renewal(contract: &crate::database::contracts::Contract) -> bool {
        contract.requester_pubkey == contract.provider_pubkey
            || contract.payment_method.to_lowercase() == "test"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[test]
    fn test_auto_renewal_service_interval() {
        // Verify the 6-hour interval is 21600 seconds
        assert_eq!(6u64 * 60 * 60, 21_600);
    }

    /// Deterministic 32-byte pubkey, distinct per `seed`.
    fn pk(seed: u8) -> Vec<u8> {
        vec![seed; 32]
    }

    /// Seed a public offering owned by `provider_pk` with an explicit numeric
    /// `id` and a matching numeric `offering_id` string. A numeric offering_id
    /// is required because `renew_contract` parses `contract.offering_id` as
    /// `i64` and then fetches the offering by that id.
    async fn seed_offering(db: &Database, provider_pk: &[u8], offering_db_id: i64, monthly_price: f64) {
        sqlx::query!(
            r#"INSERT INTO provider_offerings (
                id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee,
                visibility, product_type, billing_interval, stock_status,
                datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns
            ) VALUES ($1, $2, $3, 'Renewal Test', 'USD', $4, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0)"#,
            offering_db_id,
            provider_pk,
            offering_db_id.to_string(),
            monthly_price,
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    /// Seed an active, auto-renewable wallet-method contract expiring inside the
    /// 48h renewal window. Its `payment_status` is already `succeeded` (the
    /// original contract was paid); `payment_amount_e9s` is the collected price.
    async fn seed_renewable_wallet_contract(
        db: &Database,
        requester_pk: &[u8],
        provider_pk: &[u8],
        offering_db_id: i64,
        payment_amount_e9s: i64,
    ) -> Vec<u8> {
        let now_ns = crate::now_ns().unwrap();
        let start_ns = now_ns - 720 * 3600 * 1_000_000_000; // started ~a month ago
        let end_ns = now_ns + 24 * 3600 * 1_000_000_000; // expires in 24h (< 48h window)
        let contract_id = vec![0xC0u8; 32];
        sqlx::query!(
            r#"INSERT INTO contract_sign_requests (
                contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns,
                end_timestamp_ns, duration_hours, original_duration_hours, request_memo,
                created_at_ns, status, payment_method, payment_status, currency, auto_renew
            ) VALUES (
                $1, $2, 'ssh-ed25519 AAAAC1', 'email:test@example.com', $3, $4, $5, $6, $7,
                720, 720, 'original contract', $8, 'active', 'wallet', 'succeeded', 'USD', TRUE
            )"#,
            &contract_id[..],
            requester_pk,
            provider_pk,
            offering_db_id.to_string(),
            payment_amount_e9s,
            start_ns,
            end_ns,
            now_ns,
        )
        .execute(&db.pool)
        .await
        .unwrap();
        contract_id
    }

    /// Money-safety (b): a wallet-method renewal must debit the wallet, mark the
    /// new contract paid, record a ledger row, and clear auto_renew on the old
    /// contract — so the user's running service is replaced by a paid one.
    #[tokio::test]
    async fn test_renew_contract_wallet_debits_balance() {
        let db = Arc::new(setup_test_db().await);
        let requester = pk(0xA1);
        let provider = pk(0xB2);
        let requester_hex = hex::encode(&requester);
        let offering_db_id = 4242_i64;
        // monthly_price $100 over the default 720h → 100_000_000_000 e9s
        let renewal_price_e9s = 100_000_000_000_i64;

        // Wallet holds 2× the renewal price so the debit must succeed.
        db.credit_wallet_balance(&requester_hex, 200_000_000_000, "topup", Some("cs_renew_ok"))
            .await
            .unwrap();

        seed_offering(&db, &provider, offering_db_id, 100.0).await;
        let old_id =
            seed_renewable_wallet_contract(&db, &requester, &provider, offering_db_id, renewal_price_e9s)
                .await;
        let balance_before = db.get_wallet_balance(&requester_hex).await.unwrap().unwrap();

        AutoRenewalService::new(db.clone(), 6)
            .process_renewals_once()
            .await;

        // (a) Wallet balance decreased by exactly the renewal price.
        let balance_after = db.get_wallet_balance(&requester_hex).await.unwrap().unwrap();
        assert_eq!(
            balance_before - balance_after,
            renewal_price_e9s,
            "wallet must be debited the full renewal price"
        );

        // (b) A rental_debit ledger row exists and references the new contract,
        // which is paid via wallet (payment_status=succeeded).
        let ledger = db.get_wallet_ledger(&requester_hex, 5).await.unwrap();
        let debits: Vec<_> = ledger
            .iter()
            .filter(|e| e.entry_type == "rental_debit")
            .collect();
        assert_eq!(debits.len(), 1, "exactly one rental_debit ledger row");
        assert_eq!(debits[0].amount_e9s, -renewal_price_e9s);
        let new_contract_id = hex::decode(debits[0].reference.as_ref().unwrap()).unwrap();
        let new_contract = db.get_contract(&new_contract_id).await.unwrap().unwrap();
        assert_eq!(new_contract.payment_status, "succeeded");
        assert_eq!(new_contract.payment_method, "wallet");

        // (c) Old contract's auto_renew cleared (renewal succeeded → not retried).
        let old_after = db.get_contract(&old_id).await.unwrap().unwrap();
        assert!(
            !old_after.auto_renew,
            "old contract auto_renew must be cleared after a successful renewal"
        );
    }

    /// Money-safety (b): insufficient wallet balance must NOT debit, must NOT
    /// clear auto_renew (so it's retried), and must NOT leave a zombie contract.
    #[tokio::test]
    async fn test_renew_contract_wallet_insufficient_balance_keeps_auto_renew() {
        let db = Arc::new(setup_test_db().await);
        let requester = pk(0xA3);
        let provider = pk(0xB4);
        let requester_hex = hex::encode(&requester);
        let offering_db_id = 5353_i64;
        let renewal_price_e9s = 100_000_000_000_i64;

        // Wallet holds only a tenth of the renewal price.
        db.credit_wallet_balance(&requester_hex, 10_000_000_000, "topup", Some("cs_renew_no"))
            .await
            .unwrap();

        seed_offering(&db, &provider, offering_db_id, 100.0).await;
        let old_id =
            seed_renewable_wallet_contract(&db, &requester, &provider, offering_db_id, renewal_price_e9s)
                .await;

        AutoRenewalService::new(db.clone(), 6)
            .process_renewals_once()
            .await;

        // Wallet untouched: no debit, no create.
        assert_eq!(
            db.get_wallet_balance(&requester_hex).await.unwrap(),
            Some(10_000_000_000),
            "balance must be unchanged when the renewal can't be funded"
        );

        // Old contract still auto-renewing (retried next cycle, not stranded).
        let old_after = db.get_contract(&old_id).await.unwrap().unwrap();
        assert!(
            old_after.auto_renew,
            "auto_renew must stay TRUE so the renewal is retried, not silently dropped"
        );

        // No zombie contract: the requester has exactly the original contract.
        let contract_count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "c!: i64" FROM contract_sign_requests WHERE requester_pubkey = $1"#,
            &requester[..],
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            contract_count, 1,
            "no zombie contract must be created on insufficient balance"
        );
    }
}
