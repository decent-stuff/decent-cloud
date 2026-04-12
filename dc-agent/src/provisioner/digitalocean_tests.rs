use super::*;

// ── DO API response deserialization tests ────────────────────────────────────
// These validate the response types against known DigitalOcean API v2 JSON.

#[test]
fn test_deserialize_droplets_response() {
    let json = r#"{
        "droplets": [
            {
                "id": 3164444,
                "name": "example.com",
                "status": "active",
                "memory": 1024,
                "vcpus": 1,
                "disk": 25,
                "locked": false,
                "created_at": "2020-07-21T18:37:44Z",
                "networks": {
                    "v4": [
                        {"ip_address": "10.128.192.124", "netmask": "255.255.0.0", "gateway": "", "type": "private"},
                        {"ip_address": "192.241.165.154", "netmask": "255.255.255.0", "gateway": "192.241.165.1", "type": "public"}
                    ],
                    "v6": [
                        {"ip_address": "2604:a880:0:1010::18a:a001", "netmask": 64, "gateway": "2604:a880:0:1010::1", "type": "public"}
                    ]
                },
                "region": {"name": "New York 3", "slug": "nyc3"},
                "size_slug": "s-1vcpu-1gb",
                "tags": ["web", "env:prod"],
                "image": {
                    "id": 63663980,
                    "name": "20.04 (LTS) x64",
                    "slug": "ubuntu-20-04-x64",
                    "distribution": "Ubuntu"
                },
                "features": ["backups", "private_networking", "ipv6"],
                "backup_ids": [],
                "snapshot_ids": [],
                "volume_ids": []
            }
        ],
        "meta": {"total": 1}
    }"#;

    let resp: DropletsResponse = serde_json::from_str(json).expect("Failed to deserialize droplets");
    assert_eq!(resp.droplets.len(), 1);

    let droplet = &resp.droplets[0];
    assert_eq!(droplet.id, 3164444);
    assert_eq!(droplet.name, "example.com");
    assert_eq!(droplet.status, "active");
    assert_eq!(droplet.memory, 1024);
    assert_eq!(droplet.vcpus, 1);
    assert_eq!(droplet.disk, 25);
    assert_eq!(droplet.region.slug, "nyc3");
    assert_eq!(droplet.size_slug, "s-1vcpu-1gb");
    assert_eq!(droplet.tags, vec!["web", "env:prod"]);

    assert_eq!(
        droplet.public_ipv4().as_deref(),
        Some("192.241.165.154")
    );
    assert_eq!(
        droplet.public_ipv6().as_deref(),
        Some("2604:a880:0:1010::18a:a001")
    );

    let meta = resp.meta.as_ref().expect("meta should be present");
    assert_eq!(meta.total, 1);
}

#[test]
fn test_deserialize_create_droplet_response() {
    let json = r#"{
        "droplet": {
            "id": 12345678,
            "name": "dc-test-contract",
            "status": "new",
            "memory": 1024,
            "vcpus": 1,
            "disk": 25,
            "locked": false,
            "created_at": "2020-07-21T18:37:44Z",
            "networks": {"v4": [], "v6": []},
            "region": {"name": "New York 3", "slug": "nyc3"},
            "size_slug": "s-1vcpu-1gb",
            "tags": ["dc-agent"],
            "features": [],
            "backup_ids": [],
            "snapshot_ids": [],
            "volume_ids": []
        },
        "links": {"actions": [{"id": 999, "rel": "create", "href": "https://api.digitalocean.com/v2/actions/999"}]}
    }"#;

    let resp: DropletResponse = serde_json::from_str(json).expect("Failed to deserialize droplet");
    assert_eq!(resp.droplet.id, 12345678);
    assert_eq!(resp.droplet.status, "new");
    assert!(resp.droplet.public_ipv4().is_none());
}

#[test]
fn test_deserialize_sizes_response() {
    let json = r#"{
        "sizes": [
            {
                "slug": "s-1vcpu-1gb",
                "memory": 1024,
                "vcpus": 1,
                "disk": 25,
                "price_monthly": 5.0,
                "price_hourly": 0.00744,
                "available": true,
                "regions": ["nyc3", "ams3", "sfo3"]
            },
            {
                "slug": "s-2vcpu-4gb",
                "memory": 4096,
                "vcpus": 2,
                "disk": 80,
                "price_monthly": 20.0,
                "price_hourly": 0.02976,
                "available": true,
                "regions": ["nyc3", "ams3", "sfo3"]
            }
        ],
        "meta": {"total": 2}
    }"#;

    let resp: SizesResponse = serde_json::from_str(json).expect("Failed to deserialize sizes");
    assert_eq!(resp.sizes.len(), 2);
    assert_eq!(resp.sizes[0].slug, "s-1vcpu-1gb");
    assert_eq!(resp.sizes[1].vcpus, 2);
}

