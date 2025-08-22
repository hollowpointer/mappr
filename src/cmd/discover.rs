use anyhow::Result;
use mac_oui::Oui;
use pnet::datalink;
use pnet::datalink::{Config, NetworkInterface};
use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use crate::cmd::Target;
use crate::net::packets;
use crate::net;

pub struct Host {
    vendor: String,
    ipv4: Ipv4Addr,
    mac_addr: MacAddr,
}

pub fn discover(target: Target) {
    let intf = net::interface::select(Target::LAN, &datalink::interfaces());
    let ip_range = net::range::ip_range(target.clone(), &intf);
    if let Err(e) = match target {
        Target::LAN => discover_lan(intf, ip_range.0, ip_range.1),
        _ => { Ok(()) }
    } {
        eprintln!("discover failed: {e}")
    }
}

fn discover_lan(intf: NetworkInterface, start_addr: Ipv4Addr, end_addr: Ipv4Addr) -> Result<()> {
    let oui_db = Oui::default().expect("Failed to load OUI DB");
    let mut channel_cfg: Config = Config::default();
    channel_cfg.read_timeout = Some(Duration::from_millis(100));
    let (mut tx, mut rx) = net::channel::open_ethernet_channel(&intf, &channel_cfg)?;
    for ip in u32::from(start_addr)..=u32::from(end_addr) {
        packets::arp::send(&intf, Ipv4Addr::from(ip), &mut tx).expect("Failed to perform ARP sweep");
    }
    let deadline = Instant::now() + Duration::from_millis(3000);
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => { packets::handle_frame(&frame, &oui_db); },
            Err(_) => { }
        }
    }
    Ok(())
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
        Prints this for each host found:
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