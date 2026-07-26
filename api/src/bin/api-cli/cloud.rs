//! Cloud subcommand: accounts/resources/catalog/marketplace (Hetzner/Proxmox).
use crate::api_cli::{Identity, SignedClient};
use crate::{AddCloudAccountRequest, CloudAccountResponse, ProvisionResourceRequest};
use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
#[derive(Subcommand)]
pub(crate) enum CloudAction {
    /// List cloud accounts
    ListAccounts {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Add a cloud account
    AddAccount {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Backend type (hetzner or proxmox_api)
        #[arg(long)]
        backend: String,
        /// Display name for the account
        #[arg(long)]
        name: String,
        /// Credentials (API token for Hetzner, JSON config for Proxmox)
        #[arg(long)]
        credentials: String,
    },
    /// Delete a cloud account
    DeleteAccount {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud account ID (UUID)
        #[arg(long)]
        id: String,
    },
    /// Show available server types, locations, and images
    Catalog {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud account ID (UUID)
        #[arg(long)]
        account_id: String,
    },
    /// List cloud resources
    ListResources {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Provision a new cloud resource (VM)
    Provision {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud account ID (UUID)
        #[arg(long)]
        account_id: String,
        /// VM name
        #[arg(long)]
        name: String,
        /// Server type (e.g., cx22)
        #[arg(long)]
        server_type: String,
        /// Location (e.g., fsn1)
        #[arg(long)]
        location: String,
        /// OS image (e.g., ubuntu-24.04)
        #[arg(long)]
        image: String,
        /// SSH public key for VM access
        #[arg(long)]
        ssh_pubkey: String,
    },
    /// Delete a cloud resource
    DeleteResource {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud resource ID (UUID)
        #[arg(long)]
        id: String,
    },
    /// Start a stopped cloud resource (VM)
    Start {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud resource ID (UUID)
        #[arg(long)]
        id: String,
    },
    /// Stop a running cloud resource (VM)
    Stop {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud resource ID (UUID)
        #[arg(long)]
        id: String,
    },
    /// Re-validate cloud account credentials
    ValidateAccount {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud account ID (UUID)
        #[arg(long)]
        id: String,
    },
    /// List a running resource on the marketplace
    ListOnMarketplace {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud resource ID (UUID)
        #[arg(long)]
        resource_id: String,
        /// Monthly price in USD
        #[arg(long)]
        price: f64,
        /// Offering name (defaults to resource name)
        #[arg(long)]
        name: Option<String>,
        /// Offering description
        #[arg(long)]
        description: Option<String>,
    },
    /// Unlist a resource from the marketplace
    UnlistFromMarketplace {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud resource ID (UUID)
        #[arg(long)]
        resource_id: String,
    },
}
// =============================================================================
// Cloud handlers
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudAccountListResponse {
    accounts: Vec<CloudAccountResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CloudResourceResponse {
    id: String,
    name: String,
    server_type: String,
    location: String,
    image: String,
    status: String,
    public_ip: Option<String>,
    cloud_account_name: String,
    cloud_account_backend: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudResourceListResponse {
    resources: Vec<CloudResourceResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogServerType {
    id: String,
    name: String,
    cores: u32,
    memory_gb: f64,
    disk_gb: u32,
    price_monthly: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogLocation {
    id: String,
    name: String,
    city: String,
    country: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogImage {
    id: String,
    name: String,
    os_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogResponse {
    server_types: Vec<CatalogServerType>,
    locations: Vec<CatalogLocation>,
    images: Vec<CatalogImage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListOnMarketplaceRequest {
    offer_name: String,
    monthly_price: f64,
    description: Option<String>,
}

pub(crate) async fn handle_cloud_action(action: CloudAction, api_url: &str) -> Result<()> {
    match action {
        CloudAction::ListAccounts { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let resp: CloudAccountListResponse = client.get_api("/cloud-accounts").await?;

            if resp.accounts.is_empty() {
                println!("No cloud accounts found.");
            } else {
                println!("\nCloud Accounts:");
                println!("{}", "=".repeat(110));
                println!(
                    "{:<38} {:<20} {:<12} {:<8} {:<20}",
                    "ID", "Name", "Backend", "Valid?", "Created"
                );
                println!("{}", "-".repeat(110));
                for a in &resp.accounts {
                    let valid = if a.is_valid { "yes" } else { "NO" };
                    let created = &a.created_at[..a.created_at.len().min(19)];
                    println!(
                        "{:<38} {:<20} {:<12} {:<8} {:<20}",
                        a.id,
                        &a.name[..a.name.len().min(18)],
                        a.backend_type,
                        valid,
                        created
                    );
                    if let Some(err) = &a.validation_error {
                        println!("  Error: {}", err);
                    }
                }
                println!("{}", "=".repeat(110));
                println!("Total: {} account(s)", resp.accounts.len());
            }
        }
        CloudAction::AddAccount {
            identity,
            backend,
            name,
            credentials,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = AddCloudAccountRequest {
                backend_type: backend,
                name: name.clone(),
                credentials,
            };

            let account: CloudAccountResponse =
                client.post_api("/cloud-accounts", &request).await?;
            println!("Cloud account created:");
            println!("  ID: {}", account.id);
            println!("  Name: {}", account.name);
            println!("  Backend: {}", account.backend_type);
            println!("  Valid: {}", account.is_valid);
        }
        CloudAction::DeleteAccount {
            identity,
            id: account_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-accounts/{}", account_id);
            let _: serde_json::Value = client.delete_api(&path).await?;
            println!("Cloud account {} deleted.", account_id);
        }
        CloudAction::Catalog {
            identity,
            account_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-accounts/{}/catalog", account_id);
            let catalog: CatalogResponse = client.get_api(&path).await?;

            // Server types
            println!("\nServer Types:");
            println!("{}", "=".repeat(90));
            println!(
                "{:<12} {:<25} {:<8} {:<12} {:<10} {:<12}",
                "ID", "Name", "Cores", "Memory GB", "Disk GB", "Price/mo"
            );
            println!("{}", "-".repeat(90));
            for st in &catalog.server_types {
                let price = st
                    .price_monthly
                    .map(|p| format!("${:.2}", p))
                    .unwrap_or_else(|| "N/A".to_string());
                println!(
                    "{:<12} {:<25} {:<8} {:<12.1} {:<10} {:<12}",
                    st.id,
                    &st.name[..st.name.len().min(23)],
                    st.cores,
                    st.memory_gb,
                    st.disk_gb,
                    price
                );
            }
            println!("{}", "=".repeat(90));
            println!("Total: {} server type(s)", catalog.server_types.len());

            // Locations
            println!("\nLocations:");
            println!("{}", "=".repeat(70));
            println!(
                "{:<12} {:<20} {:<20} {:<15}",
                "ID", "Name", "City", "Country"
            );
            println!("{}", "-".repeat(70));
            for loc in &catalog.locations {
                println!(
                    "{:<12} {:<20} {:<20} {:<15}",
                    loc.id, loc.name, loc.city, loc.country
                );
            }
            println!("{}", "=".repeat(70));
            println!("Total: {} location(s)", catalog.locations.len());

            // Images
            println!("\nImages:");
            println!("{}", "=".repeat(60));
            println!("{:<25} {:<20} {:<12}", "ID", "Name", "OS Type");
            println!("{}", "-".repeat(60));
            for img in &catalog.images {
                println!(
                    "{:<25} {:<20} {:<12}",
                    &img.id[..img.id.len().min(23)],
                    &img.name[..img.name.len().min(18)],
                    img.os_type
                );
            }
            println!("{}", "=".repeat(60));
            println!("Total: {} image(s)", catalog.images.len());
        }
        CloudAction::ListResources { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let resp: CloudResourceListResponse = client.get_api("/cloud-resources").await?;

            if resp.resources.is_empty() {
                println!("No cloud resources found.");
            } else {
                println!("\nCloud Resources:");
                println!("{}", "=".repeat(130));
                println!(
                    "{:<38} {:<15} {:<12} {:<16} {:<12} {:<15} {:<12}",
                    "ID", "Name", "Status", "IP", "Type", "Account", "Backend"
                );
                println!("{}", "-".repeat(130));
                for r in &resp.resources {
                    let ip = r.public_ip.as_deref().unwrap_or("N/A");
                    println!(
                        "{:<38} {:<15} {:<12} {:<16} {:<12} {:<15} {:<12}",
                        r.id,
                        &r.name[..r.name.len().min(13)],
                        r.status,
                        ip,
                        r.server_type,
                        &r.cloud_account_name[..r.cloud_account_name.len().min(13)],
                        r.cloud_account_backend
                    );
                }
                println!("{}", "=".repeat(130));
                println!("Total: {} resource(s)", resp.resources.len());
            }
        }
        CloudAction::Provision {
            identity,
            account_id,
            name,
            server_type,
            location,
            image,
            ssh_pubkey,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = ProvisionResourceRequest {
                cloud_account_id: account_id,
                name,
                server_type,
                location,
                image,
                ssh_pubkey,
            };

            let resource: serde_json::Value = client.post_api("/cloud-resources", &request).await?;
            println!("Cloud resource provisioning started:");
            println!("  ID: {}", resource["id"].as_str().unwrap_or("N/A"));
            println!("  Name: {}", resource["name"].as_str().unwrap_or("N/A"));
            println!("  Status: {}", resource["status"].as_str().unwrap_or("N/A"));
        }
        CloudAction::DeleteResource {
            identity,
            id: resource_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-resources/{}", resource_id);
            let _: serde_json::Value = client.delete_api(&path).await?;
            println!("Cloud resource {} deleted.", resource_id);
        }
        CloudAction::Start {
            identity,
            id: resource_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-resources/{}/start", resource_id);
            let _: serde_json::Value = client.post_api(&path, &()).await?;
            println!("Cloud resource {} started.", resource_id);
        }
        CloudAction::Stop {
            identity,
            id: resource_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-resources/{}/stop", resource_id);
            let _: serde_json::Value = client.post_api(&path, &()).await?;
            println!("Cloud resource {} stopped.", resource_id);
        }
        CloudAction::ValidateAccount {
            identity,
            id: account_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-accounts/{}/validate", account_id);
            let account: CloudAccountResponse = client.post_api(&path, &()).await?;
            println!("Cloud account validated:");
            println!("  ID: {}", account.id);
            println!("  Name: {}", account.name);
            println!("  Valid: {}", account.is_valid);
            if let Some(err) = &account.validation_error {
                println!("  Error: {}", err);
            }
        }
        CloudAction::ListOnMarketplace {
            identity,
            resource_id,
            price,
            name,
            description,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = ListOnMarketplaceRequest {
                offer_name: name
                    .unwrap_or_else(|| format!("VM {}", &resource_id[..8.min(resource_id.len())])),
                monthly_price: price,
                description,
            };

            let path = format!("/cloud-resources/{}/list-on-marketplace", resource_id);
            let offering: serde_json::Value = client.post_api(&path, &request).await?;
            println!("Resource listed on marketplace:");
            println!("{}", serde_json::to_string_pretty(&offering)?);
        }
        CloudAction::UnlistFromMarketplace {
            identity,
            resource_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-resources/{}/unlist-from-marketplace", resource_id);
            let _: serde_json::Value = client.post_api(&path, &()).await?;
            println!("Resource {} unlisted from marketplace.", resource_id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod cloud_tests {
    use super::*;

    #[test]
    fn test_add_cloud_account_request_serialization() {
        let req = AddCloudAccountRequest {
            backend_type: "hetzner".to_string(),
            name: "My Account".to_string(),
            credentials: "secret-token".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["backendType"], "hetzner");
        assert_eq!(json["name"], "My Account");
        assert_eq!(json["credentials"], "secret-token");
        // Must NOT have snake_case keys
        assert!(json.get("backend_type").is_none());
    }

    #[test]
    fn test_provision_resource_request_serialization() {
        let req = ProvisionResourceRequest {
            cloud_account_id: "uuid-123".to_string(),
            name: "my-vm".to_string(),
            server_type: "cx22".to_string(),
            location: "fsn1".to_string(),
            image: "ubuntu-24.04".to_string(),
            ssh_pubkey: "ssh-ed25519 AAAA...".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["cloudAccountId"], "uuid-123");
        assert_eq!(json["serverType"], "cx22");
        assert_eq!(json["sshPubkey"], "ssh-ed25519 AAAA...");
        // Must NOT have snake_case keys
        assert!(json.get("cloud_account_id").is_none());
        assert!(json.get("server_type").is_none());
        assert!(json.get("ssh_pubkey").is_none());
    }

    #[test]
    fn test_cloud_account_response_deserialization() {
        let json = r#"{
            "id": "abc-123",
            "backendType": "hetzner",
            "name": "Test Account",
            "isValid": true,
            "validationError": null,
            "createdAt": "2026-01-15T10:00:00Z"
        }"#;
        let account: CloudAccountResponse = serde_json::from_str(json).unwrap();
        assert_eq!(account.id, "abc-123");
        assert_eq!(account.backend_type, "hetzner");
        assert!(account.is_valid);
        assert!(account.validation_error.is_none());
    }

    #[test]
    fn test_cloud_account_response_with_validation_error() {
        let json = r#"{
            "id": "abc-123",
            "backendType": "hetzner",
            "name": "Bad Account",
            "isValid": false,
            "validationError": "Invalid API token",
            "createdAt": "2026-01-15T10:00:00Z"
        }"#;
        let account: CloudAccountResponse = serde_json::from_str(json).unwrap();
        assert!(!account.is_valid);
        assert_eq!(
            account.validation_error.as_deref(),
            Some("Invalid API token")
        );
    }

    #[test]
    fn test_cloud_account_list_response_deserialization() {
        let json = r#"{"accounts": []}"#;
        let resp: CloudAccountListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.accounts.is_empty());
    }

    #[test]
    fn test_cloud_resource_response_deserialization() {
        let json = r#"{
            "id": "res-456",
            "name": "my-vm",
            "serverType": "cx22",
            "location": "fsn1",
            "image": "ubuntu-24.04",
            "status": "running",
            "publicIp": "203.0.113.5",
            "cloudAccountName": "My Hetzner",
            "cloudAccountBackend": "hetzner",
            "createdAt": "2026-01-15T12:00:00Z"
        }"#;
        let resource: CloudResourceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resource.id, "res-456");
        assert_eq!(resource.status, "running");
        assert_eq!(resource.public_ip.as_deref(), Some("203.0.113.5"));
        assert_eq!(resource.server_type, "cx22");
    }

    #[test]
    fn test_cloud_resource_response_no_ip() {
        let json = r#"{
            "id": "res-789",
            "name": "pending-vm",
            "serverType": "cx22",
            "location": "fsn1",
            "image": "ubuntu-24.04",
            "status": "provisioning",
            "publicIp": null,
            "cloudAccountName": "My Hetzner",
            "cloudAccountBackend": "hetzner",
            "createdAt": "2026-01-15T12:00:00Z"
        }"#;
        let resource: CloudResourceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resource.status, "provisioning");
        assert!(resource.public_ip.is_none());
    }

    #[test]
    fn test_catalog_response_deserialization() {
        let json = r#"{
            "serverTypes": [{
                "id": "cx22",
                "name": "CX22",
                "cores": 2,
                "memoryGb": 4.0,
                "diskGb": 40,
                "priceMonthly": 3.92
            }],
            "locations": [{
                "id": "fsn1",
                "name": "Falkenstein",
                "city": "Falkenstein",
                "country": "DE"
            }],
            "images": [{
                "id": "161547269",
                "name": "ubuntu-24.04",
                "osType": "ubuntu"
            }]
        }"#;
        let catalog: CatalogResponse = serde_json::from_str(json).unwrap();
        assert_eq!(catalog.server_types.len(), 1);
        assert_eq!(catalog.server_types[0].cores, 2);
        assert_eq!(catalog.server_types[0].price_monthly, Some(3.92));
        assert_eq!(catalog.locations.len(), 1);
        assert_eq!(catalog.locations[0].country, "DE");
        assert_eq!(catalog.images.len(), 1);
        assert_eq!(catalog.images[0].os_type, "ubuntu");
    }

    #[test]
    fn test_cloud_resource_list_response_deserialization() {
        let json = r#"{"resources": []}"#;
        let resp: CloudResourceListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.resources.is_empty());
    }
}
