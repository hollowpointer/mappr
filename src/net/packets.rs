pub mod icmp;
mod ip;
pub mod tcp;

use std::net::Ipv4Addr;
use anyhow::{Context, Ok, bail};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::util::MacAddr;
use crate::host::InternalHost;
use crate::net::datalink::arp;
use crate::net::datalink::channel::SenderContext;
use crate::net::range::{self, Ipv4Range};
use crate::utils::print;

pub enum PacketType {
    Arp,
    Icmpv6,
    Ndp
}

pub fn create_single_packet(sender_context: &SenderContext, packet_type: PacketType) -> anyhow::Result<Vec<u8>> {
    let packet: Vec<u8> = match packet_type {
        PacketType::Arp     => create_arp_packet(sender_context)?,
        PacketType::Ndp     => create_ndp_packet(sender_context)?,
        PacketType::Icmpv6  => create_icmpv6_packet(sender_context)?
    };
    Ok(packet)
}

pub fn create_multiple_packets(sender_context: &SenderContext, packet_types: Vec<PacketType>) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut packets: Vec<Vec<u8>> = Vec::new();
    for packet_type in packet_types {
        match packet_type {
            PacketType::Arp => packets.extend(create_arp_packets(sender_context)?),
            PacketType::Icmpv6 => packets.extend(vec![create_icmpv6_packet(sender_context)?]),
            PacketType::Ndp => packets.extend(vec![create_ndp_packet(sender_context)?])
        }
    }
    Ok(packets)
}

fn create_arp_packet(sender_context: &SenderContext) -> anyhow::Result<Vec<u8>> {
    let (ipv4_net, dst_addr) = (sender_context.ipv4_net, sender_context.dst_addr_v4);
    match (ipv4_net, dst_addr) {
        (Some(ipv4_net), Some(dst_addr)) => {
            let src_mac: MacAddr = sender_context.src_mac;
            let dst_mac: MacAddr = MacAddr::broadcast();
            let src_addr: Ipv4Addr = ipv4_net.ip();
            let packet: Vec<u8> = arp::create_packet(src_mac, dst_mac, src_addr, dst_addr)?;
            Ok(packet)
        }
        _ => {
            print::print_status("Failed to create ARP packet: invalid sender context");
            Ok(vec![])
        },
    }
}

fn create_arp_packets(sender_context: &SenderContext) -> anyhow::Result<Vec<Vec<u8>>> {
    if let Some(ipv4_net) = sender_context.ipv4_net {
        print::print_status("Creating ARP packets for ipv4 discovery");
        let src_mac: MacAddr = sender_context.src_mac;
        let dst_mac: MacAddr = MacAddr::broadcast();
        let src_addr: Ipv4Addr = ipv4_net.ip();
        let ipv4_range: Ipv4Range = range::Ipv4Range::from_tuple(range::cidr_range(src_addr, ipv4_net.prefix())?);
        let packets: Vec<Vec<u8>> = range::ip_iter(&ipv4_range)
            .map(|dst_addr| {
                arp::create_packet(src_mac, dst_mac, src_addr, dst_addr)
            })
            .collect::<Result<Vec<Vec<u8>>, _>>()?;
        Ok(packets)
    } else {
        print::print_status("Failed to create ARP packets: No ipv4 found");
        Ok(vec![])
    }
}

fn create_icmpv6_packet(sender_context: &SenderContext) -> anyhow::Result<Vec<u8>> {
    if let Some(link_local) = sender_context.link_local {
        print::print_status("Creating ICMPv6 packets for ipv6 discovery");
        let src_mac: MacAddr = sender_context.src_mac;
        let packet: Vec<u8> = icmp::create_all_nodes_echo_request_v6(src_mac, link_local)?;
        Ok(packet)
    } else {
        print::print_status("Failed to create ICMPv6 packets: No link local address");
        Ok(vec![])
    }
}

fn create_ndp_packet(_sender_context: &SenderContext) -> anyhow::Result<Vec<u8>> {
    anyhow::bail!("Ndp packet creation not possible as of now");
}

pub fn handle_frame(frame: &[u8]) -> anyhow::Result<InternalHost> {
    let eth = EthernetPacket::new(frame)
        .context("truncated or invalid Ethernet frame")?;
    let mac_addr: MacAddr = eth.get_source();
    let ip = match eth.get_ethertype() {
        EtherTypes::Arp => arp::handle_packet(eth)?,
        EtherTypes::Ipv6 => ip::handle_v6_packet(eth)?,
        other => bail!("unsupported ethertype: 0x{:04x}", other.0),
    };
    let mut host: InternalHost = InternalHost::from(mac_addr);
    host.ips.insert(ip);
    Ok(host)
}

// ╔════════════════════════════════════════════╗
// ║ ████████╗███████╗███████╗████████╗███████╗ ║
// ║ ╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝ ║
// ║    ██║   █████╗  ███████╗   ██║   ███████╗ ║
// ║    ██║   ██╔══╝  ╚════██║   ██║   ╚════██║ ║
// ║    ██║   ███████╗███████║   ██║   ███████║ ║
// ║    ╚═╝   ╚══════╝╚══════╝   ╚═╝   ╚══════╝ ║
// ╚════════════════════════════════════════════╝

#[cfg(test)]
pub mod tests {
    use super::*;
    use pnet::packet::ethernet::EtherTypes;
    use pnet::util::MacAddr;
    use crate::net::datalink::ethernet;
    use crate::net::utils::MIN_ETH_FRAME_NO_FCS;

    const ARP_LEN: usize = 28;
    const ETH_HDR_LEN: usize = 14;

    pub fn buf() -> [u8; MIN_ETH_FRAME_NO_FCS] {
        [0u8; MIN_ETH_FRAME_NO_FCS]
    }

    #[test]
    fn handle_frame_errors_on_short_ethernet_buffer() {
        // Too short to contain an Ethernet header
        let short = [0u8; ETH_HDR_LEN - 1];

        let err = handle_frame(&short).unwrap_err();

        assert!(
            err.to_string().contains("Ethernet"),
            "unexpected error: {err:?}"
        );
    }


    #[test]
    fn handle_frame_errors_on_bad_arp_buffer() {
        // Frame declares ARP but payload is too short for an ARP packet
        let mut frame = vec![0u8; ETH_HDR_LEN + ARP_LEN - 1]; // one byte short
        ethernet::make_header(
            &mut frame,
            MacAddr::zero(),
            MacAddr::broadcast(),
            EtherTypes::Arp,
        )
            .expect("eth header");

        let err = handle_frame(&frame).unwrap_err();

        assert!(
            err.to_string().contains("ARP"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn handle_frame_unsupported_ethertype() {
        let mut b = buf();
        ethernet::make_header(
            &mut b,
            MacAddr::zero(),
            MacAddr::broadcast(),
            EtherTypes::Ipv4,
        )
            .expect("eth header");

        let err = handle_frame(&b).unwrap_err();

        assert!(
            err.to_string().contains("unsupported ethertype"),
            "unexpected error: {err:?}"
        );
        assert!(
            err.to_string().contains(&format!("{:04x}", EtherTypes::Ipv4.0)),
            "error did not mention Ipv4 ethertype: {err:?}"
        );
    }
}
