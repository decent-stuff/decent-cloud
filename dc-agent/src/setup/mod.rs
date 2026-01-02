//! Setup wizard for configuring dc-agent with various provisioners.

pub mod gateway;
pub mod proxmox;

pub use gateway::GatewaySetup;
pub use proxmox::ProxmoxSetup;
