use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::gateway::GatewayManager;
use crate::orphan_tracker::OrphanTracker;
use crate::provisioner::{create_provisioner_map, ProvisionerMap};

mod reconcile;

/// Optional gateway manager wrapped in Arc<Mutex> for shared async access
type OptionalGatewayManager = Option<std::sync::Arc<tokio::sync::Mutex<GatewayManager>>>;

pub async fn run(config: Config) -> Result<()> {
    info!("Starting dc-agent");

    // Validate config for placeholder values before starting
    config.validate()?;

    let api_client = std::sync::Arc::new(ApiClient::new(&config.api)?);
    let (provisioners, default_provisioner_type) = create_provisioner_map(&config)?;

    // Verify provisioner setup before starting the polling loop
    // This catches issues like unreachable Proxmox API early
    let default_provisioner = provisioners
        .get(&default_provisioner_type)
        .expect("default provisioner must exist");
    let verification = default_provisioner.verify_setup().await;
    if !verification.is_ok() {
        error!(
            errors = ?verification.errors,
            "Provisioner setup verification failed"
        );
        anyhow::bail!(
            "Provisioner setup verification failed:\n  - {}\n\nRun 'dc-agent doctor' for detailed diagnostics.",
            verification.errors.join("\n  - ")
        );
    }
    for warning in &verification.warnings {
        warn!(warning = %warning, "Provisioner setup warning");
    }
    info!("Provisioner setup verified successfully");

    // Initialize gateway manager if configured
    let gateway_manager = match &config.gateway {
        Some(gw_config) => match GatewayManager::new(gw_config.clone(), api_client.clone()) {
            Ok(gm) => {
                info!(
                    dc_id = %gw_config.dc_id,
                    public_ip = %gw_config.public_ip,
                    port_range = %format!("{}-{}", gw_config.port_range_start, gw_config.port_range_end),
                    "Gateway manager initialized"
                );
                Some(std::sync::Arc::new(tokio::sync::Mutex::new(gm)))
            }
            Err(e) => {
                warn!(
                    error = ?e,
                    "Gateway configured but failed to initialize - gateway features disabled"
                );
                None
            }
        },
        None => {
            info!("Gateway not configured - VMs will not get public subdomains");
            None
        }
    };

    info!(
        available_provisioners = ?provisioners.keys().collect::<Vec<_>>(),
        default = %default_provisioner_type,
        "Provisioner inventory loaded"
    );

    let poll_interval = Duration::from_secs(config.polling.interval_seconds);
    let mut poll_ticker = interval(poll_interval);

    // Start with a 60s default heartbeat interval, will be updated from server response
    let mut heartbeat_interval_secs: u64 = 60;
    let mut heartbeat_ticker = interval(Duration::from_secs(heartbeat_interval_secs));

    // Resource collection every 5 minutes (less frequent than heartbeat)
    const RESOURCE_COLLECTION_INTERVAL_SECS: u64 = 300;
    let mut resource_ticker = interval(Duration::from_secs(RESOURCE_COLLECTION_INTERVAL_SECS));
    resource_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Cached resources (collected periodically, sent with each heartbeat)
    // Initialized below before first heartbeat
    let mut cached_resources: Option<crate::api_client::ResourceInventory>;

    // Track active contracts for heartbeat reporting
    let mut active_contracts: i64 = 0;

    // Load orphan tracker from disk (persists across restarts)
    let orphan_tracker_path = Path::new(&config.polling.orphan_tracker_path);
    let mut orphan_tracker = OrphanTracker::load(orphan_tracker_path).with_context(|| {
        format!(
            "Failed to load orphan tracker from {:?}",
            orphan_tracker_path
        )
    })?;
    info!(
        path = %config.polling.orphan_tracker_path,
        tracked_orphans = orphan_tracker.first_seen.len(),
        "Orphan tracker loaded"
    );

    // Track consecutive failures for escalating log levels
    let mut heartbeat_failures: u32 = 0;
    let mut poll_failures: u32 = 0;

    // Update check every hour (3600 seconds)
    const UPDATE_CHECK_INTERVAL_SECS: u64 = 3600;
    let mut update_check_ticker = interval(Duration::from_secs(UPDATE_CHECK_INTERVAL_SECS));
    update_check_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Health check ticker - runs health checks on all provisioned instances
    let health_check_interval_secs = config.polling.health_check_interval_seconds;
    let mut health_check_ticker = interval(Duration::from_secs(health_check_interval_secs));
    health_check_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut health_check_failures: u32 = 0;

    info!(
        poll_interval_seconds = config.polling.interval_seconds,
        heartbeat_interval_seconds = heartbeat_interval_secs,
        health_check_interval_seconds = health_check_interval_secs,
        orphan_grace_period_seconds = config.polling.orphan_grace_period_seconds,
        update_check_interval_seconds = UPDATE_CHECK_INTERVAL_SECS,
        "Agent started"
    );

    // Collect initial resources
    cached_resources = default_provisioner.collect_resources().await;
    if cached_resources.is_some() {
        info!("Collected initial resource inventory");
    }

    // Send initial heartbeat immediately
    send_heartbeat(
        &api_client,
        &default_provisioner_type,
        active_contracts,
        &mut heartbeat_interval_secs,
        &mut heartbeat_ticker,
        &mut heartbeat_failures,
        gateway_manager.clone(),
        cached_resources.clone(),
    )
    .await;

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                active_contracts = reconcile::poll_and_provision(&api_client, &provisioners, &default_provisioner_type, config.polling.orphan_grace_period_seconds, &mut orphan_tracker, &mut poll_failures, gateway_manager.clone()).await;
            }
            _ = heartbeat_ticker.tick() => {
                send_heartbeat(&api_client, &default_provisioner_type, active_contracts, &mut heartbeat_interval_secs, &mut heartbeat_ticker, &mut heartbeat_failures, gateway_manager.clone(), cached_resources.clone()).await;
            }
            _ = resource_ticker.tick() => {
                // Refresh resource inventory periodically
                if let Some(resources) = default_provisioner.collect_resources().await {
                    cached_resources = Some(resources);
                    info!("Refreshed resource inventory");
                }
            }
            _ = update_check_ticker.tick() => {
                check_for_updates_and_log().await;
            }
            _ = health_check_ticker.tick() => {
                run_health_checks(&api_client, &provisioners, &mut health_check_failures).await;
            }
        }
    }
}

