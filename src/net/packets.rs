pub mod arp;
mod ethernet;
mod icmp;
mod ip;

use std::net::{IpAddr, Ipv6Addr};
use anyhow::{bail, Context};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ipv6::MutableIpv6Packet;
use pnet::util::MacAddr;
use crate::host::Host;
use crate::net::utils::*;

pub enum PacketType {
    ARP,
    EchoRequestV6
}

pub fn handle_frame(frame: &[u8]) -> anyhow::Result<Option<Host>> {
    let eth = EthernetPacket::new(frame)
        .context("truncated or invalid Ethernet frame")?;
    let mac_addr = eth.get_source();
    let host = match eth.get_ethertype() {
        EtherTypes::Arp => { arp::handle_packet(eth)? },
        EtherTypes::Ipv6 => { ip::handle_v6_packet(eth)? },
        other => bail!("unsupported ethertype: 0x{:04x}", other.0),
    };
    if let Some(mut host) = host { host.set_mac_addr(mac_addr)?; Ok(Some(host)) } else { Ok(None) }
}

fn create_echo_request_v6(src_mac: MacAddr, src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> anyhow::Result<Vec<u8>> {
    let mut pkt = [0u8; ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN];
    let multicast_mac_addr = MacAddr::new(0x33, 0x33, 0, 0, 0, 1);
    ethernet::make_header(&mut pkt, src_mac, multicast_mac_addr, EtherTypes::Ipv6)?;
    create_ipv6_header(&mut pkt, src_addr, dst_addr)?;
    icmp::create_echo_request_v6(&mut pkt, src_addr, dst_addr)?;

    Ok(Vec::from(pkt))
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
