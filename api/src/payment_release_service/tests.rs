#[test]
fn test_release_calculation_half_time_elapsed() {
    // Contract: 100 e9s for 30 days (720 hours)
    // Time elapsed: 15 days (360 hours)
    // Expected release: 50 e9s
    let payment_amount_e9s = 100;
    let start_ns = 0;
    let end_ns = 30 * 24 * 3600 * 1_000_000_000i64; // 30 days in ns
    let current_ns = 15 * 24 * 3600 * 1_000_000_000i64; // 15 days in ns
    let last_release_ns = start_ns;

    let period_duration_ns = current_ns - last_release_ns;
    let total_duration_ns = end_ns - start_ns;
    let release_amount = (payment_amount_e9s as f64 * period_duration_ns as f64
        / total_duration_ns as f64) as i64;

    assert_eq!(release_amount, 50);
}

#[test]
fn test_release_calculation_one_day_out_of_thirty() {
    // Contract: 720 e9s for 30 days
    // Time elapsed: 1 day
    // Expected release: 24 e9s
    let payment_amount_e9s = 720;
    let start_ns = 0;
    let end_ns = 30 * 24 * 3600 * 1_000_000_000i64;
    let current_ns = 1 * 24 * 3600 * 1_000_000_000i64;
    let last_release_ns = start_ns;

    let period_duration_ns = current_ns - last_release_ns;
    let total_duration_ns = end_ns - start_ns;
    let release_amount = (payment_amount_e9s as f64 * period_duration_ns as f64
        / total_duration_ns as f64) as i64;

    assert_eq!(release_amount, 24);
}

#[test]
fn test_release_calculation_daily_incremental() {
    // Contract: 300 e9s for 10 days
    // Daily releases should sum to ~300 e9s
    let payment_amount_e9s = 300;
    let start_ns = 0;
    let end_ns = 10 * 24 * 3600 * 1_000_000_000i64;
    let total_duration_ns = end_ns - start_ns;

    let mut total_released = 0i64;
    let mut last_release_ns = start_ns;

    for day in 1..=10 {
        let current_ns = day * 24 * 3600 * 1_000_000_000i64;
        let period_duration_ns = current_ns - last_release_ns;
        let release_amount = (payment_amount_e9s as f64 * period_duration_ns as f64
            / total_duration_ns as f64) as i64;

        total_released += release_amount;
        last_release_ns = current_ns;
    }

    // Allow for small rounding differences
    assert!((total_released - payment_amount_e9s).abs() <= 1);
}

#[test]
fn test_release_calculation_no_time_elapsed() {
    // Contract just started, no time elapsed
    // Expected release: 0 e9s
    let payment_amount_e9s = 1000;
    let start_ns = 0;
    let end_ns = 30 * 24 * 3600 * 1_000_000_000i64;
    let current_ns = start_ns;
    let last_release_ns = start_ns;

    let period_duration_ns = current_ns - last_release_ns;
    let total_duration_ns = end_ns - start_ns;
    let release_amount = (payment_amount_e9s as f64 * period_duration_ns as f64
        / total_duration_ns as f64) as i64;

    assert_eq!(release_amount, 0);
}

#[test]
fn test_release_calculation_contract_ended() {
    // Contract: 1000 e9s for 30 days
    // Time elapsed: 30 days (full duration)
    // Expected release: 1000 e9s (if no prior releases)
    let payment_amount_e9s = 1000;
    let start_ns = 0;
    let end_ns = 30 * 24 * 3600 * 1_000_000_000i64;
    let current_ns = end_ns;
    let last_release_ns = start_ns;

    let period_duration_ns = current_ns - last_release_ns;
    let total_duration_ns = end_ns - start_ns;
    let release_amount = (payment_amount_e9s as f64 * period_duration_ns as f64
        / total_duration_ns as f64) as i64;

    assert_eq!(release_amount, 1000);
}
