use anyhow;
use pnet::datalink::{Config, NetworkInterface};
use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use std::time::Duration;
use anyhow::Context;
use colored::Colorize;
use crate::cmd::Target;
use crate::net::*;
use crate::net::channel::discover_hosts_on_eth_channel;
use crate::net::interface;
use crate::print;

pub struct Host {
    vendor: String,
    ipv4: Ipv4Addr,
    mac_addr: MacAddr,
    // Impl for host is at the bottom
}

pub fn discover(target: Target) -> anyhow::Result<()> {
    let hosts: Option<Vec<Host>> = match target {
        Target::LAN => {
            print::print_status("Initializing LAN discovery...");
            let intf = interface::select(Target::LAN);
            let (start, end) = range::ip_range(Target::LAN, &intf)?;
            Some(discover_lan(start, end, intf)?)
        },
        Target::Host { addr } => Some(discover_host(addr)?),
        _ => { None }
    };
    if let Some(hosts) = hosts {
        for host in hosts {
            host.print_lan();
        }
    }
    Ok(())
}

fn discover_lan(start_addr: Ipv4Addr, end_addr: Ipv4Addr, intf: NetworkInterface)
    -> anyhow::Result<Vec<Host>> {
    let mut channel_cfg: Config = Config::default();
    channel_cfg.read_timeout = Some(Duration::from_millis(100));
    print::print_status("Establishing Ethernet connection...");
    let hosts: Vec<Host> = discover_hosts_on_eth_channel(
        start_addr,
        end_addr,
        intf,
        channel_cfg,
        Duration::from_millis(3000)
    ).context("discovering via ethernet channel")?;
    Ok(hosts)
}

// WARNING: This function does not work as expected, real implementation will come later.
fn discover_host(addr: Ipv4Addr) -> anyhow::Result<Vec<Host>> {
    let hosts = match addr.octets()[0] {
        10 | 172 | 192 => { discover_lan(addr, addr, interface::select(Target::LAN)) },
        _ => { discover_lan(addr, addr, interface::select(Target::LAN)) } // WARNING this is a filler to suppress warnings
    };

    hosts
}

impl Host {
    pub fn new(vendor: String, ipv4: Ipv4Addr, mac_addr: MacAddr) -> Self {
        Self {
            vendor,
            ipv4,
            mac_addr,
        }
    }

    // Print one host entry as:
    // [+] Vendor
    //     ├─ IP     : ...
    //     └─ MAC    : ...
    pub fn print_lan(&self) {
        let vendor = self.vendor.bright_red().bold();
        print!("\x1b[32m[+] \x1b[0m{}\n\
                  \x1b[90m    ├─\x1b[0m IP  : \x1b[34m{}\x1b[0m\n\
                  \x1b[90m    └─\x1b[0m MAC : \x1b[33m{}\x1b[0m\n",
               vendor, self.ipv4, self.mac_addr);
        let separator = "------------------------------------------------------------".bright_black();
        println!("{separator}");
    }
}