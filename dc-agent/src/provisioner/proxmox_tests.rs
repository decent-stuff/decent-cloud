#[cfg(test)]
mod tests {
    use super::super::{HealthStatus, ProvisionRequest, Provisioner};
    use crate::config::ProxmoxConfig;
    use crate::provisioner::proxmox::{fnv1a_hash, ProxmoxProvisioner};
    use mockito::Server;

    fn test_config(server_url: &str) -> ProxmoxConfig {
        ProxmoxConfig {
            api_url: server_url.to_string(),
            api_token_id: "root@pam!test".to_string(),
            api_token_secret: "test-secret".to_string(),
            node: "pve1".to_string(),
            template_vmid: 9000,
            storage: "local-lvm".to_string(),
            pool: None,
            verify_ssl: false,
        }
    }

    fn test_provision_request() -> ProvisionRequest {
        ProvisionRequest {
            contract_id: "test-contract-123".to_string(),
            offering_id: "off-1".to_string(),
            cpu_cores: Some(2),
            memory_mb: Some(2048),
            storage_gb: Some(20),
            requester_ssh_pubkey: Some("ssh-ed25519 AAAA... user@host".to_string()),
            instance_config: None,
        }
    }

    #[tokio::test]
    async fn test_provision_vm_success() {
        let mut server = Server::new_async().await;

        // Mock clone endpoint - returns UPID
        let _clone_mock = server
            .mock("POST", "/api2/json/nodes/pve1/qemu/9000/clone")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001234:12345678:12345678:qmclone:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock task status - completed for clone
        let _task_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/tasks/.*/status".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"status":"stopped","exitstatus":"OK"}}"#)
            .create_async()
            .await;

        // Mock configure endpoint
        let _config_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/config".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock resize endpoint
        let _resize_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/resize".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock start endpoint - returns UPID
        let _start_mock = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/status/start".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001235:12345678:12345678:qmstart:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock get IP (QEMU guest agent) - return IP on first call
        let _network_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/agent/network-get-interfaces".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"result":[{"name":"eth0","ip-addresses":[{"ip-address":"10.0.0.100","ip-address-type":"ipv4","prefix":24}]}]}}"#,
            )
            .expect_at_least(1)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let request = test_provision_request();

        let result = provisioner.provision(&request).await;
        assert!(result.is_ok(), "Provision should succeed: {:?}", result);

        let instance = result.unwrap();
        assert!(!instance.external_id.is_empty());
        assert_eq!(instance.ip_address, Some("10.0.0.100".to_string()));
        assert_eq!(instance.ssh_port, 22);
    }

    #[tokio::test]
    async fn test_provision_vm_clone_task_failure() {
        let mut server = Server::new_async().await;

        // Mock clone endpoint - returns UPID
        let _clone_mock = server
            .mock("POST", "/api2/json/nodes/pve1/qemu/9000/clone")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001234:12345678:12345678:qmclone:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock task status - failed
        let _task_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/tasks/.*/status".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"status":"stopped","exitstatus":"VMID 100 already exists"}}"#)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let request = test_provision_request();

        let result = provisioner.provision(&request).await;
        assert!(
            result.is_err(),
            "Provision should fail when clone task fails"
        );
        let err = result.unwrap_err();
        let err_msg = format!("{:#}", err);
        assert!(
            err_msg.contains("VMID 100 already exists")
                || err_msg.contains("Task failed")
                || err_msg.contains("Clone task failed"),
            "Expected task failure error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_provision_vm_network_unavailable() {
        let mut server = Server::new_async().await;

        // Mock clone endpoint
        let _clone_mock = server
            .mock("POST", "/api2/json/nodes/pve1/qemu/9000/clone")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001234:12345678:12345678:qmclone:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock task status - success
        let _task_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/tasks/.*/status".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"status":"stopped","exitstatus":"OK"}}"#)
            .create_async()
            .await;

        // Mock configure endpoint
        let _config_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/config".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock resize endpoint
        let _resize_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/resize".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock start endpoint
        let _start_mock = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/status/start".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001235:12345678:12345678:qmstart:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock get IP - guest agent not available (404)
        let _network_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/agent/network-get-interfaces".to_string(),
                ),
            )
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .expect_at_least(1)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let request = test_provision_request();

        let result = provisioner.provision(&request).await;
        // Provisioning MUST fail if no IP address can be obtained - a VM without an IP is useless
        assert!(
            result.is_err(),
            "Provision should fail when no IP address is available"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no IP address obtained"),
            "Error should mention IP address issue: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_terminate_vm_success() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint - VM is running
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/12345/status/current")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"vmid":12345,"status":"running","uptime":3600,"name":"dc-test"}}"#,
            )
            .create_async()
            .await;

        // Mock stop endpoint - returns UPID
        let _stop_mock = server
            .mock("POST", "/api2/json/nodes/pve1/qemu/12345/status/stop")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001236:12345678:12345678:qmstop:12345:root@pam:"}"#)
            .create_async()
            .await;

        // Mock task status - completed
        let _task_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/tasks/.*/status".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"status":"stopped","exitstatus":"OK"}}"#)
            .create_async()
            .await;

        // Mock delete endpoint
        let _delete_mock = server
            .mock(
                "DELETE",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/12345.*".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":"UPID:pve1:00001237:12345678:12345678:qmdestroy:12345:root@pam:"}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.terminate("12345").await;
        assert!(result.is_ok(), "Terminate should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_terminate_vm_already_stopped() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint - VM is already stopped
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/12345/status/current")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"vmid":12345,"status":"stopped","name":"dc-test"}}"#)
            .create_async()
            .await;

        // Mock delete endpoint (no stop needed)
        let _delete_mock = server
            .mock(
                "DELETE",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/12345.*".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":"UPID:pve1:00001237:12345678:12345678:qmdestroy:12345:root@pam:"}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.terminate("12345").await;
        assert!(
            result.is_ok(),
            "Terminate should succeed for stopped VM: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_terminate_vm_not_found() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint - VM not found (404)
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/99999/status/current")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.terminate("99999").await;
        assert!(
            result.is_ok(),
            "Terminate should succeed for non-existent VM (idempotent): {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_health_check_running() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint - VM is running
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/12345/status/current")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"vmid":12345,"status":"running","uptime":3600,"name":"dc-test"}}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.health_check("12345").await;
        assert!(result.is_ok(), "Health check should succeed: {:?}", result);

        match result.unwrap() {
            HealthStatus::Healthy { uptime_seconds } => {
                assert_eq!(uptime_seconds, 3600);
            }
            _ => panic!("Expected Healthy status"),
        }
    }

    #[tokio::test]
    async fn test_health_check_stopped() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint - VM is stopped
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/12345/status/current")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"vmid":12345,"status":"stopped","name":"dc-test"}}"#)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.health_check("12345").await;
        assert!(result.is_ok(), "Health check should succeed: {:?}", result);

        match result.unwrap() {
            HealthStatus::Unhealthy { reason } => {
                assert!(reason.contains("stopped"));
            }
            _ => panic!("Expected Unhealthy status"),
        }
    }

    #[tokio::test]
    async fn test_health_check_not_found() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint - VM not found
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/99999/status/current")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.health_check("99999").await;
        assert!(result.is_ok(), "Health check should succeed: {:?}", result);

        match result.unwrap() {
            HealthStatus::Unhealthy { reason } => {
                assert_eq!(reason, "VM not found");
            }
            _ => panic!("Expected Unhealthy status with 'VM not found'"),
        }
    }

    #[tokio::test]
    async fn test_get_instance_with_ip() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/12345/status/current")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"vmid":12345,"status":"running","uptime":3600,"name":"dc-test"}}"#,
            )
            .create_async()
            .await;

        // Mock get IP (QEMU guest agent)
        let _network_mock = server
            .mock(
                "GET",
                "/api2/json/nodes/pve1/qemu/12345/agent/network-get-interfaces",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"result":[{"name":"lo","ip-addresses":[{"ip-address":"127.0.0.1","ip-address-type":"ipv4","prefix":8}]},{"name":"eth0","ip-addresses":[{"ip-address":"10.0.0.100","ip-address-type":"ipv4","prefix":24},{"ip-address":"2001:db8::1","ip-address-type":"ipv6","prefix":64}]}]}}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.get_instance("12345").await;
        assert!(result.is_ok(), "Get instance should succeed: {:?}", result);

        let instance = result.unwrap();
        assert!(instance.is_some());

        let inst = instance.unwrap();
        assert_eq!(inst.external_id, "12345");
        assert_eq!(inst.ip_address, Some("10.0.0.100".to_string()));
        assert_eq!(inst.ipv6_address, Some("2001:db8::1".to_string()));
        assert_eq!(inst.ssh_port, 22);
    }

    #[tokio::test]
    async fn test_get_instance_not_found() {
        let mut server = Server::new_async().await;

        // Mock get status endpoint - VM not found
        let _status_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu/99999/status/current")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let result = provisioner.get_instance("99999").await;
        assert!(result.is_ok(), "Get instance should succeed: {:?}", result);
        assert!(
            result.unwrap().is_none(),
            "Should return None for non-existent VM"
        );
    }

    #[test]
    fn test_vmid_generation_deterministic() {
        let config = ProxmoxConfig {
            api_url: "https://test.local:8006".to_string(),
            api_token_id: "root@pam!test".to_string(),
            api_token_secret: "test-secret".to_string(),
            node: "pve1".to_string(),
            template_vmid: 9000,
            storage: "local-lvm".to_string(),
            pool: None,
            verify_ssl: false,
        };

        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        // Same contract_id should always produce same VMID
        let vmid1 = provisioner.allocate_vmid("contract-123");
        let vmid2 = provisioner.allocate_vmid("contract-123");
        assert_eq!(vmid1, vmid2);

        // Different contract_id should produce different VMID
        let vmid3 = provisioner.allocate_vmid("contract-456");
        assert_ne!(vmid1, vmid3);

        // VMID should be in valid range (10000-999999)
        assert!(vmid1 >= 10000);
        assert!(vmid1 < 1000000);
        assert!(vmid3 >= 10000);
        assert!(vmid3 < 1000000);

        // Verify known values to detect hash algorithm changes
        // These values are computed from FNV-1a and must remain stable
        assert_eq!(
            provisioner.allocate_vmid("test-contract-abc"),
            10000 + (fnv1a_hash(b"test-contract-abc") % 990000) as u32,
            "VMID generation must use FNV-1a hash"
        );
    }

    #[tokio::test]
    async fn test_provision_with_ipv6_only() {
        let mut server = Server::new_async().await;

        // Mock clone endpoint
        let _clone_mock = server
            .mock("POST", "/api2/json/nodes/pve1/qemu/9000/clone")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001234:12345678:12345678:qmclone:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock task status
        let _task_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/tasks/.*/status".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"status":"stopped","exitstatus":"OK"}}"#)
            .create_async()
            .await;

        // Mock configure endpoint
        let _config_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/config".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock resize endpoint
        let _resize_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/resize".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock start endpoint
        let _start_mock = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/status/start".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001235:12345678:12345678:qmstart:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock get IP - IPv6 only
        let _network_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/agent/network-get-interfaces".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"result":[{"name":"eth0","ip-addresses":[{"ip-address":"2001:db8::1","ip-address-type":"ipv6","prefix":64}]}]}}"#,
            )
            .expect_at_least(1)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let request = test_provision_request();

        let result = provisioner.provision(&request).await;
        assert!(result.is_ok(), "Provision should succeed: {:?}", result);

        let instance = result.unwrap();
        assert!(instance.ip_address.is_none(), "IPv4 should be None");
        assert_eq!(
            instance.ipv6_address,
            Some("2001:db8::1".to_string()),
            "IPv6 should be present"
        );
    }

    #[tokio::test]
    async fn test_provision_idempotent_reuses_existing_vm() {
        let mut server = Server::new_async().await;

        // Mock get status - VM already exists and running
        let _status_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/status/current".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"vmid":12345,"status":"running","uptime":3600,"name":"dc-test"}}"#,
            )
            .create_async()
            .await;

        // Mock get IP
        let _network_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/agent/network-get-interfaces".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"result":[{"name":"eth0","ip-addresses":[{"ip-address":"10.0.0.100","ip-address-type":"ipv4","prefix":24}]}]}}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let request = test_provision_request();

        // First provision - should reuse existing VM
        let result = provisioner.provision(&request).await;
        assert!(result.is_ok(), "Provision should succeed: {:?}", result);

        let instance = result.unwrap();
        assert_eq!(instance.ip_address, Some("10.0.0.100".to_string()));
        // Check that reused flag is set
        let details = instance.additional_details.unwrap();
        assert_eq!(details.get("reused"), Some(&serde_json::json!(true)));
    }

    #[tokio::test]
    async fn test_provision_idempotent_starts_stopped_vm() {
        let mut server = Server::new_async().await;

        // Mock get status - VM exists but stopped
        let _status_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/status/current".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"vmid":12345,"status":"stopped","name":"dc-test"}}"#)
            .create_async()
            .await;

        // Mock start endpoint
        let _start_mock = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/status/start".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001235:12345678:12345678:qmstart:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock task status
        let _task_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/tasks/.*/status".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"status":"stopped","exitstatus":"OK"}}"#)
            .create_async()
            .await;

        // Mock get IP
        let _network_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/agent/network-get-interfaces".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"result":[{"name":"eth0","ip-addresses":[{"ip-address":"10.0.0.101","ip-address-type":"ipv4","prefix":24}]}]}}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let request = test_provision_request();

        let result = provisioner.provision(&request).await;
        assert!(result.is_ok(), "Provision should succeed: {:?}", result);

        let instance = result.unwrap();
        // VM was started, so should have IP
        assert_eq!(instance.ip_address, Some("10.0.0.101".to_string()));
        // Check that reused flag is set
        let details = instance.additional_details.unwrap();
        assert_eq!(details.get("reused"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_extract_contract_id_valid() {
        assert_eq!(
            ProxmoxProvisioner::extract_contract_id("dc-test-contract-123"),
            Some("test-contract-123".to_string())
        );
    }

    #[test]
    fn test_extract_contract_id_no_prefix() {
        assert_eq!(ProxmoxProvisioner::extract_contract_id("test-vm"), None);
    }

    #[test]
    fn test_extract_contract_id_empty_after_prefix() {
        assert_eq!(
            ProxmoxProvisioner::extract_contract_id("dc-"),
            Some("".to_string())
        );
    }

    #[tokio::test]
    async fn test_list_running_instances() {
        let mut server = Server::new_async().await;

        // Mock VM list - includes running dc- VMs, stopped VMs, and non-dc VMs
        let _list_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":[
                    {"vmid":100,"name":"dc-contract-123","status":"running"},
                    {"vmid":101,"name":"dc-contract-456","status":"stopped"},
                    {"vmid":102,"name":"other-vm","status":"running"},
                    {"vmid":103,"name":"dc-contract-789","status":"running"},
                    {"vmid":9000,"status":"stopped"}
                ]}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let instances = provisioner.list_running_instances().await.unwrap();

        // Should only include running VMs with dc- prefix
        assert_eq!(instances.len(), 2);

        // Verify contract IDs are extracted correctly
        assert_eq!(instances[0].external_id, "100");
        assert_eq!(instances[0].contract_id, Some("contract-123".to_string()));

        assert_eq!(instances[1].external_id, "103");
        assert_eq!(instances[1].contract_id, Some("contract-789".to_string()));
    }

    #[tokio::test]
    async fn test_list_running_instances_empty() {
        let mut server = Server::new_async().await;

        // Mock VM list - no running dc- VMs
        let _list_mock = server
            .mock("GET", "/api2/json/nodes/pve1/qemu")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":[{"vmid":100,"name":"other-vm","status":"running"}]}"#)
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        let instances = provisioner.list_running_instances().await.unwrap();
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_provision_vm_template_override() {
        // Test that instance_config.template_vmid overrides the default template
        let mut server = Server::new_async().await;

        // Mock clone endpoint for template 8000 (the override), NOT 9000 (the default)
        let clone_mock = server
            .mock("POST", "/api2/json/nodes/pve1/qemu/8000/clone")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001234:12345678:12345678:qmclone:100:root@pam:"}"#)
            .expect(1) // Expect exactly 1 call to template 8000
            .create_async()
            .await;

        // Mock task status - completed
        let _task_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/tasks/.*/status".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"status":"stopped","exitstatus":"OK"}}"#)
            .create_async()
            .await;

        // Mock configure endpoint
        let _config_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/config".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock resize endpoint
        let _resize_mock = server
            .mock(
                "PUT",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/resize".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":null}"#)
            .create_async()
            .await;

        // Mock start endpoint
        let _start_mock = server
            .mock(
                "POST",
                mockito::Matcher::Regex(r"/api2/json/nodes/pve1/qemu/\d+/status/start".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":"UPID:pve1:00001235:12345678:12345678:qmstart:100:root@pam:"}"#)
            .create_async()
            .await;

        // Mock get IP
        let _network_mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"/api2/json/nodes/pve1/qemu/\d+/agent/network-get-interfaces".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":{"result":[{"name":"eth0","ip-addresses":[{"ip-address":"10.0.0.100","ip-address-type":"ipv4"}]}]}}"#,
            )
            .create_async()
            .await;

        let config = test_config(&server.url());
        let provisioner = ProxmoxProvisioner::new(config).unwrap();

        // Create request with template_vmid override in instance_config
        let request = ProvisionRequest {
            contract_id: "template-override-test".to_string(),
            offering_id: "off-1".to_string(),
            cpu_cores: Some(2),
            memory_mb: Some(2048),
            storage_gb: Some(20),
            requester_ssh_pubkey: Some("ssh-ed25519 AAAA... user@host".to_string()),
            instance_config: Some(serde_json::json!({"template_vmid": 8000})),
        };

        let result = provisioner.provision(&request).await;
        assert!(result.is_ok(), "Provision should succeed: {:?}", result);

        // Verify the clone was made from template 8000, not the default 9000
        clone_mock.assert();
    }
}
