use np_yaml_search::yaml_value_matches;
use serde::{Deserialize, Serialize};
use std::fmt;

// Define the Offering enum with version-specific variants
#[derive(Debug, Serialize, Deserialize)]
pub enum Offering {
    V0_1_0(CloudProviderOfferingV0_1_0),
    // Future versions can be added here
}

// Main struct for Cloud Provider Offering version 0.1.0
#[derive(Debug, Serialize, Deserialize)]
pub struct CloudProviderOfferingV0_1_0 {
    pub kind: String,
    pub metadata: Metadata,
    pub provider: Provider,
    pub defaults: Option<DefaultSpec>,
    pub regions: Vec<Region>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SLA {
    pub uptime: Option<String>,
    pub measurement_period: Option<String>,
    pub support: Option<Support>,
    pub downtime_compensation: Option<Vec<Compensation>>,
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
pub struct Compensation {
    pub less_than: Option<String>,
    pub more_than: Option<String>,
    pub credit_percentage: Option<u8>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Maintenance {
    pub window: Option<String>,
    pub notification_period: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MachineSpec {
    pub instance_types: Vec<InstanceType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceType {
    #[serde(rename = "type")]
    pub type_: String,
    pub description: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<GPU>,
    pub memory: Option<String>,
    pub storage: Option<Storage>,
    pub network: Option<NetworkSpecDetails>,
    pub pricing: Option<Pricing>,
    pub compliance: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<MetadataSpec>,
    pub ai_spec: Option<AISpec>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pricing {
    pub on_demand: String,
    pub reserved: Option<ReservedPricing>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReservedPricing {
    pub one_year: String,
    pub three_year: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataSpec {
    pub optimized_for: Option<String>,
    pub availability: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkSpecDetails {
    pub bandwidth: Option<String>,
    pub latency: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AISpec {
    pub framework_optimizations: Option<Vec<String>>,
    pub software_stack: Option<SoftwareStack>,
    pub enhanced_networking: Option<bool>,
    pub distributed_training_support: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoftwareStack {
    pub preinstalled: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkSpec {
    pub vpc_support: Option<bool>,
    pub public_ip: Option<bool>,
    pub private_ip: Option<bool>,
    pub load_balancers: Option<LoadBalancers>,
    pub firewalls: Option<Firewalls>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadBalancers {
    #[serde(rename = "type")]
    pub types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Firewalls {
    pub stateful: Option<bool>,
    pub stateless: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Security {
    pub data_encryption: Option<DataEncryption>,
    pub identity_and_access_management: Option<IAM>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataEncryption {
    pub at_rest: Option<String>,
    pub in_transit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IAM {
    pub multi_factor_authentication: Option<bool>,
    pub role_based_access_control: Option<bool>,
    pub single_sign_on: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Monitoring {
    pub enabled: Option<bool>,
    pub metrics: Option<Metrics>,
    pub logging: Option<Logging>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metrics {
    pub cpu_utilization: Option<bool>,
    pub memory_usage: Option<bool>,
    pub disk_iops: Option<bool>,
    pub network_traffic: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Logging {
    pub enabled: Option<bool>,
    pub log_retention: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Backup {
    pub enabled: Option<bool>,
    pub frequency: Option<String>,
    pub retention: Option<String>,
    pub disaster_recovery: Option<DisasterRecovery>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisasterRecovery {
    pub cross_region_replication: Option<bool>,
    pub failover_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CostOptimization {
    pub spot_instances_available: Option<bool>,
    pub savings_plans: Option<Vec<SavingsPlan>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavingsPlan {
    #[serde(rename = "type")]
    pub type_: String,
    pub discount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceIntegrations {
    pub databases: Option<Vec<String>>,
    pub storage_services: Option<Vec<String>>,
    pub messaging_services: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub description: Option<String>,
    pub geography: Option<Geography>,
    pub compliance: Option<Vec<String>>,
    pub machine_spec: Option<MachineSpec>,
    pub availability_zones: Option<Vec<AvailabilityZone>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Geography {
    pub continent: Option<String>,
    pub country: Option<String>,
    pub iso_codes: Option<IsoCodes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsoCodes {
    pub country_code: Option<String>,
    pub region_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailabilityZone {
    pub name: String,
    pub description: Option<String>,
}

impl Offering {
    pub fn parse_as_yaml_value(yaml_str: &str) -> Result<serde_yaml_ng::Value, String> {
        serde_yaml_ng::from_str(yaml_str).map_err(|e| format!("Failed to parse YAML: {}", e))
    }

    // Function to parse the input YAML string into an Offering enum
    pub fn parse(yaml_str: &str) -> Result<Self, String> {
        let doc = Self::parse_as_yaml_value(yaml_str)?;

        let kind = doc["kind"]
            .as_str()
            .ok_or("Missing or invalid 'kind' field")?;
        if kind != "cloud_provider_offering" {
            return Err(format!("Unsupported kind '{}'", kind));
        }

        let api_version = doc["api_version"]
            .as_str()
            .ok_or("Missing or invalid 'api_version' field")?;
        match api_version {
            "v0.1.0" => Self::parse_v0_1_0(doc),
            _ => Err(format!("Unsupported api_version '{}'", api_version)),
        }
    }

    fn parse_v0_1_0(doc: serde_yaml_ng::Value) -> Result<Self, String> {
        serde_yaml_ng::from_value(doc)
            .map(Offering::V0_1_0)
            .map_err(|e| format!("Failed to deserialize CloudProviderOfferingV0_1_0: {}", e))
    }

    pub fn search(yaml_str: &str, search_str: &str) -> bool {
        let yaml_value = match Offering::parse_as_yaml_value(yaml_str) {
            Ok(yaml_value) => yaml_value,
            Err(e) => {
                println!("Failed to parse YAML: {}", e);
                return false;
            }
        };

        yaml_value_matches(&yaml_value, search_str)
    }
}

impl fmt::Display for Offering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Offering::V0_1_0(offering) => write!(f, "{}", offering),
            // Add future versions' display methods as needed
        }
    }
}

impl fmt::Display for CloudProviderOfferingV0_1_0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_yaml_ng::to_string(self) {
            Ok(yaml_str) => write!(f, "{}", yaml_str),
            Err(e) => {
                write!(f, "Failed to format CloudProviderOfferingV0_1_0: {}", e)?;
                Err(fmt::Error)
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
      - type: general-purpose
        cpu: 2 vCPUs
        memory: 2 GB
        storage:
          type: SSD
          size: 50 GB
        pricing:
          on_demand: "$0.05/hour"
regions:
  - name: eu-central-1
    description: central europe region
    compliance:
      - GDPR
    machine_spec:
      instance_types:
        - type: memory-optimized
          cpu: 4 vCPUs
          memory: 16 GB
          storage:
            type: SSD
            size: 100 GB
          pricing:
            on_demand: "$0.15/hour"
"#;

    #[test]
    fn test_parse_offering() {
        let offering = Offering::parse(SAMPLE_YAML).expect("Failed to parse YAML");
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
        assert!(Offering::search(SAMPLE_YAML, "name =GenericCloudService"));
        assert!(Offering::search(SAMPLE_YAML, "name=GenericCloudService"));
        assert!(Offering::search(SAMPLE_YAML, "name= GenericCloudService"));
        assert!(Offering::search(SAMPLE_YAML, "name contains Cloud"));
        assert!(Offering::search(SAMPLE_YAML, "name contains CloudService"));
        assert!(Offering::search(SAMPLE_YAML, "name contains Service"));
        assert!(Offering::search(
            SAMPLE_YAML,
            "name contains GenericCloudService"
        ));
        assert!(Offering::search(
            SAMPLE_YAML,
            "name startswith GenericCloudService"
        ));
        assert!(Offering::search(SAMPLE_YAML, "name endswith Service"));
        assert!(Offering::search(SAMPLE_YAML, "name=eu-central-1"));
        assert!(Offering::search(SAMPLE_YAML, "type=memory-optimized"));
        assert!(!Offering::search(SAMPLE_YAML, "nonexistent=value"));
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
        let offering = Offering::parse(minimal_yaml).expect("Failed to parse minimal YAML");
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
        let offering = Offering::parse(SAMPLE_YAML).expect("Failed to parse YAML");
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
