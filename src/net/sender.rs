use std::{collections::HashSet, net::{IpAddr, Ipv4Addr, Ipv6Addr}};
use pnet::{datalink::NetworkInterface, ipnetwork::{Ipv4Network, Ipv6Network}, util::MacAddr};

use crate::{net::{datalink::interface::NetworkInterfaceExtension, range::{self, Ipv4Range}}};

#[derive(Debug, Clone, Default)]
pub struct SenderConfig {
    local_mac: Option<MacAddr>,
    ipv4_nets: Vec<Ipv4Network>,
    ipv6_nets: Vec<Ipv6Network>,
    targets_v4: HashSet<Ipv4Addr>,
    targets_v6: HashSet<Ipv6Addr>,
}

impl From<&NetworkInterface> for SenderConfig {
    fn from(interface: &NetworkInterface) -> Self {
        Self {
            local_mac: interface.mac,
            ipv4_nets: interface.get_ipv4_nets(),
            ipv6_nets: interface.get_ipv6_nets(),
            targets_v4: HashSet::new(),
            targets_v6: HashSet::new(),
        }
    }
}

impl SenderConfig {
    pub fn get_local_mac(&self) -> anyhow::Result<MacAddr> {
        self.local_mac
            .ok_or_else(|| anyhow::anyhow!("local MAC not set"))
    }

    pub fn get_ipv4_net(&self) -> anyhow::Result<Ipv4Network> {
        let ipv4_net = self.ipv4_nets.first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No IPv4 networks available in configuration"))?;
        
        Ok(ipv4_net)
    }

    pub fn get_ipv4_range(&self) -> anyhow::Result<Ipv4Range> {
        let net = self.ipv4_nets.first()
            .ok_or_else(|| anyhow::anyhow!("No IPv4 networks available in configuration"))?;
        range::from_ipv4_net(*net)
    }

    pub fn get_link_local(&self) -> anyhow::Result<Ipv6Addr> {
        self.ipv6_nets.iter()
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

    pub fn has_addr(&self, target_addr: &IpAddr) -> bool {
        match target_addr {
            IpAddr::V4(ipv4_addr) => self.targets_v4.contains(ipv4_addr),
            IpAddr::V6(ipv6_addr) => self.targets_v6.contains(ipv6_addr),
        }
    }
}