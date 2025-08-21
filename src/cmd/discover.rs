use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use pnet::datalink;
use pnet::datalink::{Channel, Config};
use anyhow::{anyhow, Result};
use pnet::packet::arp::{ArpOperations, ArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use pnet::util::MacAddr;
use crate::cmd::Target;
use crate::net::packets;
use crate::net;

struct Host {
    ipv4: Ipv4Addr,
    mac_addr: MacAddr,
}

impl Host {
    fn new(ipv4: Ipv4Addr, mac_addr: MacAddr) -> Self {
        Self {
            ipv4,
            mac_addr,
        }
    }

    fn print_tree(&self) {
        println!("Found Device:");
        println!("├── IP      : {}", self.ipv4);
        println!("└── MAC     : {}", self.mac_addr);
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
    cfg.read_timeout = Some(Duration::from_millis(50));
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

    println!("===============================");
    let deadline = Instant::now() + Duration::from_millis(1000);
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Some(eth) = EthernetPacket::new(frame) {
                    if eth.get_ethertype() == EtherTypes::Arp {
                        if let Some(arp) = ArpPacket::new(eth.payload()) {
                            if arp.get_operation() == ArpOperations::Reply {
                                let host = Host::new(
                                    arp.get_sender_proto_addr(),
                                    arp.get_sender_hw_addr());
                                host.print_tree();
                                println!("===============================");
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