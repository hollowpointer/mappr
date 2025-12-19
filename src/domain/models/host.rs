use pnet::datalink::MacAddr;
use std::{
    collections::{BTreeSet, HashSet},
    net::IpAddr,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NetworkRole {
    _Gateway,
    _DHCP,
    _DNS,
}

#[derive(Debug, Default, Clone)]
pub struct InternalHost {
    pub hostname: String,
    pub ips: BTreeSet<IpAddr>,
    pub _ports: BTreeSet<u16>,
    pub mac_addr: MacAddr,
    pub vendor: Option<String>,
    pub network_roles: HashSet<NetworkRole>,
}

#[derive(Debug, Clone)]
pub struct ExternalHost {
    pub hostname: String,
    pub ips: BTreeSet<IpAddr>,
    pub _ports: BTreeSet<u16>,
}

impl From<MacAddr> for InternalHost {
    fn from(mac_addr: MacAddr) -> Self {
        Self {
            hostname: String::from("No hostname"),
            ips: BTreeSet::new(),
            _ports: BTreeSet::new(),
            mac_addr,
            vendor: None,
            network_roles: HashSet::new(),
        }
    }
}

impl From<IpAddr> for ExternalHost {
    fn from(ip: IpAddr) -> Self {
        Self {
            hostname: String::from("No hostname"),
            ips: BTreeSet::from([ip]),
            _ports: BTreeSet::new(),
        }
    }
}

pub trait Host {
    fn get_primary_ip(&self) -> Option<IpAddr>;
    fn set_hostname(&mut self, name: String);
    fn set_vendor(&mut self, _vendor: String) {} // Default: do nothing
    fn mac_addr(&self) -> Option<MacAddr> { None }
    fn vendor(&self) -> Option<&str> { None }
    fn roles(&self) -> Option<&HashSet<NetworkRole>> { None }
    fn hostname(&self) -> &str;
    fn ips(&self) -> &BTreeSet<IpAddr>;
}

impl Host for InternalHost {
    fn get_primary_ip(&self) -> Option<IpAddr> {
        self.ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .or_else(|| self.ips.iter().next())
            .cloned()
    }

    fn set_hostname(&mut self, name: String) {
        self.hostname = name;
    }

    fn set_vendor(&mut self, vendor: String) {
        self.vendor = Some(vendor);
    }

    fn mac_addr(&self) -> Option<MacAddr> {
        Some(self.mac_addr)
    }

    fn vendor(&self) -> Option<&str> {
        self.vendor.as_deref()
    }

    fn roles(&self) -> Option<&HashSet<NetworkRole>> {
        Some(&self.network_roles)
    }

    fn hostname(&self) -> &str {
        &self.hostname
    }

    fn ips(&self) -> &BTreeSet<IpAddr> {
        &self.ips
    }
}

impl Host for ExternalHost {
    fn get_primary_ip(&self) -> Option<IpAddr> {
        self.ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .or_else(|| self.ips.iter().next())
            .cloned()
    }

    fn set_hostname(&mut self, name: String) {
        self.hostname = name;
    }

    fn hostname(&self) -> &str {
        &self.hostname
    }

    fn ips(&self) -> &BTreeSet<IpAddr> {
        &self.ips
    }
}

pub fn external_to_box(hosts: Vec<ExternalHost>) -> Vec<Box<dyn Host>> {
    hosts
        .into_iter()
        .map(|host| Box::new(host) as Box<dyn Host>)
        .collect()
}

pub fn internal_to_box(hosts: Vec<InternalHost>) -> Vec<Box<dyn Host>> {
    hosts
        .into_iter()
        .map(|host| Box::new(host) as Box<dyn Host>)
        .collect()
}