/// Run health checks on all running instances and report results to the API.
///
/// Lists running instances from all provisioners, checks each one's health,
/// and reports the result to the central API for uptime tracking.
async fn run_health_checks(
    api_client: &ApiClient,
    provisioners: &ProvisionerMap,
    consecutive_failures: &mut u32,
) {
    // Collect running instances from all provisioners
    let mut instances: Vec<(String, String)> = Vec::new(); // (contract_id, external_id)
    for (ptype, provisioner) in provisioners {
        match provisioner.list_running_instances().await {
            Ok(running) => {
                for inst in running {
                    if let Some(contract_id) = inst.contract_id {
                        instances.push((contract_id, inst.external_id));
                    }
                }
            }
            Err(e) => {
                warn!(
                    provisioner_type = %ptype,
                    error = %e,
                    "Failed to list running instances for health checks"
                );
            }
        }
    }

    if instances.is_empty() {
        return;
    }

    let mut checked = 0u32;
    let mut failed = 0u32;
    for (contract_id, external_id) in &instances {
        // Find the provisioner that owns this instance (try each)
        let mut health_status = None;
        for provisioner in provisioners.values() {
            match provisioner.health_check(external_id).await {
                Ok(status) => {
                    health_status = Some(status);
                    break;
                }
                Err(_) => continue,
            }
        }

        let status = health_status.unwrap_or(crate::provisioner::HealthStatus::Unknown);

        if let Err(e) = api_client.report_health(contract_id, &status).await {
            failed += 1;
            if *consecutive_failures < 3 {
                warn!(
                    contract_id = %contract_id,
                    error = %e,
                    "Failed to report health check"
                );
            }
        } else {
            checked += 1;
        }
    }

    if failed > 0 {
        *consecutive_failures += 1;
        if *consecutive_failures == 3 {
            warn!("Suppressing repeated health check report failures (3+ consecutive)");
        }
    } else {
        if *consecutive_failures >= 3 {
            info!(
                previous_failures = *consecutive_failures,
                "Health check reporting restored"
            );
        }
        *consecutive_failures = 0;
    }

    if checked > 0 {
        info!(checked, failed, "Health checks completed");
    }
}

