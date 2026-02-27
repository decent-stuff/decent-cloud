pub mod chatwoot;
pub mod cloud;
pub mod crypto;
pub mod database;
pub mod helpcenter;
mod icpay_client;
pub mod invoice_storage;
pub mod invoices;
mod ledger_path;
pub mod notifications;
pub mod payment_release_service;
pub mod receipts;
pub mod regions;
pub mod rental_notifications;
mod search;
pub mod stripe_client;
pub mod support_bot;

/// Returns the current UTC time as nanoseconds since Unix epoch.
///
/// Fails loudly if the timestamp overflows (would only occur past year 2262).
#[inline]
pub fn now_ns() -> anyhow::Result<i64> {
    chrono::Utc::now()
        .timestamp_nanos_opt()
        .ok_or_else(|| anyhow::anyhow!("timestamp overflow (year > 2262)"))
}
