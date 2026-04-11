use super::*;
use bollard::models::ContainerStateStatusEnum;
use bollard::service::{
    ContainerInspectResponse, ContainerState, EndpointSettings, NetworkSettings,
};

fn default_config() -> DockerConfig {
    DockerConfig {
        socket_path: "/var/run/docker.sock".to_string(),
        network: "bridge".to_string(),
        default_image: "ghcr.io/decent-stuff/dc-agent-ssh:latest".to_string(),
        ssh_port: 22,
    }
}

fn make_provision_request() -> ProvisionRequest {
    ProvisionRequest {
        contract_id: "test-contract-123".to_string(),
        offering_id: "offering-1".to_string(),
        cpu_cores: Some(2),
        memory_mb: Some(1024),
        storage_gb: None,
        requester_ssh_pubkey: Some("ssh-ed25519 AAAATEST".to_string()),
        instance_config: None,
        post_provision_script: None,
    }
}

#[test]
fn test_container_name_format() {
    assert_eq!(container_name("abc123"), "dc-abc123");
    assert_eq!(container_name("contract-456-xyz"), "dc-contract-456-xyz");
}

#[test]
fn test_resolve_image_from_config() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let request = make_provision_request();
    assert_eq!(prov.resolve_image(&request), "ghcr.io/decent-stuff/dc-agent-ssh:latest");
}

#[test]
fn test_resolve_image_from_instance_config() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let mut request = make_provision_request();
    request.instance_config = Some(serde_json::json!({
        "image": "alpine:3.19"
    }));
    assert_eq!(prov.resolve_image(&request), "alpine:3.19");
}

#[test]
fn test_resolve_image_instance_config_non_string_ignored() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let mut request = make_provision_request();
    request.instance_config = Some(serde_json::json!({
        "image": 42
    }));
    assert_eq!(prov.resolve_image(&request), "ghcr.io/decent-stuff/dc-agent-ssh:latest");
}

#[test]
fn test_resolve_image_no_instance_config() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let mut request = make_provision_request();
    request.instance_config = None;
    assert_eq!(prov.resolve_image(&request), "ghcr.io/decent-stuff/dc-agent-ssh:latest");
}

#[test]
fn test_build_container_config_defaults() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let request = make_provision_request();
    let cfg = prov.build_container_config(&request, "ghcr.io/decent-stuff/dc-agent-ssh:latest");

    assert_eq!(cfg.image.as_deref(), Some("ghcr.io/decent-stuff/dc-agent-ssh:latest"));
    assert!(cfg.exposed_ports.is_some());
    assert!(cfg.exposed_ports.as_ref().unwrap().contains_key("22"));

    let labels = cfg.labels.as_ref().unwrap();
    assert_eq!(labels.get("dc-agent"), Some(&"true".to_string()));
    assert_eq!(
        labels.get("dc-contract-id"),
        Some(&"test-contract-123".to_string())
    );

    let host_config = cfg.host_config.as_ref().unwrap();
    assert_eq!(host_config.cpu_count, Some(2));
    assert_eq!(host_config.memory, Some(1024 * 1024 * 1024));
    assert_eq!(host_config.network_mode.as_deref(), Some("bridge"));
}

#[test]
fn test_build_container_config_minimal_request() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let request = ProvisionRequest {
        contract_id: "min".to_string(),
        offering_id: "off-1".to_string(),
        cpu_cores: None,
        memory_mb: None,
        storage_gb: None,
        requester_ssh_pubkey: None,
        instance_config: None,
        post_provision_script: None,
    };
    let cfg = prov.build_container_config(&request, "alpine:3.19");

    let host_config = cfg.host_config.as_ref().unwrap();
    assert_eq!(host_config.cpu_count, Some(1));
    assert_eq!(host_config.memory, Some(512 * 1024 * 1024));

    assert!(cfg.env.is_none() || cfg.env.as_ref().unwrap().is_empty());
}

#[test]
fn test_build_container_config_ssh_key_in_env() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let request = make_provision_request();
    let cfg = prov.build_container_config(&request, "ghcr.io/decent-stuff/dc-agent-ssh:latest");

    let env = cfg.env.unwrap();
    assert!(env.iter().any(|e| e.starts_with("SSH_PUBLIC_KEY=")));
    assert!(env.iter().any(|e| e.contains("ssh-ed25519 AAAATEST")));
}

