//! Decent Agents beta waitlist (issue #423).
//!
//! Public, unauthenticated signup capture used during the soft-launch period to
//! throttle onboarding. The endpoint is idempotent on email: a duplicate signup
//! returns the existing row rather than surfacing a UNIQUE-violation error.

use super::types::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct AgentsWaitlistEntry {
    pub id: i64,
    pub email: String,
    pub github_handle: String,
    pub created_at: DateTime<Utc>,
    pub source: Option<String>,
    pub notes: Option<String>,
}

impl Database {
    /// Insert a new waitlist signup; if `email` already exists, return the
    /// existing row (idempotent) so duplicate clicks do not 500 the user.
    /// `position` is returned alongside so the caller can render "you are #N".
    pub async fn add_to_waitlist(
        &self,
        email: &str,
        github_handle: &str,
        source: Option<&str>,
    ) -> Result<(AgentsWaitlistEntry, i64)> {
        let entry: AgentsWaitlistEntry = sqlx::query_as(
            "INSERT INTO agents_waitlist (email, github_handle, source)
             VALUES ($1, $2, $3)
             ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
             RETURNING id, email, github_handle, created_at, source, notes",
        )
        .bind(email)
        .bind(github_handle)
        .bind(source)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to upsert waitlist entry for email {}", email))?;

        // `position` = total signups at the moment this row is visible.  The new
        // row is already committed by the INSERT above, so the count includes it.
        let position: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents_waitlist")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count waitlist entries")?;

        Ok((entry, position))
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_add_to_waitlist_happy_path() {
        let db = setup_test_db().await;

        let (entry, position) = db
            .add_to_waitlist("alice@example.com", "alice", Some("landing"))
            .await
            .expect("first signup should succeed");

        assert_eq!(entry.email, "alice@example.com");
        assert_eq!(entry.github_handle, "alice");
        assert_eq!(entry.source.as_deref(), Some("landing"));
        assert!(entry.notes.is_none());
        assert!(entry.id > 0);
        assert_eq!(position, 1, "first signup should be position 1");
    }

    #[tokio::test]
    async fn test_add_to_waitlist_is_idempotent_on_duplicate_email() {
        let db = setup_test_db().await;

        let (first, pos1) = db
            .add_to_waitlist("bob@example.com", "bob-original", Some("landing"))
            .await
            .expect("first signup should succeed");
        assert_eq!(pos1, 1);

        // Same email, different handle and source: must return original row,
        // not error and not overwrite the existing fields.
        let (retry, pos2) = db
            .add_to_waitlist("bob@example.com", "bob-different", Some("hn"))
            .await
            .expect("duplicate email retry must not error");

        assert_eq!(retry.id, first.id, "duplicate must return same row id");
        assert_eq!(
            retry.github_handle, "bob-original",
            "original github_handle must be preserved"
        );
        assert_eq!(
            retry.source.as_deref(),
            Some("landing"),
            "original source must be preserved"
        );
        assert_eq!(retry.created_at, first.created_at);
        assert_eq!(pos2, 1, "duplicate must not change row count");
    }

    #[tokio::test]
    async fn test_add_to_waitlist_position_increments() {
        let db = setup_test_db().await;

        let (_, p1) = db
            .add_to_waitlist("a@example.com", "a", None)
            .await
            .unwrap();
        let (_, p2) = db
            .add_to_waitlist("b@example.com", "b", None)
            .await
            .unwrap();
        let (_, p3) = db
            .add_to_waitlist("c@example.com", "c", None)
            .await
            .unwrap();

        assert_eq!((p1, p2, p3), (1, 2, 3));
    }
}
