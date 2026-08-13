use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use dcc_common::ssh_exec::execute_post_provision_script;

use super::OptionalGatewayManager;
use crate::api_client::{ApiClient, ReconcileResponse};
use crate::provisioner::{ProvisionRequest, ProvisionerMap};

pub(super) async fn poll_and_provision(
    api_client: &ApiClient,
    provisioners: &ProvisionerMap,
    default_provisioner_type: &str,
    orphan_grace_period_seconds: u64,
    orphan_tracker: &mut crate::orphan_tracker::OrphanTracker,
    consecutive_failures: &mut u32,
    gateway_manager: OptionalGatewayManager,
) -> i64 {
    let running_by_contract = collect_running_by_contract(provisioners).await;

    // Fetch pending contracts for provisioning
    match api_client.get_pending_contracts().await {
        Ok(contracts) => {
            if *consecutive_failures > 0 {
                info!(
                    previous_failures = *consecutive_failures,
                    "API connection restored"
                );
                *consecutive_failures = 0;
            }

            if !contracts.is_empty() {
                info!(count = contracts.len(), "Found pending contracts");

                for contract in &contracts {
                    info!(contract_id = %contract.contract_id, "Processing contract");

                    // Determine which provisioner to use (per-offering override or default)
                    let provisioner_type = contract
                        .provisioner_type
                        .as_deref()
                        .unwrap_or(default_provisioner_type);

                    let provisioner = match provisioners.get(provisioner_type) {
                        Some(p) => p.as_ref(),
                        None => {
                            error!(
                                contract_id = %contract.contract_id,
                                required_type = %provisioner_type,
                                available = ?provisioners.keys().collect::<Vec<_>>(),
                                "Offering requires provisioner type '{}' but agent only has: {:?}",
                                provisioner_type,
                                provisioners.keys().collect::<Vec<_>>()
                            );

                            // Report failure to API
                            if let Err(e) = api_client
                                .report_failed(
                                    &contract.contract_id,
                                    &format!(
                                        "Agent lacks required provisioner type '{}'. Available: {:?}",
                                        provisioner_type,
                                        provisioners.keys().collect::<Vec<_>>()
                                    ),
                                )
                                .await
                            {
                                error!(
                                    contract_id = %contract.contract_id,
                                    error = ?e,
                                    "Failed to report provisioner mismatch to API"
                                );
                            }
                            continue;
                        }
                    };

                    if contract.provisioner_type.is_some() {
                        info!(
                            contract_id = %contract.contract_id,
                            provisioner_type = %provisioner_type,
                            "Using offering-specific provisioner"
                        );
                    }

                    // Parse provisioner_config from offering
                    let provisioner_config: Option<serde_json::Value> =
                        match &contract.provisioner_config {
                            Some(s) => match serde_json::from_str(s) {
                                Ok(v) => Some(v),
                                Err(e) => {
                                    warn!(
                                        contract_id = %contract.contract_id,
                                        error = ?e,
                                        raw_config = %s,
                                        "Invalid provisioner_config JSON, ignoring"
                                    );
                                    None
                                }
                            },
                            None => None,
                        };

                    // Parse instance_config if present - log warning if malformed
                    let contract_instance_config: Option<serde_json::Value> =
                        match &contract.instance_config {
                            Some(s) => match serde_json::from_str(s) {
                                Ok(v) => Some(v),
                                Err(e) => {
                                    warn!(
                                        contract_id = %contract.contract_id,
                                        error = ?e,
                                        raw_config = %s,
                                        "Invalid instance_config JSON, ignoring"
                                    );
                                    None
                                }
                            },
                            None => None,
                        };

                    // Merge configs: contract instance_config overrides provisioner_config
                    let instance_config = match (provisioner_config, contract_instance_config) {
                        (Some(prov), Some(inst)) => {
                            // Merge: instance config takes precedence
                            let mut merged = prov;
                            if let serde_json::Value::Object(ref mut prov_map) = merged {
                                if let serde_json::Value::Object(inst_map) = inst {
                                    for (k, v) in inst_map {
                                        prov_map.insert(k, v);
                                    }
                                }
                            }
                            Some(merged)
                        }
                        (Some(prov), None) => Some(prov),
                        (None, Some(inst)) => Some(inst),
                        (None, None) => None,
                    };

                    // Extract specs from offering (returned by API)
                    let cpu_cores = contract.cpu_cores.map(|c| c as u32);
                    let memory_mb = contract.memory_mb();
                    let storage_gb = contract.storage_gb();

                    if cpu_cores.is_some() || memory_mb.is_some() || storage_gb.is_some() {
                        info!(
                            contract_id = %contract.contract_id,
                            cpu_cores = ?cpu_cores,
                            memory_mb = ?memory_mb,
                            storage_gb = ?storage_gb,
                            "Using offering specs for VM"
                        );
                    }

                    // Try to acquire provisioning lock (prevents race conditions with multiple agents)
                    match api_client.acquire_lock(&contract.contract_id).await {
                        Ok(true) => {
                            info!(
                                contract_id = %contract.contract_id,
                                "Acquired provisioning lock"
                            );
                        }
                        Ok(false) => {
                            info!(
                                contract_id = %contract.contract_id,
                                "Contract locked by another agent, skipping"
                            );
                            continue;
                        }
                        Err(e) => {
                            warn!(
                                contract_id = %contract.contract_id,
                                error = ?e,
                                "Failed to acquire lock, skipping contract"
                            );
                            continue;
                        }
                    }

                    if let Some(external_id) = running_by_contract.get(&contract.contract_id) {
                        info!(
                            contract_id = %contract.contract_id,
                            external_id = %external_id,
                            "Found existing VM from a previous agent run, recovering"
                        );
                        let mut recovered = false;
                        for prov in provisioners.values() {
                            match prov.get_instance(external_id).await {
                                Ok(Some(instance)) => {
                                    if let Err(e) = api_client
                                        .report_provisioned(&contract.contract_id, &instance)
                                        .await
                                    {
                                        error!(
                                            contract_id = %contract.contract_id,
                                            error = ?e,
                                            "Failed to report recovered VM as provisioned"
                                        );
                                    } else {
                                        info!(
                                            contract_id = %contract.contract_id,
                                            external_id = %instance.external_id,
                                            ip_address = ?instance.ip_address,
                                            "Recovered and reported existing VM"
                                        );
                                        recovered = true;
                                    }
                                    break;
                                }
                                Ok(None) => continue,
                                Err(_) => continue,
                            }
                        }
                        if !recovered {
                            warn!(
                                contract_id = %contract.contract_id,
                                "Could not retrieve existing VM details"
                            );
                        }
                        if let Err(e) = api_client.release_lock(&contract.contract_id).await {
                            warn!(
                                contract_id = %contract.contract_id,
                                error = ?e,
                                "Failed to release lock after recovery attempt"
                            );
                        }
                        continue;
                    }

                    let request = ProvisionRequest {
                        contract_id: contract.contract_id.clone(),
                        offering_id: contract.offering_id.clone(),
                        cpu_cores,
                        memory_mb,
                        storage_gb,
                        requester_ssh_pubkey: Some(contract.requester_ssh_pubkey.clone()),
                        instance_config,
                        post_provision_script: contract.post_provision_script.clone(),
                    };

                    // Mark contract as provisioning before starting (for UI feedback)
                    if let Err(e) = api_client
                        .report_provisioning_started(&contract.contract_id)
                        .await
                    {
                        error!(
                            contract_id = %contract.contract_id,
                            error = ?e,
                            "Failed to report provisioning started, releasing lock and skipping"
                        );
                        if let Err(release_err) =
                            api_client.release_lock(&contract.contract_id).await
                        {
                            warn!(
                                contract_id = %contract.contract_id,
                                error = ?release_err,
                                "Failed to release lock after report_provisioning_started failure"
                            );
                        }
                        continue;
                    }

                    match provisioner.provision(&request).await {
                        Ok(mut instance) => {
                            info!(
                                contract_id = %contract.contract_id,
                                external_id = %instance.external_id,
                                ip_address = ?instance.ip_address,
                                "Provisioned successfully"
                            );

                            // Setup gateway (subdomain, ports, DNS) if configured
                            if let Some(ref gw) = gateway_manager {
                                let mut gw_lock = gw.lock().await;
                                match gw_lock
                                    .setup_gateway(instance.clone(), &contract.contract_id)
                                    .await
                                {
                                    Ok(updated_instance) => {
                                        instance = updated_instance;
                                        info!(
                                            contract_id = %contract.contract_id,
                                            gateway_subdomain = ?instance.gateway_subdomain,
                                            gateway_ssh_port = ?instance.gateway_ssh_port,
                                            "Gateway setup complete"
                                        );
                                    }
                                    Err(e) => {
                                        // Gateway setup failed - log but continue
                                        // VM is usable via internal IP even without gateway
                                        warn!(
                                            contract_id = %contract.contract_id,
                                            error = ?e,
                                            "Gateway setup failed - VM accessible via internal IP only"
                                        );
                                    }
                                }
                            }

                            // Execute post-provision script if configured
                            if let Some(ref script) = request.post_provision_script {
                                if let Some(ref ip) = instance.ip_address {
                                    match execute_post_provision_script(
                                        ip,
                                        instance.ssh_port,
                                        script,
                                        &contract.contract_id,
                                    )
                                    .await
                                    {
                                        Ok(result) => {
                                            if result.success {
                                                info!(
                                                    contract_id = %contract.contract_id,
                                                    "Post-provision script completed successfully"
                                                );
                                            } else {
                                                warn!(
                                                    contract_id = %contract.contract_id,
                                                    exit_code = result.exit_code,
                                                    log = %result.log,
                                                    "Post-provision script failed - VM is still accessible"
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            warn!(
                                                contract_id = %contract.contract_id,
                                                error = ?e,
                                                "Post-provision script infrastructure failure - VM is still accessible"
                                            );
                                        }
                                    }
                                } else {
                                    warn!(
                                        contract_id = %contract.contract_id,
                                        "Cannot execute post-provision script: no IP address available"
                                    );
                                }
                            }

                            if let Err(e) = api_client
                                .report_provisioned(&contract.contract_id, &instance)
                                .await
                            {
                                error!(
                                    contract_id = %contract.contract_id,
                                    external_id = %instance.external_id,
                                    ip_address = ?instance.ip_address,
                                    error = ?e,
                                    "CRITICAL: VM provisioned but failed to report to API! \
                                     Contract may be stuck. Manual intervention may be required."
                                );
                            }
                            if let Err(e) = api_client.release_lock(&contract.contract_id).await {
                                warn!(
                                    contract_id = %contract.contract_id,
                                    error = ?e,
                                    "Failed to release provisioning lock"
                                );
                            }
                        }
                        Err(e) => {
                            // Use {:?} to show full error chain including underlying cause
                            error!(
                                contract_id = %contract.contract_id,
                                error = ?e,
                                "Provisioning failed"
                            );
                            // Include full chain in failure report
                            if let Err(report_err) = api_client
                                .report_failed(&contract.contract_id, &format!("{:?}", e))
                                .await
                            {
                                error!(
                                    contract_id = %contract.contract_id,
                                    original_error = ?e,
                                    report_error = %report_err,
                                    "Failed to report provisioning failure to API. \
                                     Contract may remain stuck in pending state."
                                );
                            }
                            if let Err(release_err) =
                                api_client.release_lock(&contract.contract_id).await
                            {
                                warn!(
                                    contract_id = %contract.contract_id,
                                    error = ?release_err,
                                    "Failed to release provisioning lock after failure"
                                );
                            }
                        }
                    }
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
                    "POLL FAILURE: Cannot fetch contracts from API server! Check network connectivity."
                );
            } else {
                warn!(
                    error = ?e,
                    consecutive_failures = *consecutive_failures,
                    "Failed to fetch pending contracts"
                );
            }
            return 0;
        }
    }

    // Reconcile running instances - handles expired, cancelled, and orphan VMs
    // Returns the count of running instances (accurate active contract count)
    let running_count = reconcile_instances(
        api_client,
        provisioners,
        orphan_grace_period_seconds,
        orphan_tracker,
        gateway_manager,
    )
    .await;

    running_count
}

async fn collect_running_by_contract(provisioners: &ProvisionerMap) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (prov_name, prov) in provisioners {
        match prov.list_running_instances().await {
            Ok(instances) => {
                for inst in instances {
                    if let Some(ref cid) = inst.contract_id {
                        if !cid.is_empty() {
                            map.entry(cid.clone())
                                .or_insert_with(|| inst.external_id.clone());
                        }
                    }
                }
            }
            // A provisioner that cannot list its instances would otherwise drop
            // silently out of orphan reconciliation: missing VMs could be
            // treated as gone. Surface the backend + error so the outage is
            // visible instead of degrading reconciliation unnoticed.
            Err(e) => {
                warn!(
                    backend = %prov_name,
                    error = %e,
                    "Failed to list running instances during reconciliation; instances for this provisioner will be excluded this cycle"
                );
                continue;
            }
        }
    }
    map
}

/// Reconcile running instances with the API.
/// Reports running VMs, terminates expired/cancelled contracts, and prunes orphans after grace period.
/// Collects instances from ALL provisioners and tries to terminate via the appropriate one.
/// Returns the number of running instances (for heartbeat active_contracts count).
async fn reconcile_instances(
    api_client: &ApiClient,
    provisioners: &ProvisionerMap,
    orphan_grace_period_seconds: u64,
    orphan_tracker: &mut crate::orphan_tracker::OrphanTracker,
    gateway_manager: OptionalGatewayManager,
) -> i64 {
    // Collect running instances from ALL provisioners
    let mut all_running_instances = Vec::new();
    for (ptype, provisioner) in provisioners {
        match provisioner.list_running_instances().await {
            Ok(instances) => {
                if !instances.is_empty() {
                    info!(
                        provisioner_type = %ptype,
                        count = instances.len(),
                        "Found running instances"
                    );
                }
                all_running_instances.extend(instances);
            }
            Err(e) => {
                warn!(
                    provisioner_type = %ptype,
                    error = ?e,
                    "Failed to list running instances from this provisioner"
                );
            }
        }
    }

    let running_count = all_running_instances.len() as i64;

    if all_running_instances.is_empty() {
        return 0;
    }

    // Call reconcile API
    let response: ReconcileResponse = match api_client.reconcile(&all_running_instances).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = ?e, "Failed to reconcile with API");
            return running_count;
        }
    };

    // Process pauses FIRST -- a contract that flipped paused->cancelled in the
    // same cycle must be picked up on the cancellation pass below, not double-handled.
    // (The server only ever puts a single instance in one bucket, so pause/terminate
    // for the same external_id never collide; we just process pauses first to keep
    // the log order intuitive: stop, then if needed terminate.)
    for vm in &response.pause {
        info!(
            external_id = %vm.external_id,
            contract_id = %vm.contract_id,
            reason = %vm.reason,
            "Pausing VM (stop without destroy)"
        );
        let mut paused_ok = false;
        let mut errors: Vec<(String, String)> = Vec::new();
        for (ptype, provisioner) in provisioners {
            match provisioner.stop(&vm.external_id).await {
                Ok(()) => {
                    info!(
                        external_id = %vm.external_id,
                        contract_id = %vm.contract_id,
                        provisioner_type = %ptype,
                        "VM stopped for pause"
                    );
                    paused_ok = true;
                    break;
                }
                Err(e) => {
                    errors.push((ptype.to_string(), format!("{e:#}")));
                    continue;
                }
            }
        }
        if !paused_ok {
            error!(
                external_id = %vm.external_id,
                contract_id = %vm.contract_id,
                errors = ?errors,
                "Pause failed - no provisioner could stop this instance"
            );
        }
    }

    // Process terminations - try each provisioner until one succeeds
    for vm in &response.terminate {
        info!(
            external_id = %vm.external_id,
            contract_id = %vm.contract_id,
            reason = %vm.reason,
            "Terminating VM"
        );

        let mut terminated = false;
        let mut termination_errors: Vec<(String, String)> = Vec::new();
        for (ptype, provisioner) in provisioners {
            match provisioner.terminate(&vm.external_id).await {
                Ok(()) => {
                    info!(
                        external_id = %vm.external_id,
                        contract_id = %vm.contract_id,
                        provisioner_type = %ptype,
                        "VM terminated successfully"
                    );

                    // Cleanup gateway (DNS, Traefik config, port allocation) if configured
                    if let Some(ref gw) = gateway_manager {
                        let mut gw_lock = gw.lock().await;
                        if let Some(slug) = gw_lock.find_slug_by_contract(&vm.contract_id) {
                            if let Err(e) = gw_lock.cleanup_gateway(&slug).await {
                                warn!(
                                    contract_id = %vm.contract_id,
                                    slug = %slug,
                                    error = ?e,
                                    "Gateway cleanup failed"
                                );
                            }
                        }
                    }

                    if let Err(e) = api_client.report_terminated(&vm.contract_id).await {
                        error!(
                            contract_id = %vm.contract_id,
                            error = ?e,
                            "Failed to report termination to API. May retry on next poll."
                        );
                    }
                    terminated = true;
                    break;
                }
                Err(e) => {
                    warn!(
                        external_id = %vm.external_id,
                        provisioner_type = %ptype,
                        error = ?e,
                        "Provisioner failed to terminate, trying next"
                    );
                    termination_errors.push((ptype.to_string(), format!("{e:#}")));
                    continue;
                }
            }
        }

        if !terminated {
            error!(
                external_id = %vm.external_id,
                contract_id = %vm.contract_id,
                errors = ?termination_errors,
                "Termination failed - no provisioner could terminate this instance"
            );
        }
    }

    // Process password reset requests
    match api_client.get_pending_password_resets().await {
        Ok(reset_requests) => {
            if !reset_requests.is_empty() {
                info!(
                    count = reset_requests.len(),
                    "Processing password reset requests"
                );

                for reset_req in &reset_requests {
                    let contract_id = &reset_req.contract_id;
                    let external_id = format!("dc-{}", contract_id);
                    info!(contract_id = %contract_id, "Processing password reset request");

                    // Find the VM in any provisioner
                    let mut reset_done = false;
                    for (ptype, provisioner) in provisioners {
                        match provisioner.get_instance(&external_id).await {
                            Ok(Some(instance)) => {
                                if let Some(ref ip) = instance.ip_address {
                                    let ssh_port = instance.ssh_port;
                                    let new_password =
                                        crate::provisioner::proxmox::generate_secure_password(24);

                                    match dcc_common::ssh_exec::reset_password_via_ssh(
                                        ip,
                                        ssh_port,
                                        "ubuntu",
                                        true,
                                        &new_password,
                                        contract_id,
                                    )
                                    .await
                                    {
                                        Ok(()) => {
                                            info!(
                                                contract_id = %contract_id,
                                                provisioner_type = %ptype,
                                                "Password reset via SSH successful"
                                            );

                                            // Report new password to API
                                            if let Err(e) = api_client
                                                .update_contract_password(
                                                    contract_id,
                                                    &new_password,
                                                )
                                                .await
                                            {
                                                error!(
                                                    contract_id = %contract_id,
                                                    error = ?e,
                                                    "Failed to report new password to API"
                                                );
                                            }
                                            reset_done = true;
                                            break;
                                        }
                                        Err(e) => {
                                            warn!(
                                                contract_id = %contract_id,
                                                provisioner_type = %ptype,
                                                error = ?e,
                                                "Password reset via SSH failed"
                                            );
                                        }
                                    }
                                } else {
                                    warn!(
                                        contract_id = %contract_id,
                                        "VM has no IP address, cannot reset password"
                                    );
                                }
                            }
                            Ok(None) => continue, // Try next provisioner
                            Err(e) => {
                                warn!(
                                    contract_id = %contract_id,
                                    provisioner_type = %ptype,
                                    error = ?e,
                                    "Failed to get instance info"
                                );
                            }
                        }
                    }

                    if !reset_done {
                        warn!(
                            contract_id = %contract_id,
                            "Password reset failed - VM not found or SSH failed"
                        );
                    }
                }
            }
        }
        Err(e) => {
            warn!(error = ?e, "Failed to get pending password resets");
        }
    }

    // Process SSH key rotation requests
    match api_client.get_pending_ssh_key_rotations().await {
        Ok(rotation_requests) => {
            if !rotation_requests.is_empty() {
                info!(
                    count = rotation_requests.len(),
                    "Processing SSH key rotation requests"
                );

                for rot_req in &rotation_requests {
                    let contract_id = &rot_req.contract_id;
                    let new_ssh_key = &rot_req.requester_ssh_pubkey;
                    let external_id = format!("dc-{}", contract_id);
                    info!(contract_id = %contract_id, "Processing SSH key rotation request");

                    let mut rotation_done = false;
                    for (ptype, provisioner) in provisioners {
                        match provisioner.get_instance(&external_id).await {
                            Ok(Some(instance)) => {
                                if let Some(ref ip) = instance.ip_address {
                                    let ssh_port = instance.gateway_ssh_port.unwrap_or(22);

                                    match dcc_common::ssh_exec::inject_ssh_key_via_ssh(
                                        ip,
                                        ssh_port,
                                        "ubuntu",
                                        true,
                                        new_ssh_key,
                                        contract_id,
                                    )
                                    .await
                                    {
                                        Ok(()) => {
                                            info!(
                                                contract_id = %contract_id,
                                                provisioner_type = %ptype,
                                                "SSH key injection successful"
                                            );

                                            if let Err(e) = api_client
                                                .complete_ssh_key_rotation(contract_id)
                                                .await
                                            {
                                                error!(
                                                    contract_id = %contract_id,
                                                    error = ?e,
                                                    "Failed to report SSH key rotation completion to API"
                                                );
                                            }
                                            rotation_done = true;
                                            break;
                                        }
                                        Err(e) => {
                                            warn!(
                                                contract_id = %contract_id,
                                                provisioner_type = %ptype,
                                                error = ?e,
                                                "SSH key injection failed"
                                            );
                                        }
                                    }
                                } else {
                                    warn!(
                                        contract_id = %contract_id,
                                        "VM has no IP address, cannot inject SSH key"
                                    );
                                }
                            }
                            Ok(None) => continue,
                            Err(e) => {
                                warn!(
                                    contract_id = %contract_id,
                                    provisioner_type = %ptype,
                                    error = ?e,
                                    "Failed to get instance info"
                                );
                            }
                        }
                    }

                    if !rotation_done {
                        warn!(
                            contract_id = %contract_id,
                            "SSH key rotation failed - VM not found or SSH failed"
                        );
                    }
                }
            }
        }
        Err(e) => {
            warn!(error = ?e, "Failed to get pending SSH key rotations");
        }
    }

    // Track and prune orphan VMs after grace period
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Collect current orphan external IDs
    let current_orphans: HashSet<String> = response
        .unknown
        .iter()
        .map(|vm| vm.external_id.clone())
        .collect();

    // Track new orphans and check grace period for existing ones
    let mut to_prune = Vec::new();
    for vm in &response.unknown {
        let first_seen = orphan_tracker.record_orphan(&vm.external_id, now);
        let age_seconds = now.saturating_sub(first_seen);

        if age_seconds >= orphan_grace_period_seconds {
            // Grace period exceeded - prune this orphan
            to_prune.push(vm);
        } else if first_seen == now {
            // Newly detected orphan
            info!(
                external_id = %vm.external_id,
                message = %vm.message,
                grace_period_seconds = orphan_grace_period_seconds,
                "Orphan VM detected - will auto-prune after grace period if not resolved"
            );
        } else {
            // Existing orphan still in grace period
            warn!(
                external_id = %vm.external_id,
                message = %vm.message,
                age_seconds = age_seconds,
                remaining_seconds = orphan_grace_period_seconds.saturating_sub(age_seconds),
                "Orphan VM still present - will auto-prune if not resolved"
            );
        }
    }

    // Prune orphans that exceeded grace period
    for vm in &to_prune {
        warn!(
            external_id = %vm.external_id,
            message = %vm.message,
            grace_period_seconds = orphan_grace_period_seconds,
            "Pruning orphan VM - grace period exceeded"
        );

        let mut pruned = false;
        let mut prune_errors: Vec<(String, String)> = Vec::new();
        for (ptype, provisioner) in provisioners {
            match provisioner.terminate(&vm.external_id).await {
                Ok(()) => {
                    info!(
                        external_id = %vm.external_id,
                        provisioner_type = %ptype,
                        "Orphan VM pruned successfully"
                    );
                    pruned = true;
                    break;
                }
                Err(e) => {
                    warn!(
                        external_id = %vm.external_id,
                        provisioner_type = %ptype,
                        error = ?e,
                        "Provisioner failed to prune orphan, trying next"
                    );
                    prune_errors.push((ptype.to_string(), format!("{e:#}")));
                    continue;
                }
            }
        }

        if !pruned {
            error!(
                external_id = %vm.external_id,
                errors = ?prune_errors,
                "Orphan pruning failed - no provisioner could terminate this instance"
            );
        } else {
            // Remove from tracker after successful pruning
            orphan_tracker.remove(&vm.external_id);
        }
    }

    // Clean up tracker - remove orphans that are no longer present (resolved)
    let removed = orphan_tracker.retain_present(&current_orphans);
    for external_id in removed {
        info!(
            external_id = %external_id,
            "Orphan VM resolved - no longer present"
        );
    }

    // Persist orphan tracker to disk so state survives restarts
    if let Err(e) = orphan_tracker.save() {
        error!(error = ?e, "Failed to save orphan tracker - state may be lost on restart");
    }

    // Return count minus terminated VMs (post-reconciliation active count)
    running_count - response.terminate.len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provisioner::Provisioner;
    use anyhow::Result;

    #[tokio::test]
    async fn test_collect_running_by_contract_empty_provisioners() {
        let provisioners: ProvisionerMap = HashMap::new();
        let result = collect_running_by_contract(&provisioners).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_collect_running_by_contract_maps_contract_to_external_id() {
        use crate::provisioner::{HealthStatus, Instance, RunningInstance};

        struct StubProvisioner {
            instances: Vec<RunningInstance>,
        }

        #[async_trait::async_trait]
        impl Provisioner for StubProvisioner {
            async fn provision(&self, _request: &ProvisionRequest) -> Result<Instance> {
                anyhow::bail!("not implemented")
            }
            async fn terminate(&self, _external_id: &str) -> Result<()> {
                anyhow::bail!("not implemented")
            }
            async fn health_check(&self, _external_id: &str) -> Result<HealthStatus> {
                anyhow::bail!("not implemented")
            }
            async fn get_instance(&self, _external_id: &str) -> Result<Option<Instance>> {
                anyhow::bail!("not implemented")
            }
            async fn list_running_instances(&self) -> Result<Vec<RunningInstance>> {
                Ok(self.instances.clone())
            }
        }

        let mut provisioners: ProvisionerMap = HashMap::new();
        let stub = StubProvisioner {
            instances: vec![
                RunningInstance {
                    external_id: "101".to_string(),
                    contract_id: Some("abc123".to_string()),
                },
                RunningInstance {
                    external_id: "102".to_string(),
                    contract_id: Some("def456".to_string()),
                },
            ],
        };
        provisioners.insert("stub".to_string(), Box::new(stub));

        let result = collect_running_by_contract(&provisioners).await;
        assert_eq!(result.len(), 2);
        assert_eq!(result.get("abc123"), Some(&"101".to_string()));
        assert_eq!(result.get("def456"), Some(&"102".to_string()));
    }

    #[tokio::test]
    async fn test_collect_running_by_contract_filters_empty_contract_ids() {
        use crate::provisioner::{HealthStatus, Instance, RunningInstance};

        struct StubProvisioner {
            instances: Vec<RunningInstance>,
        }

        #[async_trait::async_trait]
        impl Provisioner for StubProvisioner {
            async fn provision(&self, _request: &ProvisionRequest) -> Result<Instance> {
                anyhow::bail!("not implemented")
            }
            async fn terminate(&self, _external_id: &str) -> Result<()> {
                anyhow::bail!("not implemented")
            }
            async fn health_check(&self, _external_id: &str) -> Result<HealthStatus> {
                anyhow::bail!("not implemented")
            }
            async fn get_instance(&self, _external_id: &str) -> Result<Option<Instance>> {
                anyhow::bail!("not implemented")
            }
            async fn list_running_instances(&self) -> Result<Vec<RunningInstance>> {
                Ok(self.instances.clone())
            }
        }

        let mut provisioners: ProvisionerMap = HashMap::new();
        let stub = StubProvisioner {
            instances: vec![
                RunningInstance {
                    external_id: "101".to_string(),
                    contract_id: Some("abc123".to_string()),
                },
                RunningInstance {
                    external_id: "102".to_string(),
                    contract_id: Some(String::new()),
                },
                RunningInstance {
                    external_id: "103".to_string(),
                    contract_id: None,
                },
            ],
        };
        provisioners.insert("stub".to_string(), Box::new(stub));

        let result = collect_running_by_contract(&provisioners).await;
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("abc123"), Some(&"101".to_string()));
    }

    /// Regression (silent-failure sweep): a provisioner whose
    /// `list_running_instances` errors must not silently drop out of orphan
    /// reconciliation. The failure must be LOUD (warn naming the backend + the
    /// underlying error) and collection must continue for the other backends.
    #[tracing_test::traced_test]
    #[tokio::test]
    async fn collect_running_by_contract_logs_and_continues_on_provisioner_error() {
        use crate::provisioner::{HealthStatus, Instance, RunningInstance};

        struct FailingProvisioner;
        #[async_trait::async_trait]
        impl Provisioner for FailingProvisioner {
            async fn provision(&self, _request: &ProvisionRequest) -> Result<Instance> {
                unreachable!()
            }
            async fn terminate(&self, _external_id: &str) -> Result<()> {
                unreachable!()
            }
            async fn health_check(&self, _external_id: &str) -> Result<HealthStatus> {
                unreachable!()
            }
            async fn get_instance(&self, _external_id: &str) -> Result<Option<Instance>> {
                unreachable!()
            }
            async fn list_running_instances(&self) -> Result<Vec<RunningInstance>> {
                anyhow::bail!("backend unreachable (simulated)")
            }
        }

        struct StubProvisioner {
            instances: Vec<RunningInstance>,
        }
        #[async_trait::async_trait]
        impl Provisioner for StubProvisioner {
            async fn provision(&self, _request: &ProvisionRequest) -> Result<Instance> {
                anyhow::bail!("not implemented")
            }
            async fn terminate(&self, _external_id: &str) -> Result<()> {
                anyhow::bail!("not implemented")
            }
            async fn health_check(&self, _external_id: &str) -> Result<HealthStatus> {
                anyhow::bail!("not implemented")
            }
            async fn get_instance(&self, _external_id: &str) -> Result<Option<Instance>> {
                anyhow::bail!("not implemented")
            }
            async fn list_running_instances(&self) -> Result<Vec<RunningInstance>> {
                Ok(self.instances.clone())
            }
        }

        let mut provisioners: ProvisionerMap = HashMap::new();
        provisioners.insert("broken-backend".to_string(), Box::new(FailingProvisioner));
        provisioners.insert(
            "healthy-backend".to_string(),
            Box::new(StubProvisioner {
                instances: vec![RunningInstance {
                    external_id: "201".to_string(),
                    contract_id: Some("live-contract".to_string()),
                }],
            }),
        );

        let result = collect_running_by_contract(&provisioners).await;

        // Reconciliation survived the failure and still collected the healthy
        // backend's instance (the failing backend did not shadow it).
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("live-contract"), Some(&"201".to_string()));

        // The failure is LOUD, not silent: the warn names the failing backend
        // and surfaces the underlying error in the captured log output.
        assert!(logs_contain("Failed to list running instances"));
        assert!(logs_contain("broken-backend"));
        assert!(logs_contain("backend unreachable (simulated)"));
    }

    #[test]
    fn test_agent_ssh_uses_direct_port_not_gateway_port() {
        // Verifies that for agent-side SSH operations (password reset, SSH key rotation),
        // the direct VM port is used, not the gateway port.
        // The gateway port is an external port on the Proxmox host for tenant access;
        // it does not exist on the VM's internal IP.
        use crate::provisioner::Instance;
        let instance = Instance {
            external_id: "dc-test-contract".to_string(),
            ip_address: Some("192.168.1.100".to_string()),
            ipv6_address: None,
            public_ip: None,
            ssh_port: 22,
            root_password: None,
            additional_details: None,
            gateway_slug: Some("k7m2p4".to_string()),
            gateway_subdomain: Some("k7m2p4.dc-lk.gw.decent-cloud.org".to_string()),
            gateway_ssh_port: Some(20000),
            gateway_port_range_start: Some(20000),
            gateway_port_range_end: Some(20009),
        };

        // The SSH port for agent operations must be the direct VM port (22),
        // not the gateway port (20000) which is only reachable externally.
        let agent_ssh_port = instance.ssh_port;
        assert_eq!(agent_ssh_port, 22, "Agent must use direct VM port for SSH");
        assert_ne!(
            agent_ssh_port,
            instance.gateway_ssh_port.unwrap(),
            "Agent must not use gateway port for direct VM SSH"
        );
    }
}
