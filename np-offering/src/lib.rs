use np_json_search::value_matches_with_parents;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml_ng::{self, Value as YamlValue};
use std::collections::HashMap;

// Define the Offering enum with version-specific variants
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Offering {
    V0_1_0(CloudProviderOfferingV0_1_0),
    // Future versions can be added here
}

// Main struct for Cloud Provider Offering version 0.1.0
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloudProviderOfferingV0_1_0 {
    pub kind: String,
    pub metadata: Metadata,
    pub api_version: String,
    pub provider: Provider,
    pub defaults: Option<DefaultSpec>,
    pub regions: Vec<Region>,

    // Raw JsonValue representation, for use in matches_search
    #[serde(skip)]
    orig_json_value: JsonValue,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provider {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefaultSpec {
    pub compliance: Option<Vec<String>>,
    pub sla: Option<SLA>,
    pub machine_spec: Option<MachineSpec>,
    pub network_spec: Option<NetworkSpec>,
    pub security: Option<Security>,
    pub monitoring: Option<Monitoring>,
    pub backup: Option<Backup>,
    pub cost_optimization: Option<CostOptimization>,
    pub service_integrations: Option<ServiceIntegrations>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SLA {
    pub uptime: Option<String>,
    pub measurement_period: Option<String>,
    pub support: Option<Support>,
    pub downtime_compensation: Option<Vec<DowntimeCompensation>>,
    pub maintenance: Option<Maintenance>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Support {
    pub levels: Option<Vec<String>>,
    pub response_time: Option<ResponseTime>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ResponseTime {
    pub critical: Option<String>,
    pub high: Option<String>,
    pub medium: Option<String>,
    pub low: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DowntimeCompensation {
    pub less_than: Option<String>,
    pub more_than: Option<String>,
    pub credit_percentage: Option<u8>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Maintenance {
    pub window: Option<String>,
    pub notification_period: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MachineSpec {
    pub instance_types: Vec<InstanceType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceType {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub description: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<GPU>,
    pub memory: Option<String>,
    pub storage: Option<Storage>,
    pub network: Option<NetworkSpecDetails>,
    pub pricing: Option<HashMap<String, HashMap<String, String>>>,
    pub compliance: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<MetadataSpec>,
    pub ai_spec: Option<AISpec>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GPU {
    pub count: u32,
    #[serde(rename = "type")]
    pub type_: String,
    pub memory: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Storage {
    #[serde(rename = "type")]
    pub type_: String,
    pub size: String,
    pub iops: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataSpec {
    pub optimized_for: Option<String>,
    pub availability: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkSpecDetails {
    pub bandwidth: Option<String>,
    pub latency: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AISpec {
    pub framework_optimizations: Option<Vec<String>>,
    pub software_stack: Option<SoftwareStack>,
    pub enhanced_networking: Option<bool>,
    pub distributed_training_support: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SoftwareStack {
    pub preinstalled: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkSpec {
    pub vpc_support: Option<bool>,
    pub public_ip: Option<bool>,
    pub private_ip: Option<bool>,
    pub load_balancers: Option<LoadBalancers>,
    pub firewalls: Option<Firewalls>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadBalancers {
    #[serde(rename = "type")]
    pub types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Firewalls {
    pub stateful: Option<bool>,
    pub stateless: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Security {
    pub data_encryption: Option<DataEncryption>,
    pub identity_and_access_management: Option<IAM>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataEncryption {
    pub at_rest: Option<String>,
    pub in_transit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IAM {
    pub multi_factor_authentication: Option<bool>,
    pub role_based_access_control: Option<bool>,
    pub single_sign_on: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Monitoring {
    pub enabled: Option<bool>,
    pub metrics: Option<Metrics>,
    pub logging: Option<Logging>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Metrics {
    pub cpu_utilization: Option<bool>,
    pub memory_usage: Option<bool>,
    pub disk_iops: Option<bool>,
    pub network_traffic: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Logging {
    pub enabled: Option<bool>,
    pub log_retention: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Backup {
    pub enabled: Option<bool>,
    pub frequency: Option<String>,
    pub retention: Option<String>,
    pub disaster_recovery: Option<DisasterRecovery>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisasterRecovery {
    pub cross_region_replication: Option<bool>,
    pub failover_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostOptimization {
    pub spot_instances_available: Option<bool>,
    pub savings_plans: Option<Vec<SavingsPlan>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavingsPlan {
    #[serde(rename = "type")]
    pub type_: String,
    pub discount: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceIntegrations {
    pub databases: Option<Vec<String>>,
    pub storage_services: Option<Vec<String>>,
    pub messaging_services: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub name: String,
    pub description: Option<String>,
    pub geography: Option<Geography>,
    pub compliance: Option<Vec<String>>,
    pub machine_spec: Option<MachineSpec>,
    pub availability_zones: Option<Vec<AvailabilityZone>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Geography {
    pub continent: Option<String>,
    pub country: Option<String>,
    pub iso_codes: Option<IsoCodes>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IsoCodes {
    pub country_code: Option<String>,
    pub region_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AvailabilityZone {
    pub name: String,
    pub description: Option<String>,
}

impl Offering {
    pub fn new_from_file(path: &str) -> Result<Self, String> {
        let input =
            fs_err::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        if path.to_lowercase().ends_with(".json") {
            Self::new_from_str(&input, "json")
        } else if path.to_lowercase().ends_with(".yaml") {
            Self::new_from_str(&input, "yaml")
        } else {
            Err("Unsupported file format. Use '.json' or '.yaml'.".to_string())
        }
    }

    pub fn new_from_bytes(input: &[u8], format: &str) -> Result<Self, String> {
        let input = std::str::from_utf8(input)
            .map_err(|e| format!("Failed to convert bytes to UTF8 string: {}", e))?;
        Self::new_from_str(input, format)
    }

    pub fn new_from_str(input: &str, format: &str) -> Result<Self, String> {
        let doc: JsonValue = match format {
            "yaml" => {
                let yaml_value: YamlValue = serde_yaml_ng::from_str(input)
                    .map_err(|e| format!("Failed to parse YAML: {}", e))?;
                serde_json::to_value(yaml_value)
                    .map_err(|e| format!("Failed to convert YAML to JSON value: {}", e))?
            }
            "json" => {
                serde_json::from_str(input).map_err(|e| format!("Failed to parse JSON: {}", e))?
            }
            _ => return Err("Unsupported format. Use 'yaml' or 'json'.".to_string()),
        };

        match doc.get("api_version").and_then(|v| v.as_str()) {
            Some("v0.1.0") => {
                // TODO: add schema validation
                serde_json::from_value::<CloudProviderOfferingV0_1_0>(doc.clone())
                .map(|o| CloudProviderOfferingV0_1_0 {
                    orig_json_value: doc,
                    ..o
                })
                    .map(Offering::V0_1_0)
                    .map_err(|e| {
                        format!("Failed to deserialize CloudProviderOfferingV0_1_0: (err {}) from input doc {}", e, input)
                    })
            }
            Some(version) => Err(format!(
                "Unsupported api_version '{}' in the input doc {}",
                version, input
            )),
            None => Err(format!(
                "Missing 'api_version' field in the input doc {}",
                input
            )),
        }
    }

    pub fn matches_search(&self, search_str: &str) -> Vec<String> {
        match self {
            Offering::V0_1_0(offering) => offering.matches_search(search_str),
        }
    }

    pub fn get_instance_with_id(&self, id: &str) -> Option<InstanceType> {
        match self {
            Offering::V0_1_0(offering) => offering.get_instance_with_id(id),
        }
    }

    pub fn json_value(&self) -> &serde_json::Value {
        match self {
            Offering::V0_1_0(offering) => &offering.orig_json_value,
        }
    }

    pub fn as_json_string(&self) -> Result<String, String> {
        serde_json::to_string(&self.json_value()).map_err(|e| e.to_string())
    }

    pub fn as_json_string_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.json_value()).map_err(|e| e.to_string())
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&self.json_value()).map_err(|e| e.to_string())
    }

    pub fn deserialize(data: &[u8]) -> Result<Offering, String> {
        serde_json::from_slice(data).map_err(|e| e.to_string())
    }

    pub fn instance_pricing(&self, instance_id: &str) -> HashMap<String, HashMap<String, String>> {
        self.get_instance_with_id(instance_id)
            .and_then(|instance_type| instance_type.pricing)
            .unwrap_or_default()
    }
}

impl CloudProviderOfferingV0_1_0 {
    pub fn matches_search(&self, search_str: &str) -> Vec<String> {
        value_matches_with_parents(&self.orig_json_value, "instance_types.id", search_str)
    }

    pub fn get_instance_with_id(&self, id: &str) -> Option<InstanceType> {
        for region in &self.regions {
            if let Some(machine_spec) = &region.machine_spec {
                for instance_type in &machine_spec.instance_types {
                    if instance_type.id == id {
                        return Some(instance_type.clone());
                    }
                }
            }
        }
        if let Some(default_spec) = &self.defaults {
            if let Some(machine_spec) = &default_spec.machine_spec {
                for instance_type in &machine_spec.instance_types {
                    if instance_type.id == id {
                        return Some(instance_type.clone());
                    }
                }
            }
        }
        None
    }
}

impl std::fmt::Display for Offering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Offering::V0_1_0(offering) => write!(f, "{}", offering),
        }
    }
}

impl std::fmt::Display for CloudProviderOfferingV0_1_0 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_yaml_ng::to_string(self) {
            Ok(yaml_str) => write!(f, "{}", yaml_str),
            Err(e) => {
                write!(f, "Failed to format CloudProviderOfferingV0_1_0: {}", e)?;
                Err(std::fmt::Error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
api_version: v0.1.0
kind: cloud_provider_offering
metadata:
  name: "GenericCloudService"
  version: "1.0"
provider:
  name: generic cloud provider
  description: a generic offering specification for a cloud provider
defaults:
  compliance:
    - ISO 27001
    - SOC 2
  machine_spec:
    instance_types:
      - id: xxx-small
        type: general-purpose
        cpu: 2 vCPUs
        memory: 2 GB
        storage:
          type: SSD
          size: 50 GB
        pricing:
          on_demand:
            hour: "0.05"
regions:
  - name: eu-central-1
    description: central europe region
    compliance:
      - GDPR
    machine_spec:
      instance_types:
        - id: xx-small
          type: memory-optimized
          cpu: 4 vCPUs
          memory: 16 GB
          storage:
            type: SSD
            size: 100 GB
          pricing:
            on_demand:
                hour: "0.15"
"#;

    #[test]
    fn test_parse_offering() {
        let offering = Offering::new_from_str(SAMPLE_YAML, "yaml").expect("Failed to parse YAML");
        match offering {
            Offering::V0_1_0(offering) => {
                assert_eq!(offering.metadata.name, "GenericCloudService");
                assert_eq!(offering.provider.name, "generic cloud provider");
                assert_eq!(offering.defaults.unwrap().compliance.unwrap().len(), 2);
                assert_eq!(offering.regions.len(), 1);
                assert_eq!(offering.regions[0].name, "eu-central-1");
            }
        }
    }

    #[test]
    fn test_search_offering() {
        let offering = Offering::new_from_str(SAMPLE_YAML, "yaml").expect("Failed to parse YAML");
        // Test matches_search with spaces before and after
        assert!(!offering
            .matches_search("name =GenericCloudService")
            .is_empty());
        assert!(!offering
            .matches_search("name=GenericCloudService")
            .is_empty());
        assert!(!offering
            .matches_search("name= GenericCloudService")
            .is_empty());

        assert!(!offering
            .matches_search("provider.name=generic cloud provider")
            .is_empty());
        assert!(!offering.matches_search("name contains Cloud").is_empty());
        assert!(!offering
            .matches_search("name contains CloudService")
            .is_empty());
        assert!(!offering.matches_search("name contains Service").is_empty());
        assert!(!offering
            .matches_search("name contains GenericCloudService")
            .is_empty());
        assert!(!offering
            .matches_search("name startswith GenericCloudService")
            .is_empty());
        assert!(!offering.matches_search("name endswith Service").is_empty());

        assert!(!offering.matches_search("type=memory-optimized").is_empty());
        assert!(!offering
            .matches_search("type=memory-optimized and name=GenericCloudService")
            .is_empty());
        assert!(!offering.matches_search("name endswith Service").is_empty());
        assert!(!offering
            .matches_search("regions.name=eu-central-1")
            .is_empty());
        assert!(!offering
            .matches_search("pricing.on_demand.hour <= 0.05")
            .is_empty());
        assert!(offering
            .matches_search("pricing.on_demand.hour < 0.05")
            .is_empty());
        assert!(offering
            .matches_search("pricing.on_demand.hour <= 0.01")
            .is_empty());
        assert!(!offering
            .matches_search("pricing.on_demand.hour >= 0.05")
            .is_empty());
        assert!(!offering
            .matches_search("pricing.on_demand.hour >= 0.15")
            .is_empty());
        assert!(offering
            .matches_search("pricing.on_demand.hour > 0.15")
            .is_empty());
        assert!(offering
            .matches_search("pricing.on_demand.hour > 0.5")
            .is_empty());
        assert!(!offering.matches_search("type=memory-optimized").is_empty());
        assert!(offering.matches_search("nonexistent=value").is_empty());
        println!(
            "{:#?}",
            offering.matches_search("memory > 1gb and storage.size > 1gb")
        );
        assert_eq!(
            offering
                .matches_search("memory > 1gb and storage.size > 1gb")
                .len(),
            2
        );
        assert_eq!(
            offering
                .matches_search("memory > 512MB AND storage.size > 1gb")
                .len(),
            2
        );
    }

    #[test]
    fn test_optional_fields() {
        let minimal_yaml = r#"
api_version: v0.1.0
kind: cloud_provider_offering
metadata:
  name: "MinimalCloudService"
  version: "1.0"
provider:
  name: minimal provider
regions:
  - name: us-east-1
"#;
        let offering =
            Offering::new_from_str(minimal_yaml, "yaml").expect("Failed to parse minimal YAML");
        match offering {
            Offering::V0_1_0(offering) => {
                assert_eq!(offering.metadata.name, "MinimalCloudService");
                assert_eq!(offering.provider.name, "minimal provider");
                assert!(offering.defaults.is_none());
                assert_eq!(offering.regions.len(), 1);
            }
        }
    }

    #[test]
    fn test_default_inheritance() {
        let offering = Offering::new_from_str(SAMPLE_YAML, "yaml").expect("Failed to parse YAML");
        match offering {
            Offering::V0_1_0(offering) => {
                let default_instance_types = &offering
                    .defaults
                    .unwrap()
                    .machine_spec
                    .unwrap()
                    .instance_types;
                let region_instance_types = &offering.regions[0]
                    .machine_spec
                    .as_ref()
                    .unwrap()
                    .instance_types;
                // Ensure defaults and region-specific instance types are different
                assert_ne!(
                    default_instance_types[0].type_,
                    region_instance_types[0].type_
                );
            }
        }
    }
}
