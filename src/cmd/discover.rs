use anyhow::{anyhow, Result};
use mac_oui::Oui;
use pnet::datalink;
use pnet::datalink::{Channel, Config};
use pnet::packet::arp::{ArpOperations, ArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use crate::cmd::Target;
use crate::net::packets;
use crate::net;

struct Host {
    id: u32,
    vendor: String,
    ipv4: Ipv4Addr,
    mac_addr: MacAddr,
}

impl Host {
    fn new(id: u32, vendor: String, ipv4: Ipv4Addr, mac_addr: MacAddr) -> Self {
        Self {
            id,
            vendor,
            ipv4,
            mac_addr,
        }
    }

    fn print(&self) {
        let side = "\x1b[90m│\x1b[0m";
        let width = 50; // inner width of the box

        println!("\x1b[90m┌{}┐\x1b[0m", "─".repeat(width));

        // Device Found line (pad first, then color)
        let text = format!("[+] Device {} Found", self.id);
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

pub fn discover(target: Target) {
    if let Err(e) = match target {
        Target::LAN => discover_lan()
    } {
        eprintln!("discover failed: {e}")
    }
}

fn discover_lan() -> Result<()> {
    let interface = net::interface::select(Target::LAN, &datalink::interfaces())
        .ok_or_else(|| anyhow!("No suitable LAN interface found"))?;

    let mut cfg: Config = Config::default();
    cfg.read_timeout = Some(Duration::from_millis(100));
    let (mut tx, mut rx) = match datalink::channel(&interface, cfg) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => anyhow::bail!("Non-ethernet channel returned for interface {}", interface.name),
        Err(e) => anyhow::bail!("Error creating datalink channel: {}", e)
    };

    for ip in 1..=254u8 {
        let ip_addr = Ipv4Addr::new(192,168,0, ip);
        let pkt = packets::Packet::new(packets::PacketType::ARP, &interface, ip_addr)?;
        if let Some(Err(e)) = tx.send_to(pkt.bytes(), Some(interface.clone())) {
            eprintln!("send {ip_addr} failed: {e}");
        }
    }

    let oui_db = Oui::default().expect("Failed to load OUI DB");
    let deadline = Instant::now() + Duration::from_millis(3000);
    let mut id = 1;
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Some(eth) = EthernetPacket::new(frame) {
                    if eth.get_ethertype() == EtherTypes::Arp {
                        if let Some(arp) = ArpPacket::new(eth.payload()) {
                            if arp.get_operation() == ArpOperations::Reply {
                                let vendor: String = match oui_db.lookup_by_mac(&arp.get_sender_hw_addr().to_string()) {
                                    Ok(Some(entry)) => entry.company_name.clone(), // get the String from the entry
                                    Ok(None)        => "Unknown".to_string(),
                                    Err(e) => {
                                        eprintln!("OUI lookup failed: {e}");
                                        "Unknown".to_string()
                                    }
                                };
                                let host = Host::new(
                                    id,
                                    vendor,
                                    arp.get_sender_proto_addr(),
                                    arp.get_sender_hw_addr());
                                host.print();
                                id += 1;
                            }
                        }
                    }
                }
            },
            Err(_) => { }
        }
    }

    Ok(())
}