use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResellerRelationship {
    pub id: i64,
    pub reseller_pubkey: Vec<u8>,
    pub external_provider_pubkey: Vec<u8>,
    pub commission_percent: i64,
    pub status: String,
    pub created_at_ns: i64,
    pub updated_at_ns: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResellerOrder {
    pub id: i64,
    pub contract_id: Vec<u8>,
    pub reseller_pubkey: Vec<u8>,
    pub external_provider_pubkey: Vec<u8>,
    pub offering_id: i64,
    pub base_price_e9s: i64,
    pub commission_e9s: i64,
    pub total_paid_e9s: i64,
    pub external_order_id: Option<String>,
    pub external_order_details: Option<String>,
    pub status: String,
    pub created_at_ns: i64,
    pub fulfilled_at_ns: Option<i64>,
}

impl Database {
    /// Create a new reseller relationship
    pub async fn create_reseller_relationship(
        &self,
        reseller_pubkey: &[u8],
        external_provider_pubkey: &[u8],
        commission_percent: i64,
    ) -> Result<i64> {
        // Validate commission_percent range (0-50)
        if !(0..=50).contains(&commission_percent) {
            anyhow::bail!(
                "commission_percent must be between 0 and 50, got {}",
                commission_percent
            );
        }

        let created_at_ns = crate::now_ns()?;

        let id = sqlx::query_scalar!(
            r#"INSERT INTO reseller_relationships (reseller_pubkey, external_provider_pubkey, commission_percent, status, created_at_ns)
               VALUES ($1, $2, $3, 'active', $4)
               RETURNING id"#,
            reseller_pubkey,
            external_provider_pubkey,
            commission_percent,
            created_at_ns
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Update a reseller relationship by reseller and external provider pubkeys
    pub async fn update_reseller_relationship_by_pubkeys(
        &self,
        reseller_pubkey: &[u8],
        external_provider_pubkey: &[u8],
        commission_percent: Option<i64>,
        status: Option<&str>,
    ) -> Result<()> {
        // Validate commission_percent if provided
        if let Some(pct) = commission_percent {
            if !(0..=50).contains(&pct) {
                anyhow::bail!("commission_percent must be between 0 and 50, got {}", pct);
            }
        }

        let updated_at_ns = crate::now_ns()?;

        // Get current values
        let current = sqlx::query!(
            "SELECT commission_percent, status FROM reseller_relationships WHERE reseller_pubkey = $1 AND external_provider_pubkey = $2",
            reseller_pubkey,
            external_provider_pubkey
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Reseller relationship not found"))?;

        let new_commission = commission_percent.unwrap_or(current.commission_percent);
        let new_status = status.unwrap_or(&current.status);

        let result = sqlx::query!(
            "UPDATE reseller_relationships SET commission_percent = $1, status = $2, updated_at_ns = $3 WHERE reseller_pubkey = $4 AND external_provider_pubkey = $5",
            new_commission,
            new_status,
            updated_at_ns,
            reseller_pubkey,
            external_provider_pubkey
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Reseller relationship not found");
        }

        Ok(())
    }

    /// Delete a reseller relationship by reseller and external provider pubkeys
    pub async fn delete_reseller_relationship_by_pubkeys(
        &self,
        reseller_pubkey: &[u8],
        external_provider_pubkey: &[u8],
    ) -> Result<()> {
        let result = sqlx::query!(
            "DELETE FROM reseller_relationships WHERE reseller_pubkey = $1 AND external_provider_pubkey = $2",
            reseller_pubkey,
            external_provider_pubkey
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Reseller relationship not found");
        }

        Ok(())
    }

    /// List all reseller relationships for a given provider (as reseller)
    pub async fn list_reseller_relationships_for_provider(
        &self,
        pubkey: &[u8],
    ) -> Result<Vec<ResellerRelationship>> {
        let relationships = sqlx::query_as!(
            ResellerRelationship,
            r#"SELECT id as "id!", reseller_pubkey, external_provider_pubkey, commission_percent as "commission_percent!", status as "status!", created_at_ns as "created_at_ns!", updated_at_ns FROM reseller_relationships WHERE reseller_pubkey = $1 ORDER BY created_at_ns DESC"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(relationships)
    }

    /// Get a reseller order by contract_id
    pub async fn get_reseller_order(&self, contract_id: &[u8]) -> Result<Option<ResellerOrder>> {
        let order = sqlx::query_as!(
            ResellerOrder,
            r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status as "status!", created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE contract_id = $1"#,
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    /// List reseller orders for a provider as reseller (orders they placed) with optional status filter
    pub async fn list_reseller_orders_for_provider(
        &self,
        pubkey: &[u8],
        status_filter: Option<&str>,
    ) -> Result<Vec<ResellerOrder>> {
        let orders = if let Some(status) = status_filter {
            sqlx::query_as!(
                ResellerOrder,
                r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status, created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE reseller_pubkey = $1 AND status = $2 ORDER BY created_at_ns DESC"#,
                pubkey,
                status
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as!(
                ResellerOrder,
                r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status, created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE reseller_pubkey = $1 ORDER BY created_at_ns DESC"#,
                pubkey
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(orders)
    }

    /// Fulfill a reseller order (update with external order details)
    pub async fn fulfill_reseller_order(
        &self,
        contract_id: &[u8],
        external_order_id: &str,
        external_order_details: &str,
    ) -> Result<()> {
        let fulfilled_at_ns = crate::now_ns()?;

        let result = sqlx::query!(
            "UPDATE reseller_orders SET external_order_id = $1, external_order_details = $2, status = 'fulfilled', fulfilled_at_ns = $3 WHERE contract_id = $4",
            external_order_id,
            external_order_details,
            fulfilled_at_ns,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Reseller order not found");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_create_reseller_relationship() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];

        let id = db
            .create_reseller_relationship(&reseller_pubkey, &external_provider_pubkey, 15)
            .await
            .unwrap();

        assert!(id > 0);

        let relationships = db
            .list_reseller_relationships_for_provider(&reseller_pubkey)
            .await
            .unwrap();
        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].reseller_pubkey, reseller_pubkey);
        assert_eq!(
            relationships[0].external_provider_pubkey,
            external_provider_pubkey
        );
        assert_eq!(relationships[0].commission_percent, 15);
        assert_eq!(relationships[0].status, "active");
    }

    #[tokio::test]
    async fn test_create_reseller_relationship_invalid_commission() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];

        // Test below range
        let result = db
            .create_reseller_relationship(&reseller_pubkey, &external_provider_pubkey, -1)
            .await;
        assert!(result.is_err());

        // Test above range
        let result = db
            .create_reseller_relationship(&reseller_pubkey, &external_provider_pubkey, 51)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_reseller_relationships_for_provider() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_1 = vec![2u8; 32];
        let external_provider_2 = vec![3u8; 32];

        db.create_reseller_relationship(&reseller_pubkey, &external_provider_1, 10)
            .await
            .unwrap();
        db.create_reseller_relationship(&reseller_pubkey, &external_provider_2, 20)
            .await
            .unwrap();

        let relationships = db
            .list_reseller_relationships_for_provider(&reseller_pubkey)
            .await
            .unwrap();

        assert_eq!(relationships.len(), 2);
        assert_eq!(relationships[0].reseller_pubkey, reseller_pubkey);
        assert_eq!(relationships[1].reseller_pubkey, reseller_pubkey);
    }

    #[tokio::test]
    async fn test_update_reseller_relationship_by_pubkeys() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];

        db.create_reseller_relationship(&reseller_pubkey, &external_provider_pubkey, 15)
            .await
            .unwrap();

        // Update commission
        db.update_reseller_relationship_by_pubkeys(
            &reseller_pubkey,
            &external_provider_pubkey,
            Some(20),
            None,
        )
        .await
        .unwrap();

        let relationships = db
            .list_reseller_relationships_for_provider(&reseller_pubkey)
            .await
            .unwrap();
        assert_eq!(relationships[0].commission_percent, 20);
        assert_eq!(relationships[0].status, "active");

        // Update status
        db.update_reseller_relationship_by_pubkeys(
            &reseller_pubkey,
            &external_provider_pubkey,
            None,
            Some("suspended"),
        )
        .await
        .unwrap();

        let relationships = db
            .list_reseller_relationships_for_provider(&reseller_pubkey)
            .await
            .unwrap();
        assert_eq!(relationships[0].commission_percent, 20);
        assert_eq!(relationships[0].status, "suspended");
    }

    #[tokio::test]
    async fn test_delete_reseller_relationship_by_pubkeys() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];

        db.create_reseller_relationship(&reseller_pubkey, &external_provider_pubkey, 15)
            .await
            .unwrap();

        db.delete_reseller_relationship_by_pubkeys(&reseller_pubkey, &external_provider_pubkey)
            .await
            .unwrap();

        let relationships = db
            .list_reseller_relationships_for_provider(&reseller_pubkey)
            .await
            .unwrap();
        assert!(relationships.is_empty());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_relationship_by_pubkeys() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];

        let result = db
            .delete_reseller_relationship_by_pubkeys(&reseller_pubkey, &external_provider_pubkey)
            .await;
        assert!(result.is_err());
    }
}
