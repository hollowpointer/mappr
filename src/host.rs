use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::IpAddr;
use anyhow::Context;
use colored::{ColoredString, Colorize};
use mac_oui::Oui;
use pnet::datalink::MacAddr;
use once_cell::sync::Lazy;

static OUI_DB: Lazy<Oui> = Lazy::new(|| {
    Oui::default().expect("failed to load OUI database")
});

#[derive(Debug)]
pub struct Host {
    ip_addrs: Vec<IpAddr>,
    vendor: Option<String>,
    mac_addr: Option<MacAddr>,
}

impl Host {
    pub fn new(ip_addr: IpAddr, mac_addr: Option<MacAddr>) -> Self {
        let vendor = mac_addr.and_then(|mac|
            identify_vendor(mac).expect("failed to identify vendor"));
        Self { ip_addrs: vec![ip_addr], vendor, mac_addr }
    }

    pub fn add_ip(&mut self, ip_addr: IpAddr) {
        if !self.ip_addrs.contains(&ip_addr) { self.ip_addrs.push(ip_addr) }
    }

    pub fn add_ips(&mut self, ip_addrs: Vec<IpAddr>) {
        for ip in ip_addrs { self.add_ip(ip) }
    }

    pub fn set_mac_addr(&mut self, mac: MacAddr) -> anyhow::Result<()> {
        self.mac_addr = Some(mac);
        self.vendor = self.mac_addr.and_then(|m| identify_vendor(m).ok()).context("")?;
        Ok(())
    }

    pub fn print_lan(&self, idx: u32) {
        let ip_addr = format!("{:?}", self.ip_addrs).blue();
        let mut vendor: ColoredString = "Unknown".red().bold();
        if let Some(vendor_string) = self.vendor.clone() {
            vendor = vendor_string.red().bold();
        }
        let mut mac_addr_str: ColoredString = "??:??:??:??:??:??".yellow();
        if let Some(mac_addr) = self.mac_addr {
            mac_addr_str = mac_addr.to_string().yellow();
        }
        print!("\x1b[32m[{idx}] {vendor}\n\
                       ├─ IP  : {ip_addr}\n\
                       └─ MAC : {mac_addr_str}\n"
        );
        let separator = "------------------------------------------------------------".bright_black();
        println!("{separator}");
    }
}

pub fn merge_by_mac_addr(hosts: Vec<Host>) -> Vec<Host> {
    let mut by_mac: HashMap<MacAddr, Host> = HashMap::new();
    let mut out: Vec<Host> = Vec::new();

    for host in hosts {
        match host.mac_addr {
            Some(mac) => match by_mac.entry(mac) {
                Entry::Vacant(v) => { v.insert(host); }
                Entry::Occupied(mut e) => {
                    e.get_mut().add_ips(host.ip_addrs);
                    if e.get().vendor.is_none() && host.vendor.is_some() {
                        e.get_mut().vendor = host.vendor;
                    }
                }
            },
            None => { out.push(host); }
        }
    }

    out.extend(by_mac.into_values());
    out
}

fn identify_vendor(mac_addr: MacAddr) -> anyhow::Result<Option<String>> {
    let oui_db = &*OUI_DB;
    let vendor: String = match oui_db.lookup_by_mac(&mac_addr.to_string()) {
        Ok(Some(entry)) => entry.company_name.clone(),
        Ok(None)        => "Unknown".to_string(),
        Err(e) => {
            eprintln!("OUI lookup failed: {e}");
            "Unknown".to_string()
        }
    };
    Ok(Some(vendor))
}