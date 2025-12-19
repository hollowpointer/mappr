use mac_oui::Oui;
use pnet::datalink::MacAddr;
use std::{
    collections::{BTreeSet, HashSet},
    net::IpAddr, sync::OnceLock,
};

static OUI_DB: OnceLock<Oui> = OnceLock::new();

fn get_oui_db() -> &'static Oui {
    OUI_DB.get_or_init(|| {
        Oui::default().expect("failed to load OUI database")
    })
}

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
            vendor: identify_vendor(mac_addr),
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

fn identify_vendor(mac_addr: MacAddr) -> Option<String> {
    let oui_db: &Oui = get_oui_db();
    match oui_db.lookup_by_mac(&mac_addr.to_string()) {
        Ok(Some(entry)) => Some(entry.company_name.clone()),
        Ok(None) => None,
        Err(_) => {
            // We sink the error here as it's just enrichment
            None
        }
    }
}
