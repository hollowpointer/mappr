//! # Local System Information Service
//!
//! Implements the "System Info" use case.
//!
//! This service acts as a facade for gathering local machine statistics and
//! configuration, useful for debugging or self-awareness context.

use crate::domain::models::localhost::{FirewallStatus, IpServiceGroup};
use crate::ports::outbound::system_repository::SystemRepository;
use pnet::datalink::NetworkInterface;

/// Application Service for Local System Information.
///
/// Responsible for gathering and aggregating information about the machine
/// where the application is running, such as:
/// * Network Interfaces (IPs, MACs).
/// * Active Services (Open ports).
/// * Firewall status.
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

    /// Retrieves a comprehensive snapshot of the local system's network state.
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
