use crate::net::utils::{ETH_HDR_LEN, IP_V4_HDR_LEN, IP_V6_HDR_LEN};
use anyhow::Context;
use pnet::packet::Packet;
use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet, checksum};
use pnet::packet::ipv6::{Ipv6Packet, MutableIpv6Packet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const WORD_LEN: usize = 4;
const NO_FRAG_FLAG: u8 = 1 << 1;
const IP_V6_PACKET_LEN: usize = ETH_HDR_LEN + IP_V6_HDR_LEN;
const IP_V4_PACKET_LEN: usize = ETH_HDR_LEN + IP_V4_HDR_LEN;

pub fn make_ipv6_header(
    src_addr: Ipv6Addr,
    dst_addr: Ipv6Addr,
    payload_length: u16,
    next_protocol: IpNextHeaderProtocol,
    buffer: &mut [u8],
) -> anyhow::Result<()> {
    let mut pkt = MutableIpv6Packet::new(&mut buffer[ETH_HDR_LEN..IP_V6_PACKET_LEN])
        .context("creating ipv6 packet")?;
    pkt.set_version(6);
    pkt.set_traffic_class(0);
    pkt.set_flow_label(rand::random());
    pkt.set_payload_length(payload_length);
    pkt.set_next_header(next_protocol);
    pkt.set_hop_limit(1);
    pkt.set_source(src_addr);
    pkt.set_destination(dst_addr);
    Ok(())
}

pub fn create_ipv4_header(
    src_addr: Ipv4Addr,
    dst_addr: Ipv4Addr,
    total_length: u16,
    next_protocol: IpNextHeaderProtocol,
    buffer: &mut [u8],
) -> anyhow::Result<()> {
    let mut ipv4 = MutableIpv4Packet::new(&mut buffer[ETH_HDR_LEN..IP_V4_PACKET_LEN])
        .context("creating ipv4 packet")?;
    ipv4.set_version(4);
    ipv4.set_header_length((IP_V4_HDR_LEN / WORD_LEN) as u8);
    ipv4.set_dscp(0);
    ipv4.set_ecn(0);
    ipv4.set_total_length(total_length);
    ipv4.set_identification(rand::random());
    ipv4.set_flags(NO_FRAG_FLAG);
    ipv4.set_fragment_offset(0);
    ipv4.set_ttl(64);
    ipv4.set_next_level_protocol(next_protocol);
    ipv4.set_source(src_addr);
    ipv4.set_destination(dst_addr);

    ipv4.set_checksum(0);
    let ipv4_imm = ipv4.to_immutable();
    let ipv4_pkt = Ipv4Packet::new(ipv4_imm.packet()).context("transforming ipv4 to packet")?;
    let csm = checksum(&ipv4_pkt);
    ipv4.set_checksum(csm);
    Ok(())
}

pub fn extract_addr_if_icmpv6(eth_packet: EthernetPacket) -> anyhow::Result<Option<IpAddr>> {
    let ipv6_packet: Ipv6Packet = Ipv6Packet::new(eth_packet.payload()).context(format!(
        "truncated or invalid IPv6 packet (payload len {})",
        eth_packet.payload().len()
    ))?;
    if ipv6_packet.get_next_header() == IpNextHeaderProtocols::Icmpv6 {
        let src_addr: IpAddr = IpAddr::V6(ipv6_packet.get_source());
        return Ok(Some(src_addr));
    }
    Ok(None)
}
