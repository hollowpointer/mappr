use std::{collections::{BTreeSet, HashMap, hash_map}, net::IpAddr};
use colored::*;
use mac_oui::Oui;
use pnet::datalink::MacAddr;
use once_cell::sync::Lazy;
use crate::{net::ip, utils::{colors, print}};

static OUI_DB: Lazy<Oui> = Lazy::new(|| {
    Oui::default().expect("failed to load OUI database")
});

pub trait Host {
    fn print_details(&self, idx: usize);
    fn get_primary_ip(&self) -> Option<IpAddr>;
}

#[derive(Debug, Default, Clone)]
pub struct InternalHost {
    pub ips: BTreeSet<IpAddr>,
    pub ports: BTreeSet<u16>,
    pub mac_addr: MacAddr,
    pub vendor: String,
}

pub struct ExternalHost {
    ips: BTreeSet<IpAddr>,
    _ports: BTreeSet<u16>
}

impl From<MacAddr> for InternalHost {
    fn from(mac_addr: MacAddr) -> Self {
        Self {
            ips: BTreeSet::new(),
            ports: BTreeSet::new(),
            mac_addr,
            vendor: identify_vendor(mac_addr)
        }
    }
}

impl From<IpAddr> for ExternalHost {
    fn from(ip: IpAddr) -> Self {
        Self {
            ips: BTreeSet::from([ip]), 
            _ports: BTreeSet::new()
        }
    }
}

impl Host for InternalHost {
    fn print_details(&self, idx: usize) {
        print::tree_head(idx, &self.vendor);
        let mut key_value_pair: Vec<(String, ColoredString)> = ip::to_key_value_pair(&self.ips);
        let mac_key_value: (String, ColoredString) = ("MAC".to_string(), self.mac_addr.to_string().color(colors::MAC_ADDR));
        key_value_pair.push(mac_key_value);
        print::as_tree_one_level(key_value_pair);
    }
    
    fn get_primary_ip(&self) -> Option<IpAddr> {
        self.ips.iter().next().cloned()
    }
}

impl Host for ExternalHost {
    fn print_details(&self, idx: usize) {
        print::tree_head(idx, "Unknown");
        let key_value_pair: Vec<(String, ColoredString)> = ip::to_key_value_pair(&self.ips);
        print::as_tree_one_level(key_value_pair);
    }
    
    fn get_primary_ip(&self) -> Option<IpAddr> {
        self.ips.iter().next().cloned()
    }
}

pub fn external_to_box(hosts: Vec<ExternalHost>) -> Vec<Box<dyn Host>> {
    hosts.into_iter()
        .map(|host| Box::new(host) as Box<dyn Host>)
        .collect()
}

pub fn internal_to_box(hosts: Vec<InternalHost>) -> Vec<Box<dyn Host>> {
    hosts.into_iter()
        .map(|host| Box::new(host) as Box<dyn Host>)
        .collect()
}

pub fn merge_by_mac(hosts: &mut Vec<InternalHost>) {
    let mut merged: HashMap<MacAddr, InternalHost> = HashMap::with_capacity(hosts.len());
    for host in hosts.drain(..) {
        match merged.entry(host.mac_addr) {
            hash_map::Entry::Occupied(mut occupied_entry) => {
                let existing_host = occupied_entry.get_mut();
                existing_host.ips.extend(host.ips);
                existing_host.ports.extend(host.ports);
                if existing_host.vendor.is_empty() && !host.vendor.is_empty() {
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

fn identify_vendor(mac_addr: MacAddr) -> String {
    let oui_db = &*OUI_DB;
    let vendor: String = match oui_db.lookup_by_mac(&mac_addr.to_string()) {
        Ok(Some(entry)) => entry.company_name.clone(),
        Ok(None) => "Unknown".to_string(),
        Err(e) => {
            eprintln!("OUI lookup failed: {e}");
            "Unknown".to_string()
        }
    };
    vendor
}