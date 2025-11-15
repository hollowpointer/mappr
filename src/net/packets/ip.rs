use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use anyhow::Context;
use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::{checksum, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::ipv6::{Ipv6Packet, MutableIpv6Packet};
use pnet::packet::Packet;
use crate::net::utils::{ETH_HDR_LEN, ICMP_V6_ECHO_REQ_LEN, IP_V6_HDR_LEN};

pub fn create_ipv6_header(buf: &mut[u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> anyhow::Result<()> {
    let mut pkt = MutableIpv6Packet::new(
        &mut buf[ETH_HDR_LEN..ETH_HDR_LEN+IP_V6_HDR_LEN]).context("creating ipv6 packet")?;
    pkt.set_version(6);
    pkt.set_traffic_class(0);
    pkt.set_flow_label(rand::random()); // Don't care
    pkt.set_payload_length(ICMP_V6_ECHO_REQ_LEN as u16);
    pkt.set_next_header(IpNextHeaderProtocol(58));
    pkt.set_hop_limit(1);
    pkt.set_source(src_addr);
    pkt.set_destination(dst_addr);
    Ok(())
}

pub fn _create_ipv4_header(buf: &mut[u8],
                          total_length:  u16,
                          nxt_ptc: IpNextHeaderProtocol,
                          src_addr: Ipv4Addr,
                          dst_addr: Ipv4Addr
) -> anyhow::Result<()> {
    let mut ipv4 = MutableIpv4Packet::new(
        &mut buf[..20]).context("creating ipv4 packet")?;
    ipv4.set_version(4);
    ipv4.set_header_length(5); // "The minimum value for this field is 5, which indicates a length of 5 × 32 bits = 160 bits = 20 bytes."
    ipv4.set_dscp(0); // https://en.wikipedia.org/wiki/Differentiated_services
    ipv4.set_ecn(0); // https://en.wikipedia.org/wiki/Explicit_Congestion_Notification 0 for highest compatability
    ipv4.set_total_length(total_length);
    ipv4.set_identification(rand::random()); // Don't care
    ipv4.set_flags(2); // Do not fragment (010)
    ipv4.set_fragment_offset(0); // 0 since we do not fragment
    ipv4.set_ttl(64); // Typical value (I guess)
    ipv4.set_next_level_protocol(nxt_ptc);
    ipv4.set_source(src_addr);
    ipv4.set_destination(dst_addr);

    ipv4.set_checksum(0);
    let ipv4_imm = ipv4.to_immutable();
    let ipv4_pkt = Ipv4Packet::new(ipv4_imm.packet()).context("transforming ipv4 to packet")?;
    let csm = checksum(&ipv4_pkt);
    ipv4.set_checksum(csm);
    Ok(())
}

pub fn extract_source_addr_if_icmpv6(eth_packet: EthernetPacket) -> anyhow::Result<Option<IpAddr>> {
    let ipv6_packet: Ipv6Packet = Ipv6Packet::new(eth_packet.payload())
        .context(format!(
            "truncated or invalid IPv6 packet (payload len {})",
            eth_packet.payload().len()
        ))?;
    if ipv6_packet.get_next_header() == IpNextHeaderProtocols::Icmpv6 {
        let src_addr: IpAddr = IpAddr::V6(ipv6_packet.get_source());
        return Ok(Some(src_addr));
    }
    Ok(None)
}