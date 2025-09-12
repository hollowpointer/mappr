use std::net::Ipv6Addr;
use anyhow::Context;
use pnet::datalink::MacAddr;
use pnet::packet::{icmpv6, Packet};
use pnet::packet::ethernet::EtherTypes;
use pnet::packet::icmpv6::{Icmpv6Code, Icmpv6Type, checksum, Icmpv6Packet};
use crate::net::datalink::channel::SenderContext;
use crate::net::datalink::ethernet;
use crate::net::packets::ip;
use crate::net::utils::{ETH_HDR_LEN, IP_V6_HDR_LEN, ICMP_V6_ECHO_REQ_LEN};

pub fn send_echo_request_v6(sender_context: &mut SenderContext) -> anyhow::Result<()> {
    let echo_req_pkt = create_echo_request_v6(sender_context)?;
    sender_context.tx.send_to(echo_req_pkt.as_slice(), None);
    Ok(())
}

fn create_echo_request_v6(sender_context: &SenderContext) -> anyhow::Result<Vec<u8>> {
    let mut pkt = [0u8; ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN];
    let multicast_mac_addr = MacAddr::new(0x33, 0x33, 0, 0, 0, 1);
    let ll_multicast_addr = Ipv6Addr::new(0xff02, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1);
    ethernet::make_header(&mut pkt, sender_context.mac_addr, multicast_mac_addr, EtherTypes::Ipv6)?;
    ip::create_ipv6_header(&mut pkt, sender_context.src_addr_v6, ll_multicast_addr)?;
    let icmp_payload_start = ETH_HDR_LEN + IP_V6_HDR_LEN;
    let icmp_payload_end = ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN;
    let mut icmp = icmpv6::echo_request::
    MutableEchoRequestPacket::new(&mut pkt[icmp_payload_start..icmp_payload_end]).context(
        "failed to create echo request packet")?;

    icmp.set_icmpv6_type(Icmpv6Type(128));
    icmp.set_icmpv6_code(Icmpv6Code(0));
    icmp.set_identifier(rand::random());
    icmp.set_sequence_number(0);

    icmp.set_checksum(0);
    let icmp_imm = icmp.to_immutable();
    let icmp_pkt = Icmpv6Packet::new(icmp_imm.packet()).context("failed to create ICMPv6 packet")?;
    let csm = checksum(&icmp_pkt, &sender_context.src_addr_v6, &ll_multicast_addr);
    icmp.set_checksum(csm);
    Ok(Vec::from(pkt))
}