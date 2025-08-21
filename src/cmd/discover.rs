use std::net::Ipv4Addr;
use pnet::datalink;
use crate::cmd::Target;
use crate::net::packets;
use crate::net;

pub fn discover(target: Target) {
    match target {
        Target::LAN => discover_lan()
    }
}

pub fn discover_lan() {
    if let Some(interface) = net::interface::select(Target::LAN, &datalink::interfaces()) {
        match datalink::channel(&interface, Default::default()) {
            Ok(datalink::Channel::Ethernet(mut tx, mut rx)) => {
                if let Ok(packet) = packets::Packet::new(
                    packets::PacketType::ARP,
                    &interface,
                    Ipv4Addr::new(0, 0, 0, 0)) {
                    tx.send_to(packet.bytes(), Some(interface.clone()));
                }
            }
            Ok(_) => {

            }
            Err(e) => {
                eprintln!("Error creating datalink channel: {e}")
            }
        }
    }
}