#[test]
fn test_deserialize_regions_response() {
    let json = r#"{
        "regions": [
            {"name": "New York 3", "slug": "nyc3", "available": true},
            {"name": "Amsterdam 3", "slug": "ams3", "available": true}
        ],
        "meta": {"total": 2}
    }"#;

    let resp: RegionsResponse = serde_json::from_str(json).expect("Failed to deserialize regions");
    assert_eq!(resp.regions.len(), 2);
    assert_eq!(resp.regions[0].slug, "nyc3");
}

#[test]
fn test_deserialize_images_response() {
    let json = r#"{
        "images": [
            {
                "id": 63663980,
                "name": "20.04 (LTS) x64",
                "slug": "ubuntu-20-04-x64",
                "distribution": "Ubuntu",
                "public": true,
                "available": true
            },
            {
                "id": 12345,
                "name": "My Custom Image",
                "slug": null,
                "distribution": "Ubuntu",
                "public": false,
                "available": true
            }
        ],
        "meta": {"total": 2}
    }"#;

    let resp: ImagesResponse = serde_json::from_str(json).expect("Failed to deserialize images");
    assert_eq!(resp.images.len(), 2);
    assert_eq!(resp.images[0].slug, Some("ubuntu-20-04-x64".to_string()));
    assert_eq!(resp.images[1].slug, None);
}

#[test]
fn test_deserialize_ssh_key_response() {
    let json = r#"{
        "ssh_key": {
            "id": 512189,
            "name": "dc-test-contract",
            "fingerprint": "3b:16:bf:e4:8b:00:8b:b8:59:8c:a1:09:41:3b:3e:5e"
        }
    }"#;

    let resp: SshKeyResponse = serde_json::from_str(json).expect("Failed to deserialize SSH key");
    assert_eq!(resp.ssh_key.id, 512189);
    assert_eq!(resp.ssh_key.name, "dc-test-contract");
}

#[test]
fn test_deserialize_action_response() {
    let json = r#"{
        "action": {
            "id": 36804636,
            "status": "in-progress",
            "type": "create"
        }
    }"#;

    let resp: DoActionResponse = serde_json::from_str(json).expect("Failed to deserialize action");
    assert_eq!(resp.action.id, 36804636);
    assert_eq!(resp.action.status, "in-progress");
    assert_eq!(resp.action.action_type, "create");
}

#[test]
fn test_droplet_name_format() {
    assert_eq!(droplet_name("abc123"), "dc-abc123");
    assert_eq!(droplet_name("test-contract-456"), "dc-test-contract-456");
}

#[test]
fn test_extract_contract_id() {
    assert_eq!(extract_contract_id("dc-abc123"), Some("abc123".to_string()));
    assert_eq!(extract_contract_id("dc-test-contract"), Some("test-contract".to_string()));
    assert_eq!(extract_contract_id("other-name"), None);
}

#[test]
fn test_droplet_network_extraction_no_networks() {
    let droplet = Droplet {
        id: 1,
        name: "test".to_string(),
        status: "active".to_string(),
        memory: 1024,
        vcpus: 1,
        disk: 25,
        locked: false,
        created_at: "2020-07-21T18:37:44Z".to_string(),
        networks: Networks::default(),
        region: DoRegion { name: "test".to_string(), slug: "nyc3".to_string() },
        size_slug: "s-1vcpu-1gb".to_string(),
        tags: vec![],
        image: None,
        features: vec![],
    };
    assert!(droplet.public_ipv4().is_none());
    assert!(droplet.public_ipv6().is_none());
}

#[test]
fn test_droplet_multiple_v4_picks_public() {
    let droplet = Droplet {
        id: 1,
        name: "test".to_string(),
        status: "active".to_string(),
        memory: 1024,
        vcpus: 1,
        disk: 25,
        locked: false,
        created_at: "2020-07-21T18:37:44Z".to_string(),
        networks: Networks {
            v4: vec![
                NetworkV4 {
                    ip_address: "10.0.0.1".to_string(),
                    netmask: "255.255.0.0".to_string(),
                    gateway: "".to_string(),
                    network_type: "private".to_string(),
                },
                NetworkV4 {
                    ip_address: "203.0.113.1".to_string(),
                    netmask: "255.255.255.0".to_string(),
                    gateway: "203.0.113.1".to_string(),
                    network_type: "public".to_string(),
                },
            ],
            v6: vec![],
        },
        region: DoRegion { name: "test".to_string(), slug: "nyc3".to_string() },
        size_slug: "s-1vcpu-1gb".to_string(),
        tags: vec![],
        image: None,
        features: vec![],
    };
    assert_eq!(droplet.public_ipv4().as_deref(), Some("203.0.113.1"));
}

