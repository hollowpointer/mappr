pub mod arp;
pub mod dns;
pub mod icmp;
pub mod ip;
pub mod ndp;
pub mod udp;

use crate::engine::sender::SenderConfig;
use crate::adapters::outbound::terminal::print;
use anyhow::Context;
use pnet::ipnetwork::Ipv4Network;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::util::MacAddr;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn create_packets(sender_config: &SenderConfig) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut packets: Vec<Vec<u8>> = Vec::new();

    match create_arp_packets(sender_config) {
        Ok(arp_packets) => {
            print::print_status(&format!(
                "Created {} ARP discovery packets",
                arp_packets.len()
            ));
            packets.extend(arp_packets);
        }
        Err(e) => {
            print::print_status(&format!("Skipping ARP discovery: {}", e));
        }
    }

    match create_icmpv6_packet(sender_config) {
        Ok(icmpv6_packet) => {
            if !icmpv6_packet.is_empty() {
                print::print_status("Created ICMPv6 discovery packet");
                packets.push(icmpv6_packet);
            }
        }
        Err(e) => {
            print::print_status(&format!("Skipping ICMPv6 discovery: {}", e));
        }
    }

    if packets.is_empty() {
        Err(anyhow::anyhow!(
            "No discovery packets could be created. Check context and logs."
        ))
    } else {
        Ok(packets)
    }
}

fn create_arp_packets(sender_config: &SenderConfig) -> anyhow::Result<Vec<Vec<u8>>> {
    let src_mac: MacAddr = sender_config.get_local_mac()?;
    let dst_mac: MacAddr = MacAddr::broadcast();
    let src_net: Ipv4Network = sender_config.get_ipv4_net()?;
    let src_addr: Ipv4Addr = src_net.ip();
    let targets_v4: HashSet<Ipv4Addr> = sender_config.get_targets_v4();
    let packets: Vec<Vec<u8>> = targets_v4
        .iter()
        .map(|dst_addr: &Ipv4Addr| arp::create_packet(src_mac, dst_mac, src_addr, *dst_addr))
        .collect::<Result<Vec<Vec<u8>>, _>>()?;
    Ok(packets)
}

fn create_icmpv6_packet(sender_config: &SenderConfig) -> anyhow::Result<Vec<u8>> {
    let link_local: Ipv6Addr = sender_config.get_link_local()?;
    let local_mac: MacAddr = sender_config.get_local_mac()?;
    let packet: Vec<u8> = icmp::create_all_nodes_echo_request_v6(local_mac, link_local)
        .context("Failed to create ICMPv6 echo request")?;
    Ok(packet)
}

pub fn get_ip_addr_from_eth(frame: &EthernetPacket) -> anyhow::Result<IpAddr> {
    match frame.get_ethertype() {
        EtherTypes::Arp => Ok(IpAddr::V4(arp::get_ipv4_addr_from_eth(&frame)?)),
        EtherTypes::Ipv4 => Ok(IpAddr::V4(ip::get_ipv4_addr_from_eth(&frame)?)),
        EtherTypes::Ipv6 => Ok(IpAddr::V6(ip::get_ipv6_addr_from_eth(&frame)?)),
        _ => Err(anyhow::anyhow!(
            "Unsupported EtherType: {:?}",
            frame.get_ethertype()
        )),
    }
}
