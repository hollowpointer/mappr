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
    ip_addr: IpAddr,
    vendor: Option<String>,
    mac_addr: Option<MacAddr>,
}

impl Host {
    pub fn new(ip_addr: IpAddr, mac_addr: Option<MacAddr>) -> Self {
        let vendor = mac_addr.and_then(|mac|
            identify_vendor(mac).expect("failed to identify vendor"));
        Self { ip_addr, vendor, mac_addr }
    }

    pub fn set_mac_addr(&mut self, mac: MacAddr) -> anyhow::Result<()> {
        self.mac_addr = Some(mac);
        self.vendor = self.mac_addr.and_then(|m| identify_vendor(m).ok()).context("")?;
        Ok(())
    }

    pub fn print_lan(&self, idx: u32) {
        let ip_addr = self.ip_addr.to_string().blue();
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