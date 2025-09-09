use std::net::Ipv4Addr;
use std::thread;
use std::time::Duration;
use anyhow::Context;
use pnet::datalink::MacAddr;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use crate::host::Host;
use crate::net::channel::SenderContext;
use crate::net::packets::ethernet;
use crate::net::range::ip_iter;
use crate::net::utils::{ETH_HDR_LEN, ARP_LEN, MIN_ETH_FRAME_NO_FCS};
use crate::print;

pub fn send_packets(sender_context: &mut SenderContext) -> anyhow::Result<()> {
    let len = ip_iter(&sender_context.ipv4range).count() as u64;
    let progress_bar = print::create_progressbar(len, "ARP".to_string());
    for ip in ip_iter(&sender_context.ipv4range) {
        let arp_packet = create_packet(sender_context.mac_addr, sender_context.src_addr_v4, ip)?;
        sender_context.tx.send_to(arp_packet.as_slice(), None);
        progress_bar.inc(1);
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
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

fn create_packet(src_mac: MacAddr, src_addr: Ipv4Addr, dst_addr: Ipv4Addr)
                     -> anyhow::Result<Vec<u8>> {
    let mut pkt = [0u8; MIN_ETH_FRAME_NO_FCS];
    ethernet::make_header(&mut pkt, src_mac, MacAddr::broadcast(), EtherTypes::Arp)?;
    let mut arp_packet = MutableArpPacket::new(&mut pkt[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN])
        .context("failed to create mutable ARP packet")?;
    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);
    arp_packet.set_sender_hw_addr(src_mac);
    arp_packet.set_target_hw_addr(MacAddr::new(0, 0, 0, 0, 0, 0));
    arp_packet.set_sender_proto_addr(src_addr);
    arp_packet.set_target_proto_addr(dst_addr);
    Ok(Vec::from(pkt))
}