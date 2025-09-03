pub mod arp;
mod ethernet;
mod icmp;

use std::net::{Ipv4Addr, Ipv6Addr};
use anyhow::{bail, Context};
use pnet::datalink::NetworkInterface;
use pnet::packet::arp::ArpPacket;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ipv6::MutableIpv6Packet;
use pnet::packet::Packet;
use pnet::util::MacAddr;
use crate::host::Host;
use crate::net::interface;
use crate::net::utils::*;

#[derive(Clone, Copy, Debug)]
pub enum PacketType { ARP, _EchoRequestV6 }

#[derive(Debug)]
pub enum CraftedPacket {
    ARP([u8; MIN_ETH_FRAME_NO_FCS]),
    EchoRequestV6([u8; ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN]),
}

impl CraftedPacket {
    pub fn new(packet_type: PacketType, interface: &NetworkInterface, target_addr: Ipv4Addr)
        -> anyhow::Result<CraftedPacket> {
        let src_mac: MacAddr = interface.mac.context("failed to retrieve mac address")?;
        let src_addr: Ipv4Addr = interface::get_ipv4(interface).context("failed to fetch IPv4 address for interface")?;
        let sr_addr_v6 = Ipv6Addr::new(0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0);
        // interface::get_ipv6(interface).context("failed to fetch IPv6 address for interface")?;
        let multicast_ipv6_addr: Ipv6Addr = Ipv6Addr::new(0xff02, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1);
        match packet_type {
            PacketType::ARP => create_arp_request(src_mac, src_addr, target_addr),
            PacketType::_EchoRequestV6 => create_echo_request_v6(src_mac, sr_addr_v6, multicast_ipv6_addr)
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self {
            CraftedPacket::ARP(b) => b,
            CraftedPacket::EchoRequestV6(b) => b,
        }
    }
}

fn create_arp_request(src_mac: MacAddr, src_addr: Ipv4Addr, target_addr: Ipv4Addr)
                      -> anyhow::Result<CraftedPacket> {

    let mut pkt = [0u8; MIN_ETH_FRAME_NO_FCS];
    ethernet::make_header(&mut pkt, src_mac, MacAddr::broadcast(), EtherTypes::Arp)?;
    arp::create_request_payload(&mut pkt, src_mac, src_addr, target_addr)?;
    Ok(CraftedPacket::ARP(pkt))
}

fn create_echo_request_v6(src_mac: MacAddr, src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> anyhow::Result<CraftedPacket> {
    println!("{}", src_addr);
    let mut pkt = [0u8; ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN];
    let multicast_mac_addr = MacAddr::new(0x33, 0x33, 0, 0, 0, 1);
    ethernet::make_header(&mut pkt, src_mac, multicast_mac_addr, EtherTypes::Ipv6)?;
    create_ipv6_header(&mut pkt, src_addr, dst_addr)?;
    icmp::create_echo_request_v6(&mut pkt, src_addr, dst_addr)?;

    Ok(CraftedPacket::EchoRequestV6(pkt))
}

fn create_ipv6_header(buf: &mut[u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> anyhow::Result<()> {
    let mut pkt = MutableIpv6Packet::new(
        &mut buf[ETH_HDR_LEN..ETH_HDR_LEN+IP_V6_HDR_LEN]
    ).context("failed to create Ipv6 packet")?;
    pkt.set_version(6);
    pkt.set_traffic_class(0);
    pkt.set_flow_label(0xC000F);
    pkt.set_payload_length(ICMP_V6_ECHO_REQ_LEN as u16);
    pkt.set_next_header(IpNextHeaderProtocol(58));
    pkt.set_hop_limit(1);
    pkt.set_source(src_addr);
    pkt.set_destination(dst_addr);
    Ok(())
}

pub fn handle_frame(frame: &[u8]) -> anyhow::Result<Option<Host>> {
    let eth = EthernetPacket::new(frame)
        .context("truncated or invalid Ethernet frame")?;

    let payload = eth.payload();
    match eth.get_ethertype() {
        EtherTypes::Arp => {
            let arp_packet = ArpPacket::new(payload)
                .context(format!(
                    "truncated or invalid ARP packet (payload len {})",
                    payload.len()
                ))?;
            arp::read(&arp_packet)
        },
        other => bail!("unsupported ethertype: 0x{:04x}", other.0),
    }
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
mod tests {
    use super::*;
    use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket};
    use pnet::packet::ethernet::EtherTypes;
    use pnet::util::MacAddr;
    use std::net::Ipv4Addr;

    const ARP_LEN: usize = 28;
    const ETH_HDR_LEN: usize = 14;

    pub(crate) fn buf() -> [u8; MIN_ETH_FRAME_NO_FCS] {
        [0u8; MIN_ETH_FRAME_NO_FCS]
    }

    #[test]
    fn arp_request_payload_sets_fields() {
        let mut b = buf();

        // ethernet header can be anything; ARP parser reads after ETH_HDR_LEN.
        ethernet::make_header(&mut b, MacAddr::zero(), MacAddr::broadcast(), EtherTypes::Arp).unwrap();

        let src_mac = MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01);
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let target_ip = Ipv4Addr::new(192, 168, 1, 20);

        arp::create_request_payload(&mut b, src_mac, src_ip, target_ip).unwrap();

        let arp = ArpPacket::new(&b[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN]).expect("parse arp");
        assert_eq!(arp.get_hardware_type(), ArpHardwareTypes::Ethernet);
        assert_eq!(arp.get_protocol_type(), EtherTypes::Ipv4);
        assert_eq!(arp.get_hw_addr_len(), 6);
        assert_eq!(arp.get_proto_addr_len(), 4);
        assert_eq!(arp.get_operation(), ArpOperations::Request);
        assert_eq!(arp.get_sender_hw_addr(), src_mac);
        assert_eq!(arp.get_target_hw_addr(), MacAddr::new(0, 0, 0, 0, 0, 0));
        assert_eq!(arp.get_sender_proto_addr(), src_ip);
        assert_eq!(arp.get_target_proto_addr(), target_ip);
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
