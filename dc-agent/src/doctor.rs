use anyhow::Result;
use tracing::warn;

use crate::api_client::ApiClient;
use crate::config::{Config, ProvisionerConfig};
use crate::gateway::GatewayManager;
use crate::host::{format_bytes, QUICK_QUERY_TIMEOUT};
use crate::provisioner::{create_provisioner_from_config, ProvisionRequest};
use crate::setup::run_command_with_timeout;

pub async fn run(config: Config, verify_api: bool, test_provision: bool) -> Result<()> {
    println!("dc-agent doctor");
    println!("================");
    println!();

    // Check configuration file
    println!("Configuration:");
    println!("  API endpoint: {}", config.api.endpoint);
    println!("  Provider pubkey: {}", config.api.provider_pubkey);
    println!("  Polling interval: {}s", config.polling.interval_seconds);
    println!(
        "  Health check interval: {}s",
        config.polling.health_check_interval_seconds
    );

    // Determine auth mode
    let auth_mode = if config.api.agent_secret_key.is_some() {
        "delegated agent key"
    } else if config.api.provider_secret_key.is_some() {
        "provider key (legacy)"
    } else {
        "no key configured"
    };
    println!("  Auth mode: {}", auth_mode);
    println!();

    // Try to create API client early (needed for gateway manager).
    // Non-fatal: doctor should still run other checks, but a failure here
    // almost always means auth misconfiguration that the operator must fix
    // before the agent can do anything useful — log it loudly.
    let api_client = match ApiClient::new(&config.api) {
        Ok(c) => Some(std::sync::Arc::new(c)),
        Err(e) => {
            warn!(
                "API client init failed — API-dependent checks will be skipped: {:#}",
                e
            );
            None
        }
    };

    // Check provisioner configuration and verify setup
    let provisioner = create_provisioner_from_config(&config.provisioner)?;

    // Show provisioner inventory
    println!("Provisioner Inventory:");
    println!("  Default: {} (required)", config.provisioner.type_name());
    if config.additional_provisioners.is_empty() {
        println!("  Additional: none");
    } else {
        for ap in &config.additional_provisioners {
            println!("  Additional: {}", ap.type_name());
        }
    }
    println!();

    match &config.provisioner {
        ProvisionerConfig::Proxmox(proxmox) => {
            println!("Provisioner: Proxmox");
            println!("  API URL: {}", proxmox.api_url);
            println!("  Node: {}", proxmox.node);
            println!("  Template VMID: {}", proxmox.template_vmid);
            println!("  Storage: {}", proxmox.storage);
            println!("  Verify SSL: {}", proxmox.verify_ssl);
            if let Some(pool) = &proxmox.pool {
                println!("  Resource pool: {}", pool);
            }
            println!();
            println!("Verifying Proxmox setup...");
            let verification = provisioner.verify_setup().await;
            if verification.api_reachable == Some(true) {
                println!("  [ok] Proxmox API reachable");
            }
            if verification.template_exists == Some(true) {
                println!("  [ok] Template VM {} exists", proxmox.template_vmid);
            }
            if verification.storage_accessible == Some(true) {
                println!("  [ok] Storage '{}' accessible", proxmox.storage);
            }
            if let Some(pool) = &proxmox.pool {
                if verification.pool_exists == Some(true) {
                    println!("  [ok] Pool '{}' exists", pool);
                }
            }
            if !verification.errors.is_empty() {
                println!();
                for error in &verification.errors {
                    println!("  [FAILED] {}", error);
                }
                return Err(anyhow::anyhow!(
                    "Proxmox setup verification failed with {} error(s)",
                    verification.errors.len()
                ));
            }
        }
        ProvisionerConfig::Script(script) => {
            println!("Provisioner: Script");
            println!("  Provision script: {}", script.provision);
            println!("  Terminate script: {}", script.terminate);
            println!("  Health check script: {}", script.health_check);
            println!("  Timeout: {}s", script.timeout_seconds);

            // Check if scripts exist
            for (name, path) in [
                ("provision", &script.provision),
                ("terminate", &script.terminate),
                ("health_check", &script.health_check),
            ] {
                if std::path::Path::new(path).exists() {
                    println!("  [ok] {} script exists", name);
                } else {
                    println!("  [MISSING] {} script: {}", name, path);
                }
            }
        }
        ProvisionerConfig::Manual(manual) => {
            println!("Provisioner: Manual");
            if let Some(webhook) = &manual.notification_webhook {
                println!("  Notification webhook: {}", webhook);
            } else {
                println!("  No notification webhook configured");
            }
        }
        ProvisionerConfig::Docker(docker) => {
            println!("Provisioner: Docker");
            println!("  Socket: {}", docker.socket_path);
            println!("  Network: {}", docker.network);
            println!("  Default image: {}", docker.default_image);
            println!("  SSH port: {}", docker.ssh_port);
            println!();
            println!("Verifying Docker setup...");
            let verification = provisioner.verify_setup().await;
            if verification.api_reachable == Some(true) {
                println!("  [ok] Docker daemon reachable");
            }
            if verification.storage_accessible == Some(true) {
                println!("  [ok] Docker image storage accessible");
            }
            if verification.template_exists == Some(true) {
                println!("  [ok] Default image '{}' exists", docker.default_image);
            }
            if !verification.warnings.is_empty() {
                println!();
                for warning in &verification.warnings {
                    println!("  [WARN] {}", warning);
                }
            }
            if !verification.errors.is_empty() {
                println!();
                for error in &verification.errors {
                    println!("  [FAILED] {}", error);
                }
                return Err(anyhow::anyhow!(
                    "Docker setup verification failed with {} error(s)",
                    verification.errors.len()
                ));
            }
        }
        ProvisionerConfig::DigitalOcean(do_config) => {
            println!("Provisioner: DigitalOcean");
            println!("  Default size: {}", do_config.default_size);
            println!("  Default region: {}", do_config.default_region);
            println!("  Default image: {}", do_config.default_image);
            println!(
                "  API token: {}...",
                &do_config.api_token.chars().take(8).collect::<String>()
            );
            println!();
            println!("Verifying DigitalOcean setup...");
            let verification = provisioner.verify_setup().await;
            if verification.api_reachable == Some(true) {
                println!("  [ok] DigitalOcean API reachable");
            }
            if !verification.errors.is_empty() {
                println!();
                for error in &verification.errors {
                    println!("  [FAILED] {}", error);
                }
                return Err(anyhow::anyhow!(
                    "DigitalOcean setup verification failed with {} error(s)",
                    verification.errors.len()
                ));
            }
        }
    }
    println!();

    // Check gateway configuration
    match &config.gateway {
        Some(gw) => {
            println!("Gateway:");
            println!("  DC ID: {}", gw.dc_id);
            println!("  Public IP: {}", gw.public_ip);
            println!(
                "  Port range: {}-{} ({} ports/VM)",
                gw.port_range_start, gw.port_range_end, gw.ports_per_vm
            );
            println!("  Caddy sites dir: {}", gw.caddy_sites_dir);
            println!("  Port allocations: {}", gw.port_allocations_path);
            println!("  DNS management: via central API");
            println!("  Wildcard: *.{}.{}.{}", gw.dc_id, gw.gw_prefix, gw.domain);
            println!("  TLS: Per-provider wildcard cert via DNS-01 (acme-dns)");

            // Verify paths exist
            if std::path::Path::new(&gw.caddy_sites_dir).exists() {
                println!("  [ok] Caddy sites directory exists");
            } else {
                println!(
                    "  [WARN] Caddy sites directory does not exist: {}",
                    gw.caddy_sites_dir
                );
                println!("       Re-run setup with --gateway-dc-id to configure gateway");
            }

            // Check if Caddy is running
            match run_command_with_timeout("systemctl", &["is-active", "caddy"], QUICK_QUERY_TIMEOUT)
            {
                Ok(output) => {
                    let status = String::from_utf8_lossy(&output.stdout);
                    if status.trim() == "active" {
                        println!("  [ok] Caddy service is running");

                        // Check if Caddy is listening on expected ports
                        match run_command_with_timeout("ss", &["-tlnp"], QUICK_QUERY_TIMEOUT) {
                            Ok(ss_output) => {
                                let ss = String::from_utf8_lossy(&ss_output.stdout);
                                if ss.contains(":443") && ss.contains("caddy") {
                                    println!("  [ok] Caddy listening on port 443");
                                } else if ss.contains(":443") {
                                    println!("  [ok] Port 443 in use (Caddy or other)");
                                } else {
                                    println!("  [WARN] Caddy not listening on port 443");
                                }
                            }
                            Err(e) => {
                                // `ss` may be missing (iproute2 not installed),
                                // not installed setuid, or otherwise unavailable
                                // — surface this so the operator knows why the
                                // port-listening check did not run instead of
                                // seeing no output and assuming Caddy is fine.
                                println!(
                                    "  [WARN] Cannot verify Caddy listening ports via `ss` ({}); \
                                     install iproute2 or check `ss -tlnp` manually",
                                    e
                                );
                            }
                        }
                    } else {
                        println!(
                            "  [WARN] Caddy service not running (status: {})",
                            status.trim()
                        );
                        println!("       Run: systemctl start caddy");
                    }
                }
                Err(_) => {
                    println!("  [info] Cannot check Caddy status (systemctl not available)");
                }
            }

            // Verify GatewayManager can be initialized (requires API client)
            match api_client.clone() {
                Some(client) => match GatewayManager::new(gw.clone(), client) {
                    Ok(gw_manager) => {
                        println!("  [ok] Gateway manager initialized");

                        // Show current port allocations count
                        let allocations = gw_manager.port_allocations();
                        let count = allocations.allocations.len();
                        if count > 0 {
                            println!("  [info] {} active VM(s) with gateway routing", count);
                        }

                        // Show bandwidth stats if available
                        let stats = gw_manager.get_bandwidth_stats();
                        if !stats.is_empty() {
                            println!("  Bandwidth stats:");
                            for (slug, bw) in &stats {
                                println!(
                                    "    {}: in={} out={}",
                                    slug,
                                    format_bytes(bw.bytes_in),
                                    format_bytes(bw.bytes_out)
                                );
                            }
                        }
                    }
                    Err(e) => println!("  [FAILED] Gateway initialization: {:#}", e),
                },
                None => {
                    println!("  [WARN] Cannot verify gateway manager (API client not available)");
                }
            }
        }
        None => {
            println!("Gateway: Not configured");
            println!("  VMs will not get public subdomains");
            println!("  To enable: re-run setup with --gateway-dc-id <DC>");
        }
    }
    println!();

    let provisioner_type = config.provisioner.type_name();

    // Create API client for verification (separate from the one used for gateway check)
    let api_client = ApiClient::new(&config.api)?;
    println!("[ok] API client initialized");

    if verify_api {
        println!();
        println!("Verifying API connectivity...");

        match api_client
            .send_heartbeat(
                Some(env!("CARGO_PKG_VERSION")),
                Some(provisioner_type),
                None,
                0,
                None, // No bandwidth stats in doctor mode
                None, // No resources in doctor mode
            )
            .await
        {
            Ok(response) => {
                println!("[ok] API authentication successful");
                println!("  Heartbeat acknowledged: {}", response.acknowledged);
                println!("  Next heartbeat in: {}s", response.next_heartbeat_seconds);
            }
            Err(e) => {
                println!("[FAILED] API verification failed: {:#}", e);
                println!();
                println!("Possible causes:");
                println!("  - Agent not registered (run: dc-agent register)");
                println!("  - Agent delegation expired or revoked");
                println!("  - Invalid agent key");
                println!("  - Network connectivity issue");
                return Err(anyhow::anyhow!("API verification failed: {:#}", e));
            }
        }
    }

    // Test provisioning if requested (only for Proxmox)
    if test_provision {
        println!();
        println!("Testing provisioning...");

        match &config.provisioner {
            ProvisionerConfig::Proxmox(_) => {
                let test_contract_id = format!("doctor-test-{}", std::process::id());

                let request = ProvisionRequest {
                    contract_id: test_contract_id.clone(),
                    offering_id: "doctor-test".to_string(),
                    cpu_cores: Some(1),
                    memory_mb: Some(512),
                    storage_gb: None, // Use template default
                    requester_ssh_pubkey: None,
                    instance_config: None,
                    post_provision_script: None,
                };

                println!("  Cloning test VM from template...");
                match provisioner.provision(&request).await {
                    Ok(instance) => {
                        println!("[ok] Test VM created: VMID {}", instance.external_id);

                        // Check if we got an IP address (indicates QEMU guest agent is working)
                        let ip_warning = match &instance.ip_address {
                            None => {
                                println!(
                                    "[WARN] No IP address obtained - QEMU guest agent not running"
                                );
                                println!(
                                    "       Template may be missing qemu-guest-agent package."
                                );
                                println!("       Re-run setup to fix: dc-agent setup token ...");
                                true
                            }
                            Some(ip) => {
                                println!("  IP address: {}", ip);
                                false
                            }
                        };

                        println!("  Terminating test VM...");
                        match provisioner.terminate(&instance.external_id).await {
                            Ok(()) => {
                                println!("[ok] Test VM terminated successfully");
                                println!();
                                if ip_warning {
                                    println!("Provisioning works but IP detection is broken!");
                                    println!("VMs will start but won't report their IP addresses.");
                                    println!();
                                    println!("To fix, install qemu-guest-agent in template:");
                                    println!("  1. SSH to Proxmox: ssh root@<proxmox-host>");
                                    println!("  2. Install libguestfs-tools: apt install libguestfs-tools");
                                    println!("  3. Customize image:");
                                    println!("     virt-customize -a /var/lib/vz/images/<vmid>/vm-<vmid>-disk-0 \\");
                                    println!("       --install qemu-guest-agent \\");
                                    println!(
                                        "       --run-command 'systemctl enable qemu-guest-agent'"
                                    );
                                } else {
                                    println!("Provisioning is working correctly!");
                                }
                            }
                            Err(e) => {
                                println!("[WARN] Test VM created but termination failed: {:#}", e);
                                println!(
                                    "  Manual cleanup may be required for VMID {}",
                                    instance.external_id
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!("[FAILED] Provisioning test failed: {:#}", e);
                        println!();
                        println!("Possible causes:");
                        println!("  - Template VM does not exist or is locked");
                        println!("  - Storage pool is full or inaccessible");
                        println!("  - API token lacks required permissions");
                        println!("  - Resource pool does not exist");
                        return Err(anyhow::anyhow!("Provisioning test failed: {:#}", e));
                    }
                }
            }
            ProvisionerConfig::Docker(_) => {
                let test_contract_id = format!("doctor-test-{}", std::process::id());
                let request = ProvisionRequest {
                    contract_id: test_contract_id.clone(),
                    offering_id: "doctor-test".to_string(),
                    cpu_cores: Some(1),
                    memory_mb: Some(512),
                    storage_gb: None,
                    requester_ssh_pubkey: None,
                    instance_config: None,
                    post_provision_script: None,
                };

                println!("  Creating test Docker container...");
                match provisioner.provision(&request).await {
                    Ok(instance) => {
                        println!("[ok] Test container created: {}", instance.external_id);
                        println!("  Terminating test container...");
                        match provisioner.terminate(&instance.external_id).await {
                            Ok(()) => println!("[ok] Test container terminated successfully"),
                            Err(e) => {
                                println!("[WARN] Container created but termination failed: {:#}", e)
                            }
                        }
                    }
                    Err(e) => {
                        println!("[FAILED] Provisioning test failed: {:#}", e);
                        println!();
                        println!("Possible causes:");
                        println!("  - Docker daemon not running or not accessible");
                        println!("  - Insufficient permissions for Docker socket");
                        println!("  - Network unavailable for image pull");
                        return Err(anyhow::anyhow!("Provisioning test failed: {:#}", e));
                    }
                }
            }
            ProvisionerConfig::DigitalOcean(_) => {
                let test_contract_id = format!("doctor-test-{}", std::process::id());
                let request = ProvisionRequest {
                    contract_id: test_contract_id.clone(),
                    offering_id: "doctor-test".to_string(),
                    cpu_cores: Some(1),
                    memory_mb: Some(1024),
                    storage_gb: None,
                    requester_ssh_pubkey: None,
                    instance_config: None,
                    post_provision_script: None,
                };

                println!("  Creating test DigitalOcean droplet...");
                println!("  WARNING: This will create and destroy a real droplet (billed hourly).");
                match provisioner.provision(&request).await {
                    Ok(instance) => {
                        println!("[ok] Test droplet created: {}", instance.external_id);
                        if let Some(ip) = &instance.ip_address {
                            println!("  IP address: {}", ip);
                        }
                        println!("  Terminating test droplet...");
                        match provisioner.terminate(&instance.external_id).await {
                            Ok(()) => println!("[ok] Test droplet terminated successfully"),
                            Err(e) => {
                                println!("[WARN] Droplet created but termination failed: {:#}", e);
                                println!(
                                    "  Manual cleanup may be required for droplet {}",
                                    instance.external_id
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!("[FAILED] Provisioning test failed: {:#}", e);
                        println!();
                        println!("Possible causes:");
                        println!("  - Invalid API token or insufficient permissions");
                        println!("  - Account droplet limit reached");
                        println!("  - Invalid region or size in config");
                        return Err(anyhow::anyhow!("Provisioning test failed: {:#}", e));
                    }
                }
            }
            _ => {
                println!(
                    "  [skip] --test-provision only supported for Proxmox, Docker, and DigitalOcean provisioners"
                );
            }
        }
    }
    println!();

    println!("Doctor check complete!");
    Ok(())
}
