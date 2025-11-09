use std::net::Ipv4Addr;
use anyhow::Context;
use pnet::datalink::MacAddr;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use crate::host::Host;
use crate::net::datalink::ethernet;
use crate::net::utils::{ETH_HDR_LEN, ARP_LEN, MIN_ETH_FRAME_NO_FCS};

pub fn create_packet(src_mac: MacAddr, dst_mac: MacAddr, src_addr: Ipv4Addr, dst_addr: Ipv4Addr)
                     -> anyhow::Result<Vec<u8>> {
    let mut buffer = [0u8; MIN_ETH_FRAME_NO_FCS];
    ethernet::make_header(&mut buffer, src_mac, MacAddr::broadcast(), EtherTypes::Arp)?;
    let mut arp_packet = MutableArpPacket::new(&mut buffer[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN])
        .context("failed to create mutable ARP packet")?;
    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);
    arp_packet.set_sender_hw_addr(src_mac);
    arp_packet.set_target_hw_addr(dst_mac);
    arp_packet.set_sender_proto_addr(src_addr);
    arp_packet.set_target_proto_addr(dst_addr);
    Ok(Vec::from(buffer))
}

pub fn handle_packet(ethernet_packet: EthernetPacket) -> anyhow::Result<Option<Host>> {
    let arp_packet = ArpPacket::new(ethernet_packet.payload())
        .context(format!(
            "truncated or invalid ARP packet (payload len {})",
            ethernet_packet.payload().len()
        ))?;
    if arp_packet.get_operation() == ArpOperations::Reply {
        let mut host = Host::default();
        host.set_ipv4(arp_packet.get_sender_proto_addr());
        host.set_mac_addr(arp_packet.get_sender_hw_addr())?;
        Ok(Some(host))
    } else { Ok(None) }
}