pub mod icmp;
mod ip;
pub mod tcp;

use std::net::{IpAddr, Ipv4Addr};
use anyhow::Context;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::util::MacAddr;
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


pub fn create_discovery_packets(sender_context: &SenderContext) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut packets: Vec<Vec<u8>> = Vec::new();

    if sender_context.ipv4_net.is_some() {
        match create_arp_packets(sender_context) {
            Ok(arp_packets) => {
                print::print_status(&format!("Created {} ARP discovery packets", arp_packets.len()));
                packets.extend(arp_packets);
            }
            Err(e) => {
                print::print_status(&format!("Skipping ARP discovery: {}", e));
            }
        }
    }

    match create_icmpv6_packet(sender_context) {
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
        Err(anyhow::anyhow!("No discovery packets could be created. Check context and logs."))
    } else {
        Ok(packets)
    }
}


fn create_arp_packet(sender_context: &SenderContext) -> anyhow::Result<Vec<u8>> {
    let ipv4_net = sender_context.ipv4_net
        .context("Missing source IPv4 network for ARP packet")?;
    let dst_addr = sender_context.dst_addr_v4
        .context("Missing destination IPv4 address for ARP packet")?;
    let src_mac: MacAddr = sender_context.src_mac;
    let dst_mac: MacAddr = MacAddr::broadcast();
    let src_addr: Ipv4Addr = ipv4_net.ip();
    let packet: Vec<u8> = arp::create_packet(src_mac, dst_mac, src_addr, dst_addr)
        .context("Failed to create underlying ARP packet")?;
    Ok(packet)
}


fn create_arp_packets(sender_context: &SenderContext) -> anyhow::Result<Vec<Vec<u8>>> {
    let src_mac: MacAddr = sender_context.src_mac;
    let dst_mac: MacAddr = MacAddr::broadcast();
    let src_net = sender_context.ipv4_net
        .context("Failed to create ARP packets: No source IPv4 network in context")?;
    let src_addr: Ipv4Addr = src_net.ip();
    let target_range: Ipv4Range = match &sender_context.ipv4_range {
        Some(explicit_range) => {
            explicit_range.clone()
        }
        None => {
            range::cidr_range(src_addr, src_net.prefix())
        }
    };
    let packets: Vec<Vec<u8>> = range::ip_iter(&target_range)
        .map(|dst_addr| {
            arp::create_packet(src_mac, dst_mac, src_addr, dst_addr)
        })
        .collect::<Result<Vec<Vec<u8>>, _>>()?;
    Ok(packets)
}


fn create_icmpv6_packet(sender_context: &SenderContext) -> anyhow::Result<Vec<u8>> {
    if let Some(link_local) = sender_context.link_local {
        let src_mac: MacAddr = sender_context.src_mac;
        let packet: Vec<u8> = icmp::create_all_nodes_echo_request_v6(src_mac, link_local)
            .context("Failed to create ICMPv6 echo request")?;
        Ok(packet)
    } else {
        Err(anyhow::anyhow!("Failed to create ICMPv6 packets: No link local address in context"))
    }
}


fn create_ndp_packet(_sender_context: &SenderContext) -> anyhow::Result<Vec<u8>> {
    anyhow::bail!("Ndp packet creation not possible as of now");
}


pub fn handle_frame(frame: &[u8]) -> anyhow::Result<Option<(MacAddr, IpAddr)>> {
    let eth = EthernetPacket::new(frame)
        .context("truncated or invalid Ethernet frame")?;
    let mac_addr: MacAddr = eth.get_source();
    let ip: Option<IpAddr> = match eth.get_ethertype() {
        EtherTypes::Arp => Some(arp::get_ip_addr(eth)?),
        EtherTypes::Ipv6 => ip::extract_source_addr_if_icmpv6(eth)?,
        _ => None,
    };
    if let Some(ip) = ip {
        return Ok(Some((mac_addr, ip)));
    }
    Ok(None)
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
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use pnet::ipnetwork::Ipv4Network;
    use pnet::packet::ethernet::EtherTypes;
    use pnet::util::MacAddr;
    use crate::net::datalink::ethernet;
    use crate::net::utils::MIN_ETH_FRAME_NO_FCS;

    pub static SHOULD_FAIL: AtomicBool = AtomicBool::new(false);

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
        let mut frame = vec![0u8; ETH_HDR_LEN + ARP_LEN - 1];
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
    
    fn setup() {
        SHOULD_FAIL.store(false, Ordering::SeqCst);
    }
    
    #[test]
    fn create_arp_packet_success() {
        let src_mac = MacAddr::new(0x01, 0x02, 0x03, 0x04, 0x05, 0x06);
        let src_net_str = "192.168.1.10/24";
        let dst_addr_str = "192.168.1.1";

        let src_net = src_net_str.parse::<Ipv4Network>().unwrap();
        let dst_addr = dst_addr_str.parse::<Ipv4Addr>().unwrap();

        let context = SenderContext {
            src_mac,
            ipv4_net: Some(src_net),
            dst_addr_v4: Some(dst_addr),
            ..Default::default()
        };
        
        let result = create_arp_packet(&context);  
        assert!(result.is_ok());
        let packet_bytes = result.unwrap();        
        use pnet::packet::arp::ArpPacket;
        use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
        use pnet::packet::Packet;

        let eth_packet = EthernetPacket::new(&packet_bytes)
            .expect("Test failed: could not parse Ethernet packet");

        assert_eq!(eth_packet.get_source(), src_mac);
        assert_eq!(eth_packet.get_destination(), MacAddr::broadcast());
        assert_eq!(eth_packet.get_ethertype(), EtherTypes::Arp);
        let arp_payload = eth_packet.payload();
        let arp_packet = ArpPacket::new(arp_payload)
            .expect("Test failed: could not parse ARP packet");
        assert_eq!(arp_packet.get_sender_hw_addr(), src_mac);
        assert_eq!(arp_packet.get_sender_proto_addr(), src_net.ip());
        assert_eq!(arp_packet.get_target_hw_addr(), MacAddr::broadcast());
        assert_eq!(arp_packet.get_target_proto_addr(), dst_addr);
    }
    
    #[test]
    fn create_arp_packet_missing_source_net() {
        setup();
        let context = SenderContext {
            ipv4_net: None, // The failure case
            dst_addr_v4: Some("192.168.1.1".parse().unwrap()),
            ..Default::default()
        };
        let result = create_arp_packet(&context);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing source IPv4 network for ARP packet"));
    }
    
    #[test]
    fn create_arp_packet_missing_dest_addr() {
        setup();
        let context = SenderContext {
            ipv4_net: Some("192.168.1.10/24".parse().unwrap()),
            dst_addr_v4: None, // The failure case
            ..Default::default()
        };
        let result = create_arp_packet(&context);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing destination IPv4 address for ARP packet"));
    }
}