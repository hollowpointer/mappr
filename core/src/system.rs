use std::process::Command;

use anyhow;


use pnet::datalink::NetworkInterface;

use mappr_common::network::localhost::{IpServiceGroup, FirewallStatus};
use mappr_common::system::SystemRepository;

pub struct SystemRepo;

impl SystemRepository for SystemRepo {
    fn get_local_services(&self) -> anyhow::Result<Vec<IpServiceGroup>> {
        // TODO: Restore netstat2 usage. Currently disabled due to compilation error (libc mismatch).
        Ok(Vec::new())
    }

    fn get_firewall_status(&self) -> anyhow::Result<FirewallStatus> {
        #[cfg(target_os = "linux")]
        {
            let ufw_active = Command::new("ufw").arg("status").output().is_ok();
            let firewalld_active = Command::new("firewall-cmd").arg("--state").output().is_ok();

            if ufw_active || firewalld_active {
                Ok(FirewallStatus::Active)
            } else {
                Ok(FirewallStatus::NotDetected) 
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(FirewallStatus::NotDetected)
        }
    }

    fn get_network_interfaces(&self) -> anyhow::Result<Vec<NetworkInterface>> {
        crate::network::interface::get_prioritized_interfaces(10)
    }
}
