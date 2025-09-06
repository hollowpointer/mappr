use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use colored::{ColoredString, Colorize};
use mac_oui::Oui;
use pnet::datalink::MacAddr;
use once_cell::sync::Lazy;

static OUI_DB: Lazy<Oui> = Lazy::new(|| {
    Oui::default().expect("failed to load OUI database")
});

#[derive(Debug, Default)]
pub struct Host {
    ipv4: Option<Ipv4Addr>,
    ipv6: Vec<Ipv6Addr>,
    mac_addr: Option<MacAddr>,
    vendor: Option<String>,
}

impl Host {
    pub fn _new(ipv4: Option<Ipv4Addr>, ipv6: Vec<Ipv6Addr>, mac_addr: Option<MacAddr>)
        -> anyhow::Result<Self> {
        let vendor = match mac_addr {
            Some(mac) => identify_vendor(mac)?,
            None => None
        };
        Ok(Self { ipv4, ipv6, mac_addr, vendor })
    }

    pub fn set_ipv4(&mut self, ipv4: Ipv4Addr) {
        self.ipv4 = Some(ipv4)
    }

    pub fn add_ipv6(&mut self, ipv6: Ipv6Addr) {
        if !self.ipv6.contains(&ipv6) { self.ipv6.push(ipv6) }
    }

    pub fn add_ipv6_as_vec(&mut self, ipv6: Vec<Ipv6Addr>) {
        for ip in ipv6 { self.add_ipv6(ip) }
    }

    pub fn set_mac_addr(&mut self, mac_addr: MacAddr) -> anyhow::Result<()> {
        self.mac_addr = Some(mac_addr);
        self.vendor = identify_vendor(mac_addr)?;
        Ok(())
    }

    pub fn print_lan(&self, idx: u32) {
        let mut vendor: ColoredString = "Unknown".red().bold();
        if let Some(vendor_string) = self.vendor.clone() {
            vendor = vendor_string.red().bold();
        }
        println!("\x1b[32m[{idx}] {vendor}");
        if let Some(ipv4) = self.ipv4 {
            println!("├─ IPv4 : {}", ipv4.to_string().cyan())
        }
        if let Some(gua) = self.ipv6.iter()
            .find(|&&x| { x.to_string().starts_with("2") }) {
            println!("├─ GUA  : {}", gua.to_string().blue())
        }
        if let Some(lla) = self.ipv6.iter()
            .find(|&&x| { x.to_string().starts_with("fe80") }) {
            println!("├─ LLA  : {}", lla.to_string().blue())
        }
        if let Some(mac_addr) = self.mac_addr {
            println!("└─ MAC  : {}", mac_addr.to_string().yellow())
        }
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
                    e.get_mut().add_ipv6_as_vec(host.ipv6);
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