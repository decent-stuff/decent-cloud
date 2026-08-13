use anyhow::{Context, Result};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::gateway::GatewayManager;
use crate::provisioner::{create_provisioner_from_config, ProvisionRequest};

pub async fn test_provision(
    config: Config,
    ssh_pubkey: Option<String>,
    keep: bool,
    contract_id: Option<String>,
    test_gateway: bool,
    skip_dns: bool,
) -> Result<()> {
    println!("dc-agent test-provision");
    println!("=======================\n");

    // For test-provision, use the default provisioner
    let provisioner = create_provisioner_from_config(&config.provisioner)?;

    // Generate contract ID if not provided
    let contract_id = match contract_id {
        Some(id) => id,
        None => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("System clock is before Unix epoch - check system time")?
                .as_secs();
            format!("test-{}", secs)
        }
    };

    // Use provided SSH key or a placeholder
    let ssh_key = ssh_pubkey.unwrap_or_else(|| {
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITestKeyNotForRealUse test@dc-agent".to_string()
    });

    println!("Contract ID: {}", contract_id);
    println!("SSH Public Key: {}...", &ssh_key[..ssh_key.len().min(50)]);
    if test_gateway {
        println!(
            "Gateway testing: enabled{}",
            if skip_dns { " (DNS skipped)" } else { "" }
        );
    }
    println!();

    let request = ProvisionRequest {
        contract_id: contract_id.clone(),
        offering_id: "test-offering".to_string(),
        cpu_cores: Some(1),
        memory_mb: Some(1024),
        storage_gb: Some(10),
        requester_ssh_pubkey: Some(ssh_key),
        instance_config: None,
        post_provision_script: None,
    };

    println!("Provisioning test VM...");
    let start = std::time::Instant::now();
    let mut instance = provisioner.provision(&request).await?;
    let provision_time = start.elapsed();

    println!(
        "\n✓ VM provisioned successfully in {:.1}s",
        provision_time.as_secs_f64()
    );
    println!();
    println!("Instance details:");
    println!("  External ID: {}", instance.external_id);
    if let Some(ipv4) = &instance.ip_address {
        println!("  IPv4: {}", ipv4);
    }
    if let Some(ipv6) = &instance.ipv6_address {
        println!("  IPv6: {}", ipv6);
    }

    // Gateway setup if requested
    let gateway_manager = if test_gateway {
        match &config.gateway {
            Some(gw_config) => {
                // Create a minimal API client for gateway (DNS operations will be skipped in test mode)
                let api_client = std::sync::Arc::new(
                    ApiClient::new(&config.api)
                        .context("Failed to create API client for gateway")?,
                );
                match GatewayManager::new(gw_config.clone(), api_client) {
                    Ok(mut gm) => {
                        println!(
                            "\nSetting up gateway{}...",
                            if skip_dns {
                                " (local only, no DNS)"
                            } else {
                                ""
                            }
                        );
                        let gw_result = if skip_dns {
                            gm.setup_gateway_local(instance.clone(), &contract_id).await
                        } else {
                            gm.setup_gateway(instance.clone(), &contract_id).await
                        };
                        match gw_result {
                            Ok(updated_instance) => {
                                instance = updated_instance;
                                println!("✓ Gateway setup complete");
                                println!();
                                println!("Gateway details:");
                                if let Some(slug) = &instance.gateway_slug {
                                    println!("  Slug: {}", slug);
                                }
                                if let Some(subdomain) = &instance.gateway_subdomain {
                                    println!("  Subdomain: {}", subdomain);
                                }
                                if let Some(port) = instance.gateway_ssh_port {
                                    println!("  SSH Port: {}", port);
                                }
                                if let (Some(start), Some(end)) = (
                                    instance.gateway_port_range_start,
                                    instance.gateway_port_range_end,
                                ) {
                                    println!("  Port Range: {}-{}", start, end);
                                }
                                Some(gm)
                            }
                            Err(e) => {
                                println!("⚠ Gateway setup failed: {:#}", e);
                                println!(
                                    "  (VM provisioning succeeded, continuing without gateway)"
                                );
                                None
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠ Failed to initialize gateway manager: {:#}", e);
                        None
                    }
                }
            }
            None => {
                println!("\n⚠ --test-gateway specified but no gateway configured in dc-agent.toml");
                None
            }
        }
    } else {
        None
    };

    // Health check
    println!("\nRunning health check...");
    let health = provisioner.health_check(&instance.external_id).await?;
    println!("  Status: {:?}", health);

    if keep {
        println!("\n--keep specified, VM will remain running.");
        println!("To terminate later, use the Proxmox web UI or API.");

        // Show connection instructions
        if let Some(subdomain) = &instance.gateway_subdomain {
            if let Some(port) = instance.gateway_ssh_port {
                println!("\nSSH via gateway:");
                println!("  ssh -p {} ubuntu@{}", port, subdomain);
                println!(
                    "  ssh -p {} ubuntu@{}",
                    port,
                    config
                        .gateway
                        .as_ref()
                        .map(|g| &g.public_ip)
                        .unwrap_or(&"<public_ip>".to_string())
                );
            }
        } else if let Some(ipv4) = &instance.ip_address {
            println!("\nYou can SSH into the VM (internal network only):");
            println!("  ssh ubuntu@{}", ipv4);
        }
    } else {
        // Cleanup gateway first if it was set up
        if let Some(mut gm) = gateway_manager {
            if let Some(slug) = &instance.gateway_slug {
                println!("\nCleaning up gateway (slug: {})...", slug);
                if let Err(e) = gm.cleanup_gateway(slug).await {
                    println!("⚠ Gateway cleanup warning: {:#}", e);
                } else {
                    println!("✓ Gateway cleaned up");
                }
            }
        }

        println!("\nTerminating test VM...");
        provisioner.terminate(&instance.external_id).await?;
        println!("✓ VM terminated successfully");
    }

    println!("\n=======================");
    println!("Test complete!");

    Ok(())
}

/// Reset the root password on a provisioned VM.
pub async fn reset_password(
    config: Config,
    contract_id: &str,
    password: Option<String>,
) -> Result<()> {
    use dcc_common::ssh_exec::reset_password_via_ssh;
    use crate::provisioner::proxmox::generate_secure_password;

    println!("dc-agent reset-password");
    println!("=======================\n");

    let api_client = ApiClient::new(&config.api)?;

    // Get the provisioner
    let provisioner = create_provisioner_from_config(&config.provisioner)?;

    // Get contract details from provisioner (need external_id and IP)
    println!(
        "Looking up contract {}...",
        &contract_id[..16.min(contract_id.len())]
    );

    let _contract_bytes =
        hex::decode(contract_id).with_context(|| "Invalid contract ID format (expected hex)")?;

    // Try to get VM info from the provisioner
    let external_id = format!("dc-{}", contract_id);
    let instance = provisioner
        .get_instance(&external_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("VM not found for contract {}", contract_id))?;

    let ip_address = instance
        .ip_address
        .ok_or_else(|| anyhow::anyhow!("VM has no IP address assigned"))?;

    let ssh_port = instance.ssh_port;

    println!("Found VM at {} (port {})", ip_address, ssh_port);

    // Generate or use provided password
    let new_password = password.unwrap_or_else(|| generate_secure_password(24));
    println!("Generated new password ({} chars)", new_password.len());

    // Reset password via SSH
    println!("Resetting password via SSH...");
    // Use ubuntu user with sudo since cloud-init sets SSH keys for the default user
    reset_password_via_ssh(
        &ip_address,
        ssh_port,
        "ubuntu",
        true,
        &new_password,
        contract_id,
    )
    .await?;

    println!("[ok] Password reset on VM");

    // Report new password to API (encrypted)
    println!("Updating credentials in API...");
    api_client
        .update_contract_password(contract_id, &new_password)
        .await?;

    println!("[ok] Credentials updated in API");
    println!();
    println!("Password reset complete. The user can now retrieve the new password.");
    println!("New password: {}", new_password);

    Ok(())
}
