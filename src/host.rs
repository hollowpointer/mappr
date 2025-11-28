use crate::{
    net::ip,
    utils::{colors, print},
};
use colored::*;
use mac_oui::Oui;
use once_cell::sync::Lazy;
use pnet::datalink::MacAddr;
use std::{
    collections::{BTreeSet, HashMap, hash_map},
    net::IpAddr,
};

static OUI_DB: Lazy<Oui> = Lazy::new(|| Oui::default().expect("failed to load OUI database"));

pub trait Host {
    fn print_details(&self, idx: usize);
    fn get_primary_ip(&self) -> Option<IpAddr>;
    fn set_hostname(&mut self, name: String);
}

#[derive(Debug, Default, Clone)]
pub struct InternalHost {
    pub hostname: String,
    pub ips: BTreeSet<IpAddr>,
    pub ports: BTreeSet<u16>,
    pub mac_addr: MacAddr,
    pub vendor: Option<String>,
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
            ports: BTreeSet::new(),
            mac_addr,
            vendor: identify_vendor(mac_addr),
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
                self.vendor.clone().unwrap().to_string().color(colors::MAC_ADDR)
            );
            key_value_pair.push(vendor_key_value);
        }
        print::as_tree_one_level(key_value_pair);
    }

    fn get_primary_ip(&self) -> Option<IpAddr> {
        self.ips.iter()
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
        self.ips.iter()
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

pub fn merge_by_mac(hosts: &mut Vec<InternalHost>) {
    let mut merged: HashMap<MacAddr, InternalHost> = HashMap::with_capacity(hosts.len());
    for host in hosts.drain(..) {
        match merged.entry(host.mac_addr) {
            hash_map::Entry::Occupied(mut occupied_entry) => {
                let existing_host: &mut InternalHost = occupied_entry.get_mut();
                existing_host.ips.extend(host.ips);
                existing_host.ports.extend(host.ports);
                if existing_host.vendor.is_none() && !host.vendor.is_some() {
                    existing_host.vendor = host.vendor;
                }
            }
            hash_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(host);
            }
        }
    }
    hosts.extend(merged.into_values());
}

fn identify_vendor(mac_addr: MacAddr) -> Option<String> {
    let oui_db = &*OUI_DB;
    match oui_db.lookup_by_mac(&mac_addr.to_string()) {
        Ok(Some(entry)) => Some(entry.company_name.clone()),
        Ok(None) => None,
        Err(e) => {
            eprintln!("OUI lookup failed: {e}");
            None
        }
    }
}
