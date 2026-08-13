use std::collections::HashMap;

use anyhow::Result;
use tracing::{info, warn};

use crate::config::{Config, ProvisionerConfig};
use crate::provisioner::{
    digitalocean::DigitalOceanProvisioner, docker::DockerProvisioner, manual::ManualProvisioner,
    proxmox::ProxmoxProvisioner, script::ScriptProvisioner, Provisioner,
};

/// Map of provisioner type name to provisioner instance
pub type ProvisionerMap = HashMap<String, Box<dyn Provisioner>>;

/// Create a single provisioner from config
pub fn create_provisioner_from_config(prov_config: &ProvisionerConfig) -> Result<Box<dyn Provisioner>> {
    match prov_config {
        ProvisionerConfig::Proxmox(proxmox) => {
            info!("Creating Proxmox provisioner");
            Ok(Box::new(ProxmoxProvisioner::new(proxmox.clone())?))
        }
        ProvisionerConfig::Script(script) => {
            info!("Creating Script provisioner");
            Ok(Box::new(ScriptProvisioner::new(script.clone())))
        }
        ProvisionerConfig::Manual(manual) => {
            info!("Creating Manual provisioner");
            Ok(Box::new(ManualProvisioner::new(manual.clone())))
        }
        ProvisionerConfig::Docker(docker) => {
            info!("Creating Docker provisioner");
            Ok(Box::new(DockerProvisioner::new(docker.clone())?))
        }
        ProvisionerConfig::DigitalOcean(do_config) => {
            info!("Creating DigitalOcean provisioner");
            Ok(Box::new(DigitalOceanProvisioner::new(do_config.clone())?))
        }
    }
}

/// Create a map of all configured provisioners and return the default type
pub fn create_provisioner_map(config: &Config) -> Result<(ProvisionerMap, String)> {
    let mut map: ProvisionerMap = HashMap::new();

    // Add the default (required) provisioner
    let default_type = config.provisioner.type_name().to_string();
    let default_prov = create_provisioner_from_config(&config.provisioner)?;
    map.insert(default_type.clone(), default_prov);

    // Add any additional provisioners
    for additional in &config.additional_provisioners {
        let ptype = additional.type_name().to_string();
        if map.contains_key(&ptype) {
            warn!(
                provisioner_type = %ptype,
                "Duplicate provisioner type in additional_provisioners, skipping"
            );
            continue;
        }
        let prov = create_provisioner_from_config(additional)?;
        map.insert(ptype, prov);
    }

    Ok((map, default_type))
}
