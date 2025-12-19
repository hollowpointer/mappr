use crate::domain::models::system::{FirewallStatus, IpServiceGroup};
use crate::ports::outbound::system_repository::SystemRepository;
use pnet::datalink::NetworkInterface;

pub struct InfoService {
    system_repo: Box<dyn SystemRepository>,
}

pub struct SystemInfo {
    pub services: Vec<IpServiceGroup>,
    pub firewall: FirewallStatus,
    pub interfaces: Vec<NetworkInterface>,
}

impl InfoService {
    pub fn new(system_repo: Box<dyn SystemRepository>) -> Self {
        Self { system_repo }
    }

    pub fn get_system_info(&self) -> anyhow::Result<SystemInfo> {
        let services = self.system_repo.get_local_services()?;
        let firewall = self.system_repo.get_firewall_status()?;
        let interfaces = self.system_repo.get_network_interfaces()?;

        Ok(SystemInfo {
            services,
            firewall,
            interfaces,
        })
    }
}
