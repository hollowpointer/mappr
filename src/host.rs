use std::net::{Ipv4Addr, Ipv6Addr};
use colored::{ColoredString, Colorize};
use mac_oui::Oui;
use pnet::datalink::MacAddr;
use once_cell::sync::Lazy;
use crate::cmd::Target;

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

    pub fn set_mac_addr(&mut self, mac_addr: MacAddr) -> anyhow::Result<()> {
        self.mac_addr = Some(mac_addr);
        self.vendor = identify_vendor(mac_addr)?;
        Ok(())
    }

    pub fn get_mac_addr(&self) -> Option<MacAddr> {
        self.mac_addr
    }

    pub fn print_lan(&self, idx: u32) {
        let vendor: ColoredString = self.vendor
            .as_deref()
            .unwrap_or("Unknown")
            .red()
            .bold();

        println!("\x1b[32m[{idx}] {vendor}");

        let mut lines: Vec<(&str, ColoredString)> = Vec::new();

        if let Some(ipv4) = self.ipv4 {
            lines.push(("IPv4", ipv4.to_string().cyan()));
        }
        if let Some(gua) = self.ipv6.iter().find(|&&x| x.to_string().starts_with('2')) {
            lines.push(("GUA", gua.to_string().blue()));
        }
        if let Some(lla) = self.ipv6.iter().find(|&&x| x.to_string().starts_with("fe80")) {
            lines.push(("LLA", lla.to_string().blue()));
        }
        if let Some(mac) = self.mac_addr {
            lines.push(("MAC", mac.to_string().yellow()));
        }

        for (i, (label, value)) in lines.iter().enumerate() {
            let last = i + 1 == lines.len();
            let branch = if last { "└─" } else { "├─" };
            println!(" {branch} {label:<4} : {value}");
        }

        println!("{}", "------------------------------------------------------------".bright_black());
    }

}

pub fn print(mut hosts: Vec<Host>, target: Target) -> anyhow::Result<()> {
    match target {
        Target::LAN => {
            merge_hosts(&mut hosts);
            sort_by_ipv4(&mut hosts);
            for (idx, h) in hosts.into_iter().enumerate() {
                h.print_lan(idx as u32);
            }
            Ok(())
        }
        _ => anyhow::bail!("print implementation for given target not implemented!")
    }
}

fn sort_by_ipv4(hosts: &mut Vec<Host>) {
    hosts.sort_by(|a, b| a.ipv4.cmp(&b.ipv4));
}

fn merge_hosts(hosts: &mut Vec<Host>) {
    let mut merged: Vec<Host> = Vec::new();
    for mut host in hosts.drain(..) {
        let mut found_match = false;
        for existing_host in merged.iter_mut() {
            let mac_match = host.mac_addr.is_some() && existing_host.mac_addr == host.mac_addr;
            let ipv4_match = host.ipv4.is_some() && existing_host.ipv4 == host.ipv4;
            let ipv6_match = host.ipv6.iter().any(|ipv6| existing_host.ipv6.contains(ipv6));
            if mac_match || ipv4_match || ipv6_match {
                if existing_host.ipv4.is_none() && host.ipv4.is_some() {
                    existing_host.ipv4 = host.ipv4.take();
                }
                existing_host.ipv6.extend(&host.ipv6);
                if existing_host.mac_addr.is_none() && host.mac_addr.is_some() {
                    existing_host.mac_addr = host.mac_addr.take();
                }
                if existing_host.vendor.is_none() && host.vendor.is_some() {
                    existing_host.vendor = host.vendor.take();
                }
                found_match = true;
                break;
            }
        }
        if !found_match {
            merged.push(host);
        }
    }
    *hosts = merged;
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