//! IP geolocation detection for agent registration.
//!
//! Detects the agent's geographic location based on its public IP address
//! and compares it against the pool's expected location.

use anyhow::{Context, Result};

/// Detect agent's country code using public IP geolocation.
pub async fn detect_country() -> Result<Option<String>> {
    let lookup = public_ip_address::perform_lookup(None)
        .await
        .context("Failed to perform IP geolocation lookup")?;

    Ok(lookup.country_code)
}

// Re-export regions module from dcc-common to avoid duplication
pub use dcc_common::regions::{country_to_region, region_display_name};
