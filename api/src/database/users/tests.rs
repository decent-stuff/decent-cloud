use super::*;

#[test]
fn export_typescript_types() {
    UserActivity::export().expect("Failed to export UserActivity type");
    AccountContact::export().expect("Failed to export AccountContact type");
    AccountSocial::export().expect("Failed to export AccountSocial type");
    AccountExternalKey::export().expect("Failed to export AccountExternalKey type");
    OfferingStatsWeek::export().expect("Failed to export OfferingStatsWeek type");
}

#[test]
fn test_offering_stats_week_serializes_camelcase() {
    let row = OfferingStatsWeek {
        week_start: "2024-01-08".to_string(),
        offering_id: "pool-small".to_string(),
        total_requests: 5,
        active_count: 2,
        revenue_e9s: 3_000_000_000,
    };
    let json = serde_json::to_value(&row).unwrap();
    assert_eq!(json["weekStart"], "2024-01-08");
    assert_eq!(json["offeringId"], "pool-small");
    assert_eq!(json["totalRequests"], 5_i64);
    assert_eq!(json["activeCount"], 2_i64);
    assert_eq!(json["revenueE9s"], 3_000_000_000_i64);
    // Ensure no snake_case keys leaked
    assert!(json.get("week_start").is_none());
    assert!(json.get("offering_id").is_none());
}
