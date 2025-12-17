use crate::{
    net::ip,
    terminal::{colors, print},
};
use colored::*;
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

pub trait Host {
    fn print_details(&self, idx: usize);
    fn get_primary_ip(&self) -> Option<IpAddr>;
    fn set_hostname(&mut self, name: String);
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

impl Host for InternalHost {
    fn print_details(&self, idx: usize) {
        print::tree_head(idx, &self.hostname);
        let mut key_value_pair: Vec<(String, ColoredString)> = ip::to_key_value_pair(&self.ips);

        let mac_key_value: (String, ColoredString) = (
            "MAC".to_string(),
            self.mac_addr.to_string().color(colors::MAC_ADDR),
        );
        key_value_pair.push(mac_key_value);

        if self.vendor.is_some() {
            let vendor_key_value: (String, ColoredString) = (
                "Vendor".to_string(),
                self.vendor
                    .clone()
                    .unwrap()
                    .to_string()
                    .color(colors::MAC_ADDR),
            );
            key_value_pair.push(vendor_key_value);
        }

        if !self.network_roles.is_empty() {
            let joined_roles: String = self
                .network_roles
                .iter()
                .map(|role| format!("{:?}", role))
                .collect::<Vec<String>>()
                .join(", ");

            let roles_key_value: (String, ColoredString) =
                ("Roles".to_string(), joined_roles.normal());

            key_value_pair.push(roles_key_value);
        }

        print::as_tree_one_level(key_value_pair);
    }

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
}

impl Host for ExternalHost {
    fn print_details(&self, idx: usize) {
        print::tree_head(idx, &self.hostname);
        let key_value_pair: Vec<(String, ColoredString)> = ip::to_key_value_pair(&self.ips);
        print::as_tree_one_level(key_value_pair);
    }

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
        Err(e) => {
            eprintln!("OUI lookup failed: {e}");
            None
        }
    }
}
