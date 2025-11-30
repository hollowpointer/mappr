use std::{collections::HashSet, net::{IpAddr, Ipv4Addr, Ipv6Addr}};
use pnet::{datalink::NetworkInterface, ipnetwork::{Ipv4Network, Ipv6Network}, util::MacAddr};

use crate::net::datalink::interface::NetworkInterfaceExtension;

#[derive(Debug, Clone, Default)]
pub struct SenderConfig {
    local_mac: Option<MacAddr>,
    ipv4_net: Option<Ipv4Network>,
    ipv6_net: Option<Ipv6Network>,
    targets_v4: HashSet<Ipv4Addr>,
    targets_v6: HashSet<Ipv6Addr>,
}

impl From<&NetworkInterface> for SenderConfig {
    fn from(interface: &NetworkInterface) -> Self {
        Self {
            local_mac: interface.mac,
            ipv4_net: interface.get_ipv4_net(),
            ipv6_net: interface.get_ipv6_net(),
            targets_v4: HashSet::new(),
            targets_v6: HashSet::new(),
        }
    }
}

impl SenderConfig {
    pub fn _set_local_mac(&mut self, mac_addr: MacAddr) {
        self.local_mac = Some(mac_addr);
    }

    pub fn get_local_mac(&self) -> anyhow::Result<MacAddr> {
        self.local_mac
            .ok_or_else(|| anyhow::anyhow!("local MAC not set"))
    }

    pub fn get_ipv4_net(&self) -> anyhow::Result<Ipv4Network> {
        self.ipv4_net
            .ok_or_else(|| anyhow::anyhow!("ipv4net not set"))
    }

    pub fn get_link_local(&self) -> anyhow::Result<Ipv6Addr> {
        self.ipv6_net.iter()
            .find_map(|ipv6_net| {
                let ip = ipv6_net.ip();
                ip.is_unicast_link_local().then_some(ip)
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to find a link local address"))
    }

    pub fn get_targets_v4(&self) -> HashSet<Ipv4Addr> {
        self.targets_v4.clone()
    }

    pub fn add_target(&mut self, target_addr: IpAddr) {
        match target_addr {
            IpAddr::V4(ipv4_addr) => self.targets_v4.insert(ipv4_addr),
            IpAddr::V6(ipv6_addr) => self.targets_v6.insert(ipv6_addr),
        };
    }

    pub fn add_targets<T: IntoIterator<Item = IpAddr>>(&mut self, targets: T) {
        for target in targets {
            self.add_target(target);
        }
    }

    pub fn _target_count(&self) -> usize {
        self.targets_v4.len() + self.targets_v6.len()
    }

    pub fn _target_count_v4(&self) -> usize {
        self.targets_v4.len()
    }

    pub fn _target_count_v6(&self) -> usize {
        self.targets_v6.len()
    }

    pub fn has_addr(&self, target_addr: &IpAddr) -> bool {
        match target_addr {
            IpAddr::V4(ipv4_addr) => self.targets_v4.contains(ipv4_addr),
            IpAddr::V6(ipv6_addr) => self.targets_v6.contains(ipv6_addr),
        }
    }
}