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

        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let id = sqlx::query_scalar!(
            r#"INSERT INTO reseller_relationships (reseller_pubkey, external_provider_pubkey, commission_percent, status, created_at_ns)
               VALUES (?, ?, ?, 'active', ?)
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

    /// Update an existing reseller relationship
    pub async fn update_reseller_relationship(
        &self,
        id: i64,
        commission_percent: Option<i64>,
        status: Option<&str>,
    ) -> Result<()> {
        // Validate commission_percent if provided
        if let Some(pct) = commission_percent {
            if !(0..=50).contains(&pct) {
                anyhow::bail!("commission_percent must be between 0 and 50, got {}", pct);
            }
        }

        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // Build dynamic update query based on provided fields
        if let Some(pct) = commission_percent {
            if let Some(st) = status {
                sqlx::query!(
                    "UPDATE reseller_relationships SET commission_percent = ?, status = ?, updated_at_ns = ? WHERE id = ?",
                    pct,
                    st,
                    updated_at_ns,
                    id
                )
                .execute(&self.pool)
                .await?;
            } else {
                sqlx::query!(
                    "UPDATE reseller_relationships SET commission_percent = ?, updated_at_ns = ? WHERE id = ?",
                    pct,
                    updated_at_ns,
                    id
                )
                .execute(&self.pool)
                .await?;
            }
        } else if let Some(st) = status {
            sqlx::query!(
                "UPDATE reseller_relationships SET status = ?, updated_at_ns = ? WHERE id = ?",
                st,
                updated_at_ns,
                id
            )
            .execute(&self.pool)
            .await?;
        } else {
            anyhow::bail!("At least one of commission_percent or status must be provided");
        }

        Ok(())
    }

    /// Get a reseller relationship by id
    pub async fn get_reseller_relationship(&self, id: i64) -> Result<Option<ResellerRelationship>> {
        let relationship = sqlx::query_as!(
            ResellerRelationship,
            r#"SELECT id as "id!", reseller_pubkey, external_provider_pubkey, commission_percent as "commission_percent!", status as "status!", created_at_ns as "created_at_ns!", updated_at_ns FROM reseller_relationships WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(relationship)
    }

    /// Get a reseller relationship by reseller and external provider pubkeys
    pub async fn get_reseller_relationship_by_pubkeys(
        &self,
        reseller_pubkey: &[u8],
        external_provider_pubkey: &[u8],
    ) -> Result<Option<ResellerRelationship>> {
        let relationship = sqlx::query_as!(
            ResellerRelationship,
            r#"SELECT id as "id!", reseller_pubkey, external_provider_pubkey, commission_percent as "commission_percent!", status as "status!", created_at_ns as "created_at_ns!", updated_at_ns FROM reseller_relationships WHERE reseller_pubkey = ? AND external_provider_pubkey = ?"#,
            reseller_pubkey,
            external_provider_pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(relationship)
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

        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // Get current values
        let current = sqlx::query!(
            "SELECT commission_percent, status FROM reseller_relationships WHERE reseller_pubkey = ? AND external_provider_pubkey = ?",
            reseller_pubkey,
            external_provider_pubkey
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Reseller relationship not found"))?;

        let new_commission = commission_percent.unwrap_or(current.commission_percent);
        let new_status = status.unwrap_or(&current.status);

        let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
            "UPDATE reseller_relationships SET commission_percent = ?, status = ?, updated_at_ns = ? WHERE reseller_pubkey = ? AND external_provider_pubkey = ?",
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
        let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
            "DELETE FROM reseller_relationships WHERE reseller_pubkey = ? AND external_provider_pubkey = ?",
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
            r#"SELECT id as "id!", reseller_pubkey, external_provider_pubkey, commission_percent as "commission_percent!", status as "status!", created_at_ns as "created_at_ns!", updated_at_ns FROM reseller_relationships WHERE reseller_pubkey = ? ORDER BY created_at_ns DESC"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(relationships)
    }

    /// Delete a reseller relationship
    pub async fn delete_reseller_relationship(&self, id: i64) -> Result<()> {
        let result: sqlx::sqlite::SqliteQueryResult =
            sqlx::query!("DELETE FROM reseller_relationships WHERE id = ?", id)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Reseller relationship not found");
        }

        Ok(())
    }

    /// Create a new reseller order
    pub async fn create_reseller_order(
        &self,
        contract_id: &[u8],
        reseller_pubkey: &[u8],
        external_provider_pubkey: &[u8],
        offering_id: i64,
        base_price_e9s: i64,
        commission_e9s: i64,
        total_paid_e9s: i64,
    ) -> Result<i64> {
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let id = sqlx::query_scalar!(
            r#"INSERT INTO reseller_orders (contract_id, reseller_pubkey, external_provider_pubkey, offering_id, base_price_e9s, commission_e9s, total_paid_e9s, status, created_at_ns)
               VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)
               RETURNING id"#,
            contract_id,
            reseller_pubkey,
            external_provider_pubkey,
            offering_id,
            base_price_e9s,
            commission_e9s,
            total_paid_e9s,
            created_at_ns
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get a reseller order by contract_id
    pub async fn get_reseller_order(&self, contract_id: &[u8]) -> Result<Option<ResellerOrder>> {
        let order = sqlx::query_as!(
            ResellerOrder,
            r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status as "status!", created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE contract_id = ?"#,
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
                r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status, created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE reseller_pubkey = ? AND status = ? ORDER BY created_at_ns DESC"#,
                pubkey,
                status
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as!(
                ResellerOrder,
                r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status, created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE reseller_pubkey = ? ORDER BY created_at_ns DESC"#,
                pubkey
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(orders)
    }

    /// List reseller orders for an external provider (orders they need to fulfill) with optional status filter
    pub async fn list_reseller_orders_for_external_provider(
        &self,
        pubkey: &[u8],
        status_filter: Option<&str>,
    ) -> Result<Vec<ResellerOrder>> {
        let orders = if let Some(status) = status_filter {
            sqlx::query_as!(
                ResellerOrder,
                r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status, created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE external_provider_pubkey = ? AND status = ? ORDER BY created_at_ns DESC"#,
                pubkey,
                status
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as!(
                ResellerOrder,
                r#"SELECT id as "id!", contract_id, reseller_pubkey, external_provider_pubkey, offering_id as "offering_id!", base_price_e9s as "base_price_e9s!", commission_e9s as "commission_e9s!", total_paid_e9s as "total_paid_e9s!", external_order_id, external_order_details, status, created_at_ns as "created_at_ns!", fulfilled_at_ns FROM reseller_orders WHERE external_provider_pubkey = ? ORDER BY created_at_ns DESC"#,
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
        let fulfilled_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
            "UPDATE reseller_orders SET external_order_id = ?, external_order_details = ?, status = 'fulfilled', fulfilled_at_ns = ? WHERE contract_id = ?",
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
    use super::Database;
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

        // Verify created
        let relationship = db.get_reseller_relationship(id).await.unwrap().unwrap();
        assert_eq!(relationship.reseller_pubkey, reseller_pubkey);
        assert_eq!(
            relationship.external_provider_pubkey,
            external_provider_pubkey
        );
        assert_eq!(relationship.commission_percent, 15);
        assert_eq!(relationship.status, "active");
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
    async fn test_update_reseller_relationship() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];

        let id = db
            .create_reseller_relationship(&reseller_pubkey, &external_provider_pubkey, 15)
            .await
            .unwrap();

        // Update commission
        db.update_reseller_relationship(id, Some(20), None)
            .await
            .unwrap();

        let relationship = db.get_reseller_relationship(id).await.unwrap().unwrap();
        assert_eq!(relationship.commission_percent, 20);
        assert_eq!(relationship.status, "active");

        // Update status
        db.update_reseller_relationship(id, None, Some("suspended"))
            .await
            .unwrap();

        let relationship = db.get_reseller_relationship(id).await.unwrap().unwrap();
        assert_eq!(relationship.commission_percent, 20);
        assert_eq!(relationship.status, "suspended");
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
    async fn test_delete_reseller_relationship() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];

        let id = db
            .create_reseller_relationship(&reseller_pubkey, &external_provider_pubkey, 15)
            .await
            .unwrap();

        db.delete_reseller_relationship(id).await.unwrap();

        let relationship = db.get_reseller_relationship(id).await.unwrap();
        assert!(relationship.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_relationship() {
        let db = setup_test_db().await;

        let result = db.delete_reseller_relationship(999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_reseller_order() {
        let db = setup_test_db().await;
        let contract_id = vec![1u8; 32];
        let reseller_pubkey = vec![2u8; 32];
        let external_provider_pubkey = vec![3u8; 32];

        let id = db
            .create_reseller_order(
                &contract_id,
                &reseller_pubkey,
                &external_provider_pubkey,
                100,
                1000_000_000,
                150_000_000,
                1150_000_000,
            )
            .await
            .unwrap();

        assert!(id > 0);

        // Verify created
        let order = db.get_reseller_order(&contract_id).await.unwrap().unwrap();
        assert_eq!(order.contract_id, contract_id);
        assert_eq!(order.reseller_pubkey, reseller_pubkey);
        assert_eq!(order.offering_id, 100);
        assert_eq!(order.base_price_e9s, 1000_000_000);
        assert_eq!(order.commission_e9s, 150_000_000);
        assert_eq!(order.total_paid_e9s, 1150_000_000);
        assert_eq!(order.status, "pending");
        assert!(order.external_order_id.is_none());
    }

    #[tokio::test]
    async fn test_fulfill_reseller_order() {
        let db = setup_test_db().await;
        let contract_id = vec![1u8; 32];
        let reseller_pubkey = vec![2u8; 32];
        let external_provider_pubkey = vec![3u8; 32];

        db.create_reseller_order(
            &contract_id,
            &reseller_pubkey,
            &external_provider_pubkey,
            100,
            1000_000_000,
            150_000_000,
            1150_000_000,
        )
        .await
        .unwrap();

        // Fulfill order
        db.fulfill_reseller_order(
            &contract_id,
            "ext-order-123",
            r#"{"instance_id": "i-abc123"}"#,
        )
        .await
        .unwrap();

        let order = db.get_reseller_order(&contract_id).await.unwrap().unwrap();
        assert_eq!(order.status, "fulfilled");
        assert_eq!(order.external_order_id, Some("ext-order-123".to_string()));
        assert_eq!(
            order.external_order_details,
            Some(r#"{"instance_id": "i-abc123"}"#.to_string())
        );
        assert!(order.fulfilled_at_ns.is_some());
    }

    #[tokio::test]
    async fn test_list_reseller_orders_for_provider() {
        let db = setup_test_db().await;
        let reseller_pubkey = vec![1u8; 32];
        let external_provider_pubkey = vec![2u8; 32];
        let contract_id_1 = vec![10u8; 32];
        let contract_id_2 = vec![20u8; 32];

        db.create_reseller_order(
            &contract_id_1,
            &reseller_pubkey,
            &external_provider_pubkey,
            100,
            1000_000_000,
            150_000_000,
            1150_000_000,
        )
        .await
        .unwrap();

        db.create_reseller_order(
            &contract_id_2,
            &reseller_pubkey,
            &external_provider_pubkey,
            101,
            2000_000_000,
            300_000_000,
            2300_000_000,
        )
        .await
        .unwrap();

        // Fulfill one order
        db.fulfill_reseller_order(&contract_id_1, "ext-123", r#"{}"#)
            .await
            .unwrap();

        // List all orders
        let all_orders = db
            .list_reseller_orders_for_provider(&reseller_pubkey, None)
            .await
            .unwrap();
        assert_eq!(all_orders.len(), 2);

        // List pending orders only
        let pending_orders = db
            .list_reseller_orders_for_provider(&reseller_pubkey, Some("pending"))
            .await
            .unwrap();
        assert_eq!(pending_orders.len(), 1);
        assert_eq!(pending_orders[0].contract_id, contract_id_2);

        // List fulfilled orders only
        let fulfilled_orders = db
            .list_reseller_orders_for_provider(&reseller_pubkey, Some("fulfilled"))
            .await
            .unwrap();
        assert_eq!(fulfilled_orders.len(), 1);
        assert_eq!(fulfilled_orders[0].contract_id, contract_id_1);
    }
}
