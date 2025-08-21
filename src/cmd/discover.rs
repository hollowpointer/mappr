use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use pnet::datalink;
use pnet::datalink::Channel;
use anyhow::{anyhow, Result};
use pnet::packet::arp::{ArpOperations, ArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use crate::cmd::Target;
use crate::net::packets;
use crate::net;

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

    let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => anyhow::bail!("Non-ethernet channel returned for interface {}", interface.name),
        Err(e) => anyhow::bail!("Error creating datalink channel: {}", e)
    };

    let packet = packets::Packet::new(
        packets::PacketType::ARP,
        &interface,
        Ipv4Addr::new(192, 168, 0, 24))?;

    let mut last_send = Instant::now();
    match tx.send_to(packet.bytes(), Some(interface.clone())) {
        Some(Ok(())) => { last_send = Instant::now() },
        Some(Err(e)) => eprintln!("Failed to send packet: {e}"),
        None => eprintln!("Failed to queue packet")
    }

    loop {
        match rx.next() {
            Ok(frame) => {
                if let Some(eth) = EthernetPacket::new(frame) {
                    if eth.get_ethertype() == EtherTypes::Arp {
                        if let Some(arp) = ArpPacket::new(eth.payload()) {
                            if arp.get_operation() == ArpOperations::Reply {
                                let ip_addr = arp.get_sender_proto_addr();
                                let mac_addr = arp.get_sender_hw_addr();
                                println!("Found! IP: {ip_addr}; MAC: {mac_addr}")
                            }
                        }
                    }
                }
            },
            Err(_) => {}
        }

        if last_send.elapsed() > Duration::from_secs(3) { break }
    }

    Ok(())
}