/// Check for available updates and log if a new version is available.
/// Runs periodically during agent operation to notify about updates.
async fn check_for_updates_and_log() {
    use tracing::debug;

    let current = env!("CARGO_PKG_VERSION");
    match crate::upgrade::check_latest_version().await {
        Ok(latest) if crate::upgrade::is_newer(current, &latest) => {
            warn!(
                current_version = %current,
                latest_version = %latest,
                "Update available! Run 'dc-agent upgrade' to install"
            );
        }
        Ok(_) => {
            debug!("Version check: up to date");
        }
        Err(e) => {
            // Don't spam logs on network issues - use debug level
            debug!(error = %e, "Failed to check for updates");
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_heartbeat(
    api_client: &ApiClient,
    provisioner_type: &str,
    active_contracts: i64,
    heartbeat_interval_secs: &mut u64,
    heartbeat_ticker: &mut tokio::time::Interval,
    consecutive_failures: &mut u32,
    gateway_manager: OptionalGatewayManager,
    resources: Option<crate::api_client::ResourceInventory>,
) {
    // Collect bandwidth stats from gateway manager if available
    let bandwidth_stats = if let Some(ref gw) = gateway_manager {
        let gw_lock = gw.lock().await;
        let stats = gw_lock.get_bandwidth_stats();
        let allocations = gw_lock.port_allocations();

        if stats.is_empty() {
            None
        } else {
            // Map slug -> contract_id from allocations
            let reports: Vec<_> = stats
                .into_iter()
                .filter_map(|(slug, bw)| {
                    allocations.allocations.get(&slug).map(|alloc| {
                        crate::api_client::VmBandwidthReport {
                            gateway_slug: slug,
                            contract_id: alloc.contract_id.clone(),
                            bytes_in: bw.bytes_in,
                            bytes_out: bw.bytes_out,
                        }
                    })
                })
                .collect();

            if reports.is_empty() {
                None
            } else {
                Some(reports)
            }
        }
    } else {
        None
    };

    match api_client
        .send_heartbeat(
            Some(env!("CARGO_PKG_VERSION")),
            Some(provisioner_type),
            None,
            active_contracts,
            bandwidth_stats,
            resources,
        )
        .await
    {
        Ok(response) => {
            if *consecutive_failures > 0 {
                info!(
                    previous_failures = *consecutive_failures,
                    "Heartbeat connection restored"
                );
                *consecutive_failures = 0;
            }
            info!(
                active_contracts = active_contracts,
                next_heartbeat_seconds = response.next_heartbeat_seconds,
                "Heartbeat sent"
            );
            // Update heartbeat interval if server suggests a different one
            let suggested = response.next_heartbeat_seconds as u64;
            if suggested > 0 && suggested != *heartbeat_interval_secs {
                *heartbeat_interval_secs = suggested;
                *heartbeat_ticker = interval(Duration::from_secs(suggested));
                info!(interval_seconds = suggested, "Heartbeat interval updated");
            }
            // Check for remote upgrade directive
            if let Some(ref target_version) = response.upgrade_to_version {
                let current = env!("CARGO_PKG_VERSION");
                if crate::upgrade::is_newer(current, target_version) {
                    info!(
                        current = current,
                        target = %target_version,
                        "Remote upgrade requested, starting self-upgrade"
                    );
                    // Run upgrade in background — skip_confirm=true, force=false
                    let version = target_version.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            crate::upgrade::run_upgrade(false, true, false, Some(&version)).await
                        {
                            error!(error = %e, "Remote upgrade failed");
                        }
                    });
                }
            }
        }
        Err(e) => {
            *consecutive_failures += 1;
            // Escalate to error level after 3 consecutive failures
            if *consecutive_failures >= 3 {
                error!(
                    error = ?e,
                    consecutive_failures = *consecutive_failures,
                    "HEARTBEAT FAILURE: Agent cannot reach API server! Check network connectivity."
                );
            } else {
                warn!(
                    error = ?e,
                    consecutive_failures = *consecutive_failures,
                    "Failed to send heartbeat"
                );
            }
        }
    }
}