#[test]
fn test_droplet_to_instance_mapping() {
    let config = DigitalOceanConfig {
        api_token: "test-token".to_string(),
        default_size: "s-1vcpu-1gb".to_string(),
        default_region: "nyc3".to_string(),
        default_image: "ubuntu-24-04-x64".to_string(),
    };
    let provisioner = DigitalOceanProvisioner::new(config).unwrap();

    let droplet = Droplet {
        id: 12345,
        name: "dc-test-contract".to_string(),
        status: "active".to_string(),
        memory: 2048,
        vcpus: 2,
        disk: 50,
        locked: false,
        created_at: "2020-07-21T18:37:44Z".to_string(),
        networks: Networks {
            v4: vec![NetworkV4 {
                ip_address: "203.0.113.50".to_string(),
                netmask: "255.255.255.0".to_string(),
                gateway: "203.0.113.1".to_string(),
                network_type: "public".to_string(),
            }],
            v6: vec![NetworkV6 {
                ip_address: "2604:a880:0:1010::18a:a001".to_string(),
                netmask: 64,
                gateway: "2604:a880:0:1010::1".to_string(),
                network_type: "public".to_string(),
            }],
        },
        region: DoRegion { name: "Amsterdam 3".to_string(), slug: "ams3".to_string() },
        size_slug: "s-2vcpu-2gb".to_string(),
        tags: vec!["dc-agent".to_string()],
        image: Some(DoImage {
            id: 63663980,
            name: "20.04 (LTS) x64".to_string(),
            slug: Some("ubuntu-20-04-x64".to_string()),
            distribution: "Ubuntu".to_string(),
        }),
        features: vec![],
    };

    let instance = provisioner.droplet_to_instance(&droplet);
    assert_eq!(instance.external_id, "12345");
    assert_eq!(instance.ip_address.as_deref(), Some("203.0.113.50"));
    assert_eq!(instance.public_ip.as_deref(), Some("203.0.113.50"));
    assert_eq!(instance.ipv6_address.as_deref(), Some("2604:a880:0:1010::18a:a001"));
    assert_eq!(instance.ssh_port, 22);
    assert!(instance.root_password.is_none());

    let details = instance.additional_details.unwrap();
    assert_eq!(details["size_slug"], "s-2vcpu-2gb");
    assert_eq!(details["region"], "ams3");
    assert_eq!(details["vcpus"], 2);
}

// ── Integration test (requires DIGITALOCEAN_API_TOKEN env var) ──────────────
// Run with: cargo nextest run -p dc-agent digitalocean --run-ignored ignored-only

#[tokio::test]
#[ignore]
async fn integration_list_droplets() {
    let token = std::env::var("DIGITALOCEAN_API_TOKEN")
        .expect("DIGITALOCEAN_API_TOKEN env var required for integration test");

    let config = DigitalOceanConfig {
        api_token: token,
        default_size: "s-1vcpu-1gb".to_string(),
        default_region: "nyc3".to_string(),
        default_image: "ubuntu-24-04-x64".to_string(),
    };
    let provisioner = DigitalOceanProvisioner::new(config).unwrap();

    let verification = provisioner.verify_setup().await;
    assert!(verification.api_reachable == Some(true), "API should be reachable: {:?}", verification.errors);

    let instances = provisioner.list_running_instances().await.expect("list should succeed");
    println!("Found {} running instances", instances.len());
    for inst in &instances {
        println!("  instance: external_id={}, contract_id={:?}", inst.external_id, inst.contract_id);
    }
}

#[tokio::test]
#[ignore]
async fn integration_catalog_endpoints() {
    use reqwest::Client;

    let token = std::env::var("DIGITALOCEAN_API_TOKEN")
        .expect("DIGITALOCEAN_API_TOKEN env var required for integration test");

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    // GET /v2/regions
    let resp = client
        .get("https://api.digitalocean.com/v2/regions")
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("regions request failed");
    assert!(resp.status().is_success(), "regions: status={}", resp.status());
    let regions: RegionsResponse = resp.json().await.expect("regions parse failed");
    println!("Regions: {} available", regions.regions.iter().filter(|r| r.available).count());
    for r in regions.regions.iter().take(5) {
        println!("  {} ({}) available={}", r.slug, r.name, r.available);
    }

    // GET /v2/sizes
    let resp = client
        .get("https://api.digitalocean.com/v2/sizes")
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("sizes request failed");
    assert!(resp.status().is_success(), "sizes: status={}", resp.status());
    let sizes: SizesResponse = resp.json().await.expect("sizes parse failed");
    println!("Sizes: {} available", sizes.sizes.iter().filter(|s| s.available).count());
    for s in sizes.sizes.iter().filter(|s| s.available).take(5) {
        println!("  {} ({}MB, {}vCPU) ${}/mo", s.slug, s.memory, s.vcpus, s.price_monthly);
    }

    // GET /v2/images?type=distribution
    let resp = client
        .get("https://api.digitalocean.com/v2/images?type=distribution")
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("images request failed");
    assert!(resp.status().is_success(), "images: status={}", resp.status());
    let images: ImagesResponse = resp.json().await.expect("images parse failed");
    println!("Distribution images: {}", images.images.len());
    for img in images.images.iter().take(5) {
        println!("  {} ({}) slug={:?}", img.name, img.distribution, img.slug);
    }
}
