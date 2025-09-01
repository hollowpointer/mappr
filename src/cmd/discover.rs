use anyhow;
use pnet::datalink::{Config, NetworkInterface};
use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use std::time::Duration;
use anyhow::Context;
use colored::{ColoredString, Colorize};
use is_root::is_root;
use crate::cmd::Target;
use crate::net::*;
use crate::net::channel::discover_hosts_on_eth_channel;
use crate::net::interface;
use crate::net::tcp::handshake_discovery;
use crate::print;

pub struct Host {
    vendor: Option<String>,
    ipv4: Ipv4Addr,
    mac_addr: Option<MacAddr>,
    // Impl for host is at the bottom
}

pub async fn discover(target: Target) -> anyhow::Result<()> {
    let hosts: Option<Vec<Host>> = match target {
        Target::LAN => {
            print::print_status("Initializing LAN discovery...");
            let intf = interface::select(Target::LAN);
            let (start, end) = range::ip_range(Target::LAN, &intf)?;
            Some(discover_lan(start, end, intf).await?)
        },
        _ => { None }
    };
    print::separator("Network Discovery");
    if let Some(hosts) = hosts {
        let mut idx: u32 = 0;
        for host in hosts {
            host.print_lan(idx);
            idx += 1;
        }
    }
    Ok(())
}

async fn discover_lan(start_addr: Ipv4Addr, end_addr: Ipv4Addr, intf: NetworkInterface)
                      -> anyhow::Result<Vec<Host>> {
    let mut hosts: Vec<Host> = Vec::new();
    if !is_root() {
        let addresses = handshake_discovery(start_addr, end_addr).await?;
        for address in addresses {
            let vendor: Option<String> = None;
            let mac_addr: Option<MacAddr> = None;
            let host = Host::new(address, vendor, mac_addr);
            hosts.push(host);
        }
        return Ok(hosts)
    }
    let mut channel_cfg: Config = Config::default();
    channel_cfg.read_timeout = Some(Duration::from_millis(100));
    print::print_status("Establishing Ethernet connection...");
    hosts = discover_hosts_on_eth_channel(
        start_addr,
        end_addr,
        intf,
        channel_cfg,
        Duration::from_millis(500),
    ).context("discovering via ethernet channel")?;
    Ok(hosts)
}

impl Host {
    pub fn new(ipv4: Ipv4Addr, vendor: Option<String>, mac_addr: Option<MacAddr>) -> Self {
        Self {
            ipv4,
            vendor,
            mac_addr,
        }
    }

    pub fn print_lan(&self, idx: u32) {
        let ip_addr = self.ipv4.to_string().blue();
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