#[test]
fn test_build_container_config_custom_network() {
    let config = DockerConfig {
        socket_path: "/var/run/docker.sock".to_string(),
        network: "host".to_string(),
        default_image: "ghcr.io/decent-stuff/dc-agent-ssh:latest".to_string(),
        ssh_port: 2222,
    };
    let prov = DockerProvisioner::new_for_test(config);
    let request = make_provision_request();
    let cfg = prov.build_container_config(&request, "ghcr.io/decent-stuff/dc-agent-ssh:latest");

    let host_config = cfg.host_config.as_ref().unwrap();
    assert_eq!(host_config.network_mode.as_deref(), Some("host"));
}

#[test]
fn test_build_container_config_custom_ssh_port() {
    let config = DockerConfig {
        socket_path: "/var/run/docker.sock".to_string(),
        network: "bridge".to_string(),
        default_image: "ghcr.io/decent-stuff/dc-agent-ssh:latest".to_string(),
        ssh_port: 2222,
    };
    let prov = DockerProvisioner::new_for_test(config);
    let request = make_provision_request();
    let cfg = prov.build_container_config(&request, "ghcr.io/decent-stuff/dc-agent-ssh:latest");

    assert!(cfg.exposed_ports.as_ref().unwrap().contains_key("2222"));
}

fn make_port_map(
    port: &str,
    host_port: &str,
) -> HashMap<String, Option<Vec<bollard::service::PortBinding>>> {
    let mut map = HashMap::new();
    map.insert(
        port.to_string(),
        Some(vec![bollard::service::PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(host_port.to_string()),
        }]),
    );
    map
}

#[test]
fn test_container_to_instance_running() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);

    let inspect = ContainerInspectResponse {
        name: Some("/dc-test-contract".to_string()),
        config: Some(bollard::service::ContainerConfig {
            image: Some("ghcr.io/decent-stuff/dc-agent-ssh:latest".to_string()),
            ..Default::default()
        }),
        network_settings: Some(NetworkSettings {
            ip_address: Some("172.17.0.2".to_string()),
            ports: Some(make_port_map("22/tcp", "32768")),
            ..Default::default()
        }),
        state: Some(ContainerState {
            running: Some(true),
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let instance = prov.container_to_instance(&inspect, "abc123").unwrap();
    assert_eq!(instance.external_id, "abc123");
    assert_eq!(instance.ip_address.as_deref(), Some("172.17.0.2"));
    assert_eq!(instance.ssh_port, 32768);
    assert!(instance.additional_details.is_some());
}

#[test]
fn test_container_to_instance_prefers_configured_network_ipv6() {
    let config = DockerConfig {
        network: "dc346-ipv6-net".to_string(),
        ..default_config()
    };
    let prov = DockerProvisioner::new_for_test(config);

    let inspect = ContainerInspectResponse {
        name: Some("/dc-test-contract".to_string()),
        network_settings: Some(NetworkSettings {
            ip_address: Some("172.17.0.2".to_string()),
            global_ipv6_address: Some("fd00:old::99".to_string()),
            networks: Some(HashMap::from([(
                "dc346-ipv6-net".to_string(),
                EndpointSettings {
                    global_ipv6_address: Some("fd00:346::2".to_string()),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        }),
        ..Default::default()
    };

    let instance = prov.container_to_instance(&inspect, "abc123").unwrap();
    assert_eq!(instance.ipv6_address.as_deref(), Some("fd00:346::2"));
}

#[test]
fn test_container_to_instance_no_ip() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);

    let inspect = ContainerInspectResponse {
        name: Some("/dc-test".to_string()),
        network_settings: Some(NetworkSettings {
            ip_address: None,
            ..Default::default()
        }),
        ..Default::default()
    };

    let instance = prov.container_to_instance(&inspect, "id1").unwrap();
    assert!(instance.ip_address.is_none());
    assert_eq!(instance.ssh_port, 22);
}

#[test]
fn test_container_to_instance_empty_ip() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);

    let inspect = ContainerInspectResponse {
        name: Some("/dc-test".to_string()),
        network_settings: Some(NetworkSettings {
            ip_address: Some("".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let instance = prov.container_to_instance(&inspect, "id1").unwrap();
    assert!(instance.ip_address.is_none());
}

#[test]
fn test_container_to_instance_no_port_binding() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);

    let inspect = ContainerInspectResponse {
        name: Some("/dc-test".to_string()),
        network_settings: Some(NetworkSettings {
            ip_address: Some("172.17.0.3".to_string()),
            ports: Some(HashMap::new()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let instance = prov.container_to_instance(&inspect, "id1").unwrap();
    assert_eq!(instance.ssh_port, 22);
}

#[test]
fn test_container_to_instance_labels_and_image() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);

    let inspect = ContainerInspectResponse {
        name: Some("/dc-my-contract".to_string()),
        config: Some(bollard::service::ContainerConfig {
            image: Some("alpine:3.19".to_string()),
            ..Default::default()
        }),
        network_settings: Some(NetworkSettings {
            ip_address: Some("172.17.0.5".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let instance = prov.container_to_instance(&inspect, "id2").unwrap();
    let details = instance.additional_details.unwrap();
    assert_eq!(details["name"], "dc-my-contract");
    assert_eq!(details["image"], "alpine:3.19");
}

#[test]
fn test_health_status_from_running_container() {
    let state = ContainerState {
        running: Some(true),
        started_at: Some(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    };
    let running = state.running.unwrap_or(false);
    assert!(running);
}

#[test]
fn test_health_status_from_stopped_container() {
    let state = ContainerState {
        running: Some(false),
        status: Some(ContainerStateStatusEnum::EXITED),
        exit_code: Some(137),
        ..Default::default()
    };
    assert!(!state.running.unwrap_or(false));
    assert_eq!(state.exit_code, Some(137));
}

#[test]
fn test_container_to_instance_empty_inspect_returns_none() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let result = prov.container_to_instance(&ContainerInspectResponse::default(), "id1");
    assert!(
        result.is_none(),
        "Should return None for inspect with no name"
    );
}

#[test]
fn test_build_container_config_has_cmd() {
    let config = default_config();
    let prov = DockerProvisioner::new_for_test(config);
    let request = make_provision_request();
    let cfg = prov.build_container_config(&request, "ghcr.io/decent-stuff/dc-agent-ssh:latest");

    let cmd = cfg.cmd.expect("cmd must be set for SSH setup");
    assert_eq!(cmd[0], "/bin/bash");
    assert_eq!(cmd[1], "-c");
    assert!(
        !cmd[2].contains("apt-get"),
        "cmd must NOT contain apt-get (pre-built image has openssh-server)"
    );
    assert!(
        cmd[2].contains("authorized_keys"),
        "cmd must set up authorized_keys"
    );
    assert!(
        cmd[2].contains("sshd -D"),
        "cmd must start sshd in foreground"
    );
}

#[tokio::test]
async fn test_terminate_not_found_returns_ok() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/containers/nonexistent/json")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "No such container: nonexistent"}"#)
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let result = prov.terminate("nonexistent").await;
    assert!(result.is_ok(), "terminate() should return Ok for 404");
}

#[tokio::test]
async fn test_health_check_not_found_returns_unhealthy() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/containers/missing/json")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "No such container: missing"}"#)
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let result = prov.health_check("missing").await.unwrap();
    assert!(
        matches!(result, HealthStatus::Unhealthy { .. }),
        "health_check should return Unhealthy for 404"
    );
}

#[tokio::test]
async fn test_pull_image_propagates_list_error() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/images/json")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "permission denied"}"#)
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let result = prov.pull_image_if_needed("ghcr.io/decent-stuff/dc-agent-ssh:latest").await;
    assert!(
        result.is_err(),
        "pull_image_if_needed() should propagate list_images error"
    );
}

#[tokio::test]
async fn test_verify_setup_image_found() {
    let mut server = mockito::Server::new_async().await;
    let _ping = server
        .mock("GET", "/_ping")
        .with_status(200)
        .with_body("OK")
        .create_async()
        .await;
    let _images = server
        .mock("GET", "/images/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"Id":"sha256:abc","RepoTags":["ghcr.io/decent-stuff/dc-agent-ssh:latest"],"Created":0,"Size":0,"VirtualSize":0,"SharedSize":0,"Containers":0,"Labels":{},"ParentId":"","RepoDigests":[]}]"#)
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let result = prov.verify_setup().await;
    assert_eq!(result.api_reachable, Some(true));
    assert_eq!(result.storage_accessible, Some(true));
    assert_eq!(result.template_exists, Some(true));
    assert!(
        result.errors.is_empty(),
        "Expected no errors, got: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn test_verify_setup_image_not_found() {
    let mut server = mockito::Server::new_async().await;
    let _ping = server
        .mock("GET", "/_ping")
        .with_status(200)
        .with_body("OK")
        .create_async()
        .await;
    let _images = server
        .mock("GET", "/images/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"Id":"sha256:def","RepoTags":["alpine:3.19"],"Created":0,"Size":0,"VirtualSize":0,"SharedSize":0,"Containers":0,"Labels":{},"ParentId":"","RepoDigests":[]}]"#)
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let result = prov.verify_setup().await;
    assert_eq!(result.template_exists, Some(false));
    assert_eq!(result.errors.len(), 1);
    let err = &result.errors[0];
    assert!(
        err.contains("ghcr.io/decent-stuff/dc-agent-ssh:latest"),
        "Error should mention the image name: {}",
        err
    );
    assert!(
        err.contains("docker pull"),
        "Error should suggest docker pull: {}",
        err
    );
}

#[tokio::test]
async fn test_verify_setup_image_not_found_custom_image() {
    let mut server = mockito::Server::new_async().await;
    let _ping = server
        .mock("GET", "/_ping")
        .with_status(200)
        .with_body("OK")
        .create_async()
        .await;
    let _images = server
        .mock("GET", "/images/json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito_with_image(
        server.url(),
        "my-registry/custom:latest".to_string(),
    );
    let result = prov.verify_setup().await;
    assert_eq!(result.template_exists, Some(false));
    assert_eq!(result.errors.len(), 1);
    let err = &result.errors[0];
    assert!(
        err.contains("my-registry/custom:latest"),
        "Error should mention the custom image name: {}",
        err
    );
    assert!(
        err.contains("docker pull my-registry/custom:latest"),
        "Error should suggest the exact docker pull command: {}",
        err
    );
}

// ── Ticket 347: parse_cpu_model ──────────────────────────────────────────

#[test]
fn test_parse_cpu_model_x86() {
    let cpuinfo = "\
processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz\ncpu MHz\t\t: 3800.000\n\
processor\t: 1\nmodel name\t: Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz\ncpu MHz\t\t: 3800.000\n";
    let model = super::parse_cpu_model(cpuinfo).expect("should parse model name");
    assert_eq!(model, "Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz");
}

#[test]
fn test_parse_cpu_model_amd() {
    let cpuinfo = "processor\t: 0\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD EPYC 7763 64-Core Processor\n";
    let model = super::parse_cpu_model(cpuinfo).expect("should parse AMD model");
    assert_eq!(model, "AMD EPYC 7763 64-Core Processor");
}

#[test]
fn test_parse_cpu_model_arm_hardware() {
    let cpuinfo = "Processor\t: AArch64 Processor rev 1 (aarch64)\nHardware\t: BCM2835\n";
    let model = super::parse_cpu_model(cpuinfo).expect("should parse Hardware field");
    assert_eq!(model, "BCM2835");
}

#[test]
fn test_parse_cpu_model_arm_processor() {
    let cpuinfo = "Processor\t: ARMv7 Processor rev 3 (v7l)\nBogoMIPS\t: 89.99\n";
    let model = super::parse_cpu_model(cpuinfo).expect("should parse Processor field");
    assert_eq!(model, "ARMv7 Processor rev 3 (v7l)");
}

#[test]
fn test_parse_cpu_model_model_name_preferred_over_hardware() {
    let cpuinfo = "model name\t: ARM Cortex-A72\nHardware\t: BCM2837\n";
    let model = super::parse_cpu_model(cpuinfo).expect("should prefer model name");
    assert_eq!(model, "ARM Cortex-A72");
}

#[test]
fn test_parse_cpu_model_missing() {
    let cpuinfo = "processor\t: 0\nvendor_id\t: GenuineIntel\ncpu MHz\t\t: 3800.000\n";
    assert!(
        super::parse_cpu_model(cpuinfo).is_none(),
        "no model name field → None"
    );
}

#[test]
fn test_parse_cpu_model_empty_value() {
    let cpuinfo = "model name\t:\n";
    assert!(
        super::parse_cpu_model(cpuinfo).is_none(),
        "empty model name value → None"
    );
}

// ── Ticket 347: parse_mem_available_mb ──────────────────────────────────

#[test]
fn test_parse_mem_available_mb_typical() {
    let meminfo = "\
MemTotal:       16384000 kB\n\
MemFree:         2048000 kB\n\
MemAvailable:    8192000 kB\n\
Buffers:          204800 kB\n";
    let avail = super::parse_mem_available_mb(meminfo).expect("should parse MemAvailable");
    assert_eq!(avail, 8000); // 8192000 kB / 1024 = 8000 MB
}

#[test]
fn test_parse_mem_available_mb_missing() {
    let meminfo = "MemTotal:       16384000 kB\nMemFree:         2048000 kB\n";
    assert!(
        super::parse_mem_available_mb(meminfo).is_none(),
        "no MemAvailable → None"
    );
}

#[test]
fn test_parse_mem_available_mb_zero() {
    let meminfo = "MemAvailable:          0 kB\n";
    let avail = super::parse_mem_available_mb(meminfo).expect("should parse zero");
    assert_eq!(avail, 0);
}

// ── Ticket 347: collect_resources via mockito ────────────────────────────

/// Mock the /info endpoint to confirm cpu_model, memory fields, and
/// storage_pools are populated.
#[tokio::test]
async fn test_collect_resources_populates_cpu_and_memory() {
    let mut server = mockito::Server::new_async().await;

    let _info_mock = server
        .mock("GET", "/info")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "NCPU": 4,
                "MemTotal": 8589934592,
                "Driver": "overlay2",
                "DockerRootDir": "/var/lib/docker"
            }"#,
        )
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let resources = prov.collect_resources().await;

    let inv = resources.expect("collect_resources() must return Some");

    assert_eq!(inv.cpu_threads, 4);
    assert_eq!(inv.memory_total_mb, 8192);
    assert!(
        inv.memory_available_mb > 0,
        "available_mb must be non-zero (read from /proc/meminfo)"
    );
    assert!(inv.gpu_devices.is_empty());
    assert!(inv.templates.is_empty());
}

#[test]
fn test_fs_stats_on_tmp() {
    let (total, avail) = super::fs_stats("/tmp").expect("statvfs on /tmp must succeed");
    assert!(total > 0, "total bytes on /tmp must be non-zero");
    assert!(avail > 0, "available bytes on /tmp must be non-zero");
    assert!(avail <= total, "available must not exceed total");
}

#[test]
fn test_fs_stats_nonexistent_path_returns_none() {
    assert!(
        super::fs_stats("/no/such/path/statvfs-test-347").is_none(),
        "statvfs on nonexistent path must return None"
    );
}

#[tokio::test]
async fn test_collect_resources_storage_pools_populated() {
    let mut server = mockito::Server::new_async().await;

    let _info_mock = server
        .mock("GET", "/info")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "NCPU": 2,
                "MemTotal": 4294967296,
                "Driver": "overlay2",
                "DockerRootDir": "/tmp"
            }"#,
        )
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let inv = prov
        .collect_resources()
        .await
        .expect("collect_resources must return Some");

    assert_eq!(
        inv.storage_pools.len(),
        1,
        "should have exactly one storage pool"
    );
    let pool = &inv.storage_pools[0];
    assert_eq!(pool.name, "/tmp");
    assert_eq!(pool.storage_type, "overlay2");
    assert!(pool.total_gb > 0, "total_gb must be non-zero for /tmp");
    assert!(
        pool.available_gb > 0,
        "available_gb must be non-zero for /tmp"
    );
}

/// When /info fails, collect_resources() must return None.
#[tokio::test]
async fn test_collect_resources_returns_none_on_info_error() {
    let mut server = mockito::Server::new_async().await;
    let _info_mock = server
        .mock("GET", "/info")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"internal error"}"#)
        .create_async()
        .await;

    let prov = DockerProvisioner::new_for_mockito(server.url());
    let resources = prov.collect_resources().await;
    assert!(
        resources.is_none(),
        "collect_resources() must return None when Docker info fails"
    );
}
