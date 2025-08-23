use anyhow;
use pnet::datalink::{Config, NetworkInterface};
use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use std::time::Duration;
use anyhow::Context;
use crate::cmd::Target;
use crate::net::*;
use crate::net::channel::handle_ethernet_channel;
use crate::net::interface;

pub struct Host {
    vendor: String,
    ipv4: Ipv4Addr,
    mac_addr: MacAddr,
    // Impl for host is at the bottom
}

pub fn discover(target: Target) -> anyhow::Result<()> {
    match target {
        Target::LAN => {
            let (start, end) = range::ip_range(Target::LAN, &interface::select(Target::LAN))?;
            discover_lan(start, end, interface::select(Target::LAN))?
        },
        Target::Host { addr } => discover_host(addr)?,
        _ => {}
    }
    Ok(())
}

fn discover_lan(start_addr: Ipv4Addr, end_addr: Ipv4Addr, intf: NetworkInterface)
    -> anyhow::Result<()> {
    let mut channel_cfg: Config = Config::default();
    channel_cfg.read_timeout = Some(Duration::from_millis(100));
    handle_ethernet_channel(
        start_addr,
        end_addr,
        intf,
        channel_cfg,
        Duration::from_millis(3000)
    ).context("discovering via ethernet channel")?;
    Ok(())
}

// WARNING: This function does not work as expected, real implementation will come later.
fn discover_host(addr: Ipv4Addr) -> anyhow::Result<()> {
    match addr.octets()[0] {
        10 | 172 | 192 => { discover_lan(addr, addr, interface::select(Target::LAN)) },
        _ => Ok(())
    }
}

impl Host {
    pub(crate) fn new(vendor: String, ipv4: Ipv4Addr, mac_addr: MacAddr) -> Self {
        Self {
            vendor,
            ipv4,
            mac_addr,
        }
    }

    pub(crate) fn print_lan(&self) {
        /*
        Prints for each host found:
        ┌──────────────────────────────────────────────────┐
        │ [+] Host Found                                   │
        │ ├─ Vendor : Raspberry Pi Trading Ltd             │
        │ ├─ IP     : 192.168.0.150                        │
        │ └─ MAC    : c8:52:61:c7:a1:49                    │
        └──────────────────────────────────────────────────┘
         */
        let side = "\x1b[90m│\x1b[0m";
        let width = 50; // inner width of the box

        println!("\x1b[90m┌{}┐\x1b[0m", "─".repeat(width));

        // Host Found line (pad first, then color)
        let text = "[+] Host Found".to_string();
        println!("{side} \x1b[32m{text}\x1b[0m{:pad$}{side}", "", pad = width - text.len() - 1);

        // Vendor Line (magenta)
        let vendor_text = format!("├─ Vendor : {}", self.vendor);
        println!("{side} \x1b[35m{:<width$}\x1b[0m{side}", vendor_text, width = width - 1);

        // IP line (blue)
        let ip_text = format!("├─ IP     : {}", self.ipv4);
        println!("{side} \x1b[34m{:<width$}\x1b[0m{side}", ip_text, width = width - 1);

        // MAC line (yellow)
        let mac_text = format!("└─ MAC    : {}", self.mac_addr);
        println!("{side} \x1b[33m{:<width$}\x1b[0m{side}", mac_text, width = width - 1);

        println!("\x1b[90m└{}┘\x1b[0m", "─".repeat(width));
    }
}