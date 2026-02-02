//! Visibility allowlist management for shared offerings (Phase 2)
//!
//! Allows providers to grant specific users access to non-public offerings.

use super::types::Database;
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// An entry in the visibility allowlist
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct AllowlistEntry {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub offering_id: i64,
    /// Hex-encoded public key of the allowed user
    pub allowed_pubkey: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

impl Database {
    /// Add a pubkey to an offering's visibility allowlist.
    /// Returns the new entry ID.
    pub async fn add_to_allowlist(
        &self,
        offering_id: i64,
        allowed_pubkey: &[u8],
        owner_pubkey: &[u8],
    ) -> Result<i64> {
        // Verify ownership of the offering
        let owner: Option<Vec<u8>> = sqlx::query_scalar!(
            "SELECT pubkey FROM provider_offerings WHERE id = $1",
            offering_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match owner {
            None => anyhow::bail!("Offering not found"),
            Some(owner_pk) if owner_pk != owner_pubkey => {
                anyhow::bail!("Unauthorized: You do not own this offering")
            }
            _ => {}
        }

        // Insert into allowlist (ON CONFLICT to handle duplicates gracefully)
        let id: i64 = sqlx::query_scalar(
            r#"INSERT INTO visibility_allowlist (offering_id, allowed_pubkey)
               VALUES ($1, $2)
               ON CONFLICT (offering_id, allowed_pubkey) DO UPDATE SET offering_id = $1
               RETURNING id"#,
        )
        .bind(offering_id)
        .bind(allowed_pubkey)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Remove a pubkey from an offering's visibility allowlist.
    /// Returns true if an entry was deleted.
    pub async fn remove_from_allowlist(
        &self,
        offering_id: i64,
        allowed_pubkey: &[u8],
        owner_pubkey: &[u8],
    ) -> Result<bool> {
        // Verify ownership of the offering
        let owner: Option<Vec<u8>> = sqlx::query_scalar!(
            "SELECT pubkey FROM provider_offerings WHERE id = $1",
            offering_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match owner {
            None => anyhow::bail!("Offering not found"),
            Some(owner_pk) if owner_pk != owner_pubkey => {
                anyhow::bail!("Unauthorized: You do not own this offering")
            }
            _ => {}
        }

        let result = sqlx::query(
            "DELETE FROM visibility_allowlist WHERE offering_id = $1 AND allowed_pubkey = $2",
        )
        .bind(offering_id)
        .bind(allowed_pubkey)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get all entries in an offering's allowlist.
    /// Only the owner can list the allowlist.
    pub async fn get_allowlist(
        &self,
        offering_id: i64,
        owner_pubkey: &[u8],
    ) -> Result<Vec<AllowlistEntry>> {
        // Verify ownership of the offering
        let owner: Option<Vec<u8>> = sqlx::query_scalar!(
            "SELECT pubkey FROM provider_offerings WHERE id = $1",
            offering_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match owner {
            None => anyhow::bail!("Offering not found"),
            Some(owner_pk) if owner_pk != owner_pubkey => {
                anyhow::bail!("Unauthorized: You do not own this offering")
            }
            _ => {}
        }

        let entries = sqlx::query_as::<_, AllowlistEntry>(
            r#"SELECT id, offering_id, lower(encode(allowed_pubkey, 'hex')) as allowed_pubkey, created_at
               FROM visibility_allowlist
               WHERE offering_id = $1
               ORDER BY created_at ASC"#,
        )
        .bind(offering_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Check if a pubkey is in an offering's visibility allowlist.
    pub async fn is_in_allowlist(&self, offering_id: i64, pubkey: &[u8]) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(
                SELECT 1 FROM visibility_allowlist
                WHERE offering_id = $1 AND allowed_pubkey = $2
            )"#,
        )
        .bind(offering_id)
        .bind(pubkey)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    /// Check if a user can access an offering based on visibility rules.
    ///
    /// Access is granted if:
    /// - Offering is public
    /// - User is the owner (provider)
    /// - Offering is shared AND user is in the allowlist
    pub async fn can_access_offering(
        &self,
        offering_id: i64,
        visibility: &str,
        owner_pubkey_hex: &str,
        requester_pubkey: Option<&[u8]>,
    ) -> Result<bool> {
        // Public offerings are accessible to everyone
        if visibility.eq_ignore_ascii_case("public") {
            return Ok(true);
        }

        // No authentication means no access to non-public offerings
        let requester = match requester_pubkey {
            Some(pk) => pk,
            None => return Ok(false),
        };

        let requester_hex = hex::encode(requester);

        // Owner can always access their own offerings
        if requester_hex == owner_pubkey_hex {
            return Ok(true);
        }

        // For shared offerings, check allowlist
        if visibility.eq_ignore_ascii_case("shared") {
            return self.is_in_allowlist(offering_id, requester).await;
        }

        // Private offerings are only accessible to owner (checked above)
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_allowlist_crud() {
        let db = setup_test_db().await;

        // Create a provider pubkey and offering
        let provider_pubkey = vec![1u8; 32];
        let allowed_user = vec![2u8; 32];

        // Create a test offering
        let offering = crate::database::offerings::Offering {
            id: None,
            pubkey: hex::encode(&provider_pubkey),
            offering_id: "test-allowlist-offering".to_string(),
            offer_name: "Test Offering".to_string(),
            description: None,
            product_page_url: None,
            currency: "USD".to_string(),
            monthly_price: 10.0,
            setup_fee: 0.0,
            visibility: "shared".to_string(),
            product_type: "compute".to_string(),
            virtualization_type: None,
            billing_interval: "monthly".to_string(),
            billing_unit: "month".to_string(),
            pricing_model: None,
            price_per_unit: None,
            included_units: None,
            overage_price_per_unit: None,
            stripe_metered_price_id: None,
            is_subscription: false,
            subscription_interval_days: None,
            stock_status: "in_stock".to_string(),
            processor_brand: None,
            processor_amount: None,
            processor_cores: None,
            processor_speed: None,
            processor_name: None,
            memory_error_correction: None,
            memory_type: None,
            memory_amount: None,
            hdd_amount: None,
            total_hdd_capacity: None,
            ssd_amount: None,
            total_ssd_capacity: None,
            unmetered_bandwidth: false,
            uplink_speed: None,
            traffic: None,
            datacenter_country: "US".to_string(),
            datacenter_city: "New York".to_string(),
            datacenter_latitude: None,
            datacenter_longitude: None,
            control_panel: None,
            gpu_name: None,
            gpu_count: None,
            gpu_memory_gb: None,
            min_contract_hours: None,
            max_contract_hours: None,
            payment_methods: None,
            features: None,
            operating_systems: None,
            trust_score: None,
            has_critical_flags: None,
            is_example: false,
            offering_source: None,
            external_checkout_url: None,
            reseller_name: None,
            reseller_commission_percent: None,
            owner_username: None,
            provisioner_type: None,
            provisioner_config: None,
            template_name: None,
            agent_pool_id: None,
            provider_online: None,
            resolved_pool_id: None,
            resolved_pool_name: None,
        };

        let offering_id = db
            .create_offering(&provider_pubkey, offering)
            .await
            .expect("Failed to create offering");

        // Test adding to allowlist
        let entry_id = db
            .add_to_allowlist(offering_id, &allowed_user, &provider_pubkey)
            .await
            .expect("Failed to add to allowlist");
        assert!(entry_id > 0);

        // Test is_in_allowlist
        let in_list = db
            .is_in_allowlist(offering_id, &allowed_user)
            .await
            .expect("Failed to check allowlist");
        assert!(in_list);

        let not_in_list = db
            .is_in_allowlist(offering_id, &[3u8; 32])
            .await
            .expect("Failed to check allowlist");
        assert!(!not_in_list);

        // Test get_allowlist
        let entries = db
            .get_allowlist(offering_id, &provider_pubkey)
            .await
            .expect("Failed to get allowlist");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].allowed_pubkey, hex::encode(&allowed_user));

        // Test can_access_offering
        let provider_pubkey_hex = hex::encode(&provider_pubkey);

        // Public - anyone can access
        let can_access = db
            .can_access_offering(offering_id, "public", &provider_pubkey_hex, None)
            .await
            .expect("Failed to check access");
        assert!(can_access);

        // Shared - allowlisted user can access
        let can_access = db
            .can_access_offering(
                offering_id,
                "shared",
                &provider_pubkey_hex,
                Some(&allowed_user),
            )
            .await
            .expect("Failed to check access");
        assert!(can_access);

        // Shared - non-allowlisted user cannot access
        let can_access = db
            .can_access_offering(
                offering_id,
                "shared",
                &provider_pubkey_hex,
                Some(&[3u8; 32]),
            )
            .await
            .expect("Failed to check access");
        assert!(!can_access);

        // Private - only owner can access
        let can_access = db
            .can_access_offering(
                offering_id,
                "private",
                &provider_pubkey_hex,
                Some(&allowed_user),
            )
            .await
            .expect("Failed to check access");
        assert!(!can_access);

        // Owner can always access
        let can_access = db
            .can_access_offering(
                offering_id,
                "private",
                &provider_pubkey_hex,
                Some(&provider_pubkey),
            )
            .await
            .expect("Failed to check access");
        assert!(can_access);

        // Test removing from allowlist
        let removed = db
            .remove_from_allowlist(offering_id, &allowed_user, &provider_pubkey)
            .await
            .expect("Failed to remove from allowlist");
        assert!(removed);

        let in_list_after = db
            .is_in_allowlist(offering_id, &allowed_user)
            .await
            .expect("Failed to check allowlist");
        assert!(!in_list_after);

        // Cleanup
        db.delete_offering(&provider_pubkey, offering_id)
            .await
            .expect("Failed to delete offering");
    }

    #[tokio::test]
    async fn test_allowlist_authorization() {
        let db = setup_test_db().await;

        let provider_pubkey = vec![4u8; 32];
        let other_user = vec![5u8; 32];

        // Create a test offering
        let offering = crate::database::offerings::Offering {
            id: None,
            pubkey: hex::encode(&provider_pubkey),
            offering_id: "test-auth-offering".to_string(),
            offer_name: "Test Offering".to_string(),
            description: None,
            product_page_url: None,
            currency: "USD".to_string(),
            monthly_price: 10.0,
            setup_fee: 0.0,
            visibility: "shared".to_string(),
            product_type: "compute".to_string(),
            virtualization_type: None,
            billing_interval: "monthly".to_string(),
            billing_unit: "month".to_string(),
            pricing_model: None,
            price_per_unit: None,
            included_units: None,
            overage_price_per_unit: None,
            stripe_metered_price_id: None,
            is_subscription: false,
            subscription_interval_days: None,
            stock_status: "in_stock".to_string(),
            processor_brand: None,
            processor_amount: None,
            processor_cores: None,
            processor_speed: None,
            processor_name: None,
            memory_error_correction: None,
            memory_type: None,
            memory_amount: None,
            hdd_amount: None,
            total_hdd_capacity: None,
            ssd_amount: None,
            total_ssd_capacity: None,
            unmetered_bandwidth: false,
            uplink_speed: None,
            traffic: None,
            datacenter_country: "US".to_string(),
            datacenter_city: "New York".to_string(),
            datacenter_latitude: None,
            datacenter_longitude: None,
            control_panel: None,
            gpu_name: None,
            gpu_count: None,
            gpu_memory_gb: None,
            min_contract_hours: None,
            max_contract_hours: None,
            payment_methods: None,
            features: None,
            operating_systems: None,
            trust_score: None,
            has_critical_flags: None,
            is_example: false,
            offering_source: None,
            external_checkout_url: None,
            reseller_name: None,
            reseller_commission_percent: None,
            owner_username: None,
            provisioner_type: None,
            provisioner_config: None,
            template_name: None,
            agent_pool_id: None,
            provider_online: None,
            resolved_pool_id: None,
            resolved_pool_name: None,
        };

        let offering_id = db
            .create_offering(&provider_pubkey, offering)
            .await
            .expect("Failed to create offering");

        // Non-owner should not be able to add to allowlist
        let result = db
            .add_to_allowlist(offering_id, &[6u8; 32], &other_user)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));

        // Non-owner should not be able to get allowlist
        let result = db.get_allowlist(offering_id, &other_user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unauthorized"));

        // Cleanup
        db.delete_offering(&provider_pubkey, offering_id)
            .await
            .expect("Failed to delete offering");
    }
}
