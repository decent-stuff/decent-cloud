use super::types::Database;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct UpsertOfferingSliReport {
    pub report_date: String,
    pub uptime_percent: f64,
    pub response_sli_percent: Option<f64>,
    pub incident_count: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct OfferingSlaTimelineDay {
    pub date: String,
    pub uptime_percent: f64,
    pub response_sli_percent: Option<f64>,
    pub incident_count: i32,
    pub breached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct OfferingSlaSummary {
    pub offering_id: i64,
    pub sla_target_percent: Option<f64>,
    pub reports_30d: i64,
    pub breach_days_30d: i64,
    pub compliance_30d_percent: Option<f64>,
    pub average_uptime_30d: Option<f64>,
    pub latest_report_date: Option<String>,
    pub latest_uptime_percent: Option<f64>,
    pub latest_response_sli_percent: Option<f64>,
    pub timeline: Vec<OfferingSlaTimelineDay>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ProviderSlaSummary {
    pub offerings_tracked: i64,
    pub reports_30d: i64,
    pub breach_days_30d: i64,
    pub compliance_30d_percent: Option<f64>,
    pub average_uptime_30d: Option<f64>,
    pub average_sla_target_percent: Option<f64>,
    pub latest_report_date: Option<String>,
    pub latest_uptime_percent: Option<f64>,
    pub penalty_points: f64,
}

#[derive(Debug, Clone)]
struct ProviderSliSample {
    report_date: String,
    uptime_percent: f64,
    response_sli_percent: Option<f64>,
    incident_count: i32,
    sla_target_percent: f64,
}

impl Database {
    pub async fn upsert_provider_offering_sli_reports(
        &self,
        provider_pubkey: &[u8],
        offering_id: i64,
        sla_target_percent: f64,
        reports: &[UpsertOfferingSliReport],
    ) -> Result<()> {
        self.ensure_provider_owns_offering(provider_pubkey, offering_id)
            .await?;

        let now_ns = crate::now_ns()?;
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO provider_offering_sla_targets (offering_id, provider_pubkey, sla_target_percent, updated_at_ns)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (offering_id) DO UPDATE
                   SET provider_pubkey = EXCLUDED.provider_pubkey,
                       sla_target_percent = EXCLUDED.sla_target_percent,
                       updated_at_ns = EXCLUDED.updated_at_ns"#,
        )
        .bind(offering_id)
        .bind(provider_pubkey)
        .bind(sla_target_percent)
        .bind(now_ns)
        .execute(&mut *tx)
        .await?;

        for report in reports {
            sqlx::query(
                r#"INSERT INTO provider_offering_sli_reports (
                       offering_id, provider_pubkey, report_date, uptime_percent, response_sli_percent,
                       incident_count, notes, created_at_ns, updated_at_ns
                   )
                   VALUES ($1, $2, $3::date, $4, $5, $6, $7, $8, $8)
                   ON CONFLICT (offering_id, report_date) DO UPDATE
                       SET provider_pubkey = EXCLUDED.provider_pubkey,
                           uptime_percent = EXCLUDED.uptime_percent,
                           response_sli_percent = EXCLUDED.response_sli_percent,
                           incident_count = EXCLUDED.incident_count,
                           notes = EXCLUDED.notes,
                           updated_at_ns = EXCLUDED.updated_at_ns"#,
            )
            .bind(offering_id)
            .bind(provider_pubkey)
            .bind(&report.report_date)
            .bind(report.uptime_percent)
            .bind(report.response_sli_percent)
            .bind(report.incident_count)
            .bind(&report.notes)
            .bind(now_ns)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_offering_sla_summary(
        &self,
        offering_id: i64,
        days: i64,
    ) -> Result<OfferingSlaSummary> {
        let exists: bool =
            sqlx::query_scalar(r#"SELECT EXISTS(SELECT 1 FROM provider_offerings WHERE id = $1)"#)
                .bind(offering_id)
                .fetch_one(&self.pool)
                .await?;

        if !exists {
            return Err(anyhow!("Offering not found"));
        }

        let cutoff_date = (Utc::now().date_naive() - Duration::days(days.saturating_sub(1)))
            .format("%Y-%m-%d")
            .to_string();

        let target = sqlx::query(
            r#"SELECT sla_target_percent
               FROM provider_offering_sla_targets
               WHERE offering_id = $1"#,
        )
        .bind(offering_id)
        .fetch_optional(&self.pool)
        .await?;
        let sla_target_percent = target.map(|row| row.get::<f64, _>("sla_target_percent"));

        let latest = sqlx::query(
            r#"SELECT to_char(report_date, 'YYYY-MM-DD') AS report_date,
                      uptime_percent,
                      response_sli_percent
               FROM provider_offering_sli_reports
               WHERE offering_id = $1
               ORDER BY report_date DESC
               LIMIT 1"#,
        )
        .bind(offering_id)
        .fetch_optional(&self.pool)
        .await?;

        let rows = sqlx::query(
            r#"SELECT to_char(report_date, 'YYYY-MM-DD') AS report_date,
                      uptime_percent,
                      response_sli_percent,
                      incident_count
               FROM provider_offering_sli_reports
               WHERE offering_id = $1
                 AND report_date >= $2::date
               ORDER BY report_date ASC"#,
        )
        .bind(offering_id)
        .bind(&cutoff_date)
        .fetch_all(&self.pool)
        .await?;

        let reports_30d = rows.len() as i64;
        let breach_days_30d = rows
            .iter()
            .filter(|row| {
                sla_target_percent
                    .map(|target| row.get::<f64, _>("uptime_percent") < target)
                    .unwrap_or(false)
            })
            .count() as i64;

        let average_uptime_30d = if rows.is_empty() {
            None
        } else {
            Some(
                rows.iter()
                    .map(|row| row.get::<f64, _>("uptime_percent"))
                    .sum::<f64>()
                    / rows.len() as f64,
            )
        };
        let compliance_30d_percent = if rows.is_empty() {
            None
        } else {
            Some(((rows.len() as i64 - breach_days_30d) as f64 / rows.len() as f64) * 100.0)
        };

        let timeline = rows
            .into_iter()
            .map(|row| {
                let uptime_percent = row.get::<f64, _>("uptime_percent");
                OfferingSlaTimelineDay {
                    date: row.get::<String, _>("report_date"),
                    uptime_percent,
                    response_sli_percent: row.get::<Option<f64>, _>("response_sli_percent"),
                    incident_count: row.get::<i32, _>("incident_count"),
                    breached: sla_target_percent
                        .map(|target| uptime_percent < target)
                        .unwrap_or(false),
                }
            })
            .collect();

        Ok(OfferingSlaSummary {
            offering_id,
            sla_target_percent,
            reports_30d,
            breach_days_30d,
            compliance_30d_percent,
            average_uptime_30d,
            latest_report_date: latest
                .as_ref()
                .map(|row| row.get::<String, _>("report_date")),
            latest_uptime_percent: latest
                .as_ref()
                .map(|row| row.get::<f64, _>("uptime_percent")),
            latest_response_sli_percent: latest
                .as_ref()
                .and_then(|row| row.get::<Option<f64>, _>("response_sli_percent")),
            timeline,
        })
    }

    pub async fn get_provider_sla_summary(
        &self,
        provider_pubkey: &[u8],
        days: i64,
    ) -> Result<ProviderSlaSummary> {
        let offerings_tracked: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*)::BIGINT
               FROM provider_offering_sla_targets
               WHERE provider_pubkey = $1"#,
        )
        .bind(provider_pubkey)
        .fetch_one(&self.pool)
        .await?;

        let samples = self.get_provider_sli_samples(provider_pubkey, days).await?;
        let reports_30d = samples.len() as i64;
        let breach_days_30d = samples
            .iter()
            .filter(|sample| sample.uptime_percent < sample.sla_target_percent)
            .count() as i64;

        let average_uptime_30d = if samples.is_empty() {
            None
        } else {
            Some(
                samples
                    .iter()
                    .map(|sample| sample.uptime_percent)
                    .sum::<f64>()
                    / samples.len() as f64,
            )
        };
        let average_sla_target_percent = if samples.is_empty() {
            None
        } else {
            Some(
                samples
                    .iter()
                    .map(|sample| sample.sla_target_percent)
                    .sum::<f64>()
                    / samples.len() as f64,
            )
        };
        let compliance_30d_percent = if samples.is_empty() {
            None
        } else {
            Some(((samples.len() as i64 - breach_days_30d) as f64 / samples.len() as f64) * 100.0)
        };
        let latest = samples.last();

        Ok(ProviderSlaSummary {
            offerings_tracked,
            reports_30d,
            breach_days_30d,
            compliance_30d_percent,
            average_uptime_30d,
            average_sla_target_percent,
            latest_report_date: latest.map(|sample| sample.report_date.clone()),
            latest_uptime_percent: latest.map(|sample| sample.uptime_percent),
            penalty_points: Self::calculate_provider_sli_penalty_points(&samples),
        })
    }

    pub async fn get_provider_sli_penalty_points(
        &self,
        provider_pubkey: &[u8],
        days: i64,
    ) -> Result<f64> {
        let samples = self.get_provider_sli_samples(provider_pubkey, days).await?;
        Ok(Self::calculate_provider_sli_penalty_points(&samples))
    }

    async fn ensure_provider_owns_offering(
        &self,
        provider_pubkey: &[u8],
        offering_id: i64,
    ) -> Result<()> {
        let exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(
                   SELECT 1
                   FROM provider_offerings
                   WHERE id = $1 AND pubkey = $2
               )"#,
        )
        .bind(offering_id)
        .bind(provider_pubkey)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            Ok(())
        } else {
            Err(anyhow!("Offering not found for provider"))
        }
    }

    async fn get_provider_sli_samples(
        &self,
        provider_pubkey: &[u8],
        days: i64,
    ) -> Result<Vec<ProviderSliSample>> {
        let cutoff_date = (Utc::now().date_naive() - Duration::days(days.saturating_sub(1)))
            .format("%Y-%m-%d")
            .to_string();

        let rows = sqlx::query(
            r#"SELECT to_char(r.report_date, 'YYYY-MM-DD') AS report_date,
                      r.uptime_percent,
                      r.response_sli_percent,
                      r.incident_count,
                      t.sla_target_percent
               FROM provider_offering_sli_reports r
               INNER JOIN provider_offering_sla_targets t ON t.offering_id = r.offering_id
               INNER JOIN provider_offerings o ON o.id = r.offering_id
               WHERE o.pubkey = $1
                 AND r.report_date >= $2::date
               ORDER BY r.report_date ASC, r.offering_id ASC"#,
        )
        .bind(provider_pubkey)
        .bind(&cutoff_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ProviderSliSample {
                report_date: row.get::<String, _>("report_date"),
                uptime_percent: row.get::<f64, _>("uptime_percent"),
                response_sli_percent: row.get::<Option<f64>, _>("response_sli_percent"),
                incident_count: row.get::<i32, _>("incident_count"),
                sla_target_percent: row.get::<f64, _>("sla_target_percent"),
            })
            .collect())
    }

    fn calculate_provider_sli_penalty_points(samples: &[ProviderSliSample]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }

        let breached: Vec<&ProviderSliSample> = samples
            .iter()
            .filter(|sample| sample.uptime_percent < sample.sla_target_percent)
            .collect();

        if breached.is_empty() {
            return 0.0;
        }

        let average_day_penalty = breached
            .iter()
            .map(|sample| {
                let downtime_budget = (100.0 - sample.sla_target_percent).max(0.05);
                let shortfall = (sample.sla_target_percent - sample.uptime_percent).max(0.0);
                let budget_overrun = shortfall / downtime_budget;
                let severity_penalty = budget_overrun.ln_1p() * 12.0;
                let response_penalty = sample
                    .response_sli_percent
                    .map(|response| ((100.0 - response).max(0.0) / 10.0).min(4.0))
                    .unwrap_or(0.0);
                let incident_penalty = (sample.incident_count as f64).min(4.0) * 0.75;
                severity_penalty + response_penalty + incident_penalty
            })
            .sum::<f64>()
            / breached.len() as f64;

        let breach_rate = breached.len() as f64 / samples.len() as f64;
        let coverage_factor = (samples.len() as f64 / 30.0).clamp(0.2, 1.0);
        (average_day_penalty * 0.7 + breach_rate * 18.0 * coverage_factor).clamp(0.0, 45.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    async fn insert_provider_and_offering(db: &Database, pubkey: &[u8], offering_id: i64) {
        sqlx::query(
            "INSERT INTO provider_registrations (pubkey, signature, created_at_ns) VALUES ($1, '\\x00', 0)",
        )
        .bind(pubkey)
        .execute(&db.pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO provider_offerings (id, pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, $2, 'sli-off', 'SLI Offer', 'USD', 10.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'Dallas', FALSE, 0)",
        )
        .bind(offering_id)
        .bind(pubkey)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_get_offering_sla_summary_counts_breaches() {
        let db = setup_test_db().await;
        let pubkey = [0x91u8; 32];
        insert_provider_and_offering(&db, &pubkey, 991).await;

        db.upsert_provider_offering_sli_reports(
            &pubkey,
            991,
            99.9,
            &[
                UpsertOfferingSliReport {
                    report_date: (Utc::now().date_naive() - Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string(),
                    uptime_percent: 99.95,
                    response_sli_percent: Some(99.0),
                    incident_count: 0,
                    notes: None,
                },
                UpsertOfferingSliReport {
                    report_date: Utc::now().date_naive().format("%Y-%m-%d").to_string(),
                    uptime_percent: 99.5,
                    response_sli_percent: Some(96.0),
                    incident_count: 2,
                    notes: Some("packet loss".to_string()),
                },
            ],
        )
        .await
        .unwrap();

        let summary = db.get_offering_sla_summary(991, 30).await.unwrap();
        assert_eq!(summary.sla_target_percent, Some(99.9));
        assert_eq!(summary.reports_30d, 2);
        assert_eq!(summary.breach_days_30d, 1);
        assert_eq!(summary.compliance_30d_percent, Some(50.0));
        assert!(summary.timeline.iter().any(|day| day.breached));
    }

    #[tokio::test]
    async fn test_provider_sli_penalty_points_increase_with_budget_overrun() {
        let mild_penalty = Database::calculate_provider_sli_penalty_points(&[ProviderSliSample {
            report_date: "2026-01-01".to_string(),
            uptime_percent: 99.89,
            response_sli_percent: Some(99.0),
            incident_count: 0,
            sla_target_percent: 99.9,
        }]);
        let severe_penalty =
            Database::calculate_provider_sli_penalty_points(&[ProviderSliSample {
                report_date: "2026-01-01".to_string(),
                uptime_percent: 99.3,
                response_sli_percent: Some(88.0),
                incident_count: 3,
                sla_target_percent: 99.9,
            }]);

        assert!(mild_penalty > 0.0);
        assert!(severe_penalty > mild_penalty);
    }

    #[test]
    fn test_penalty_zero_samples() {
        let penalty = Database::calculate_provider_sli_penalty_points(&[]);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_penalty_all_compliant() {
        let penalty = Database::calculate_provider_sli_penalty_points(&[
            ProviderSliSample {
                report_date: "2026-01-01".to_string(),
                uptime_percent: 99.95,
                response_sli_percent: Some(99.0),
                incident_count: 0,
                sla_target_percent: 99.9,
            },
            ProviderSliSample {
                report_date: "2026-01-02".to_string(),
                uptime_percent: 100.0,
                response_sli_percent: Some(100.0),
                incident_count: 0,
                sla_target_percent: 99.9,
            },
        ]);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_penalty_capped_at_45() {
        let mut samples = Vec::new();
        for i in 0..30 {
            samples.push(ProviderSliSample {
                report_date: format!("2026-01-{i:02}"),
                uptime_percent: 0.0,
                response_sli_percent: Some(0.0),
                incident_count: 100,
                sla_target_percent: 99.9,
            });
        }
        let penalty = Database::calculate_provider_sli_penalty_points(&samples);
        assert_eq!(penalty, 45.0);
    }

    #[test]
    fn test_penalty_coverage_factor_reduces_sparse_data() {
        let sparse = vec![ProviderSliSample {
            report_date: "2026-01-01".to_string(),
            uptime_percent: 99.0,
            response_sli_percent: None,
            incident_count: 0,
            sla_target_percent: 99.9,
        }];
        let mut full = Vec::new();
        for i in 0..30 {
            full.push(ProviderSliSample {
                report_date: format!("2026-01-{i:02}"),
                uptime_percent: 99.0,
                response_sli_percent: None,
                incident_count: 0,
                sla_target_percent: 99.9,
            });
        }
        let sparse_penalty = Database::calculate_provider_sli_penalty_points(&sparse);
        let full_penalty = Database::calculate_provider_sli_penalty_points(&full);
        assert!(sparse_penalty < full_penalty);
    }
}
