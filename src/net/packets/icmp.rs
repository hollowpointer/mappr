use std::net::Ipv6Addr;
use anyhow::Context;
use pnet::packet::{icmpv6, Packet};
use pnet::packet::icmpv6::{Icmpv6Code, Icmpv6Type, checksum, Icmpv6Packet};
use crate::net::utils::{ETH_HDR_LEN, IP_V6_HDR_LEN, ICMP_V6_ECHO_REQ_LEN};

pub fn create_echo_request_v6(buffer: &mut [u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> anyhow::Result<()> {
    let icmp_start = ETH_HDR_LEN + IP_V6_HDR_LEN;
    let icmp_end = ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN;
    let mut icmp = icmpv6::echo_request::
    MutableEchoRequestPacket::new(&mut buffer[icmp_start..icmp_end]).context(
        "failed to create echo request packet")?;

    icmp.set_icmpv6_type(Icmpv6Type(128));
    icmp.set_icmpv6_code(Icmpv6Code(0));
    icmp.set_identifier(rand::random());
    icmp.set_sequence_number(0);

    icmp.set_checksum(0);
    let icmp_imm = icmp.to_immutable();
    let icmp_pkt = Icmpv6Packet::new(icmp_imm.packet()).context("failed to create ICMPv6 packet")?;
    let csm = checksum(&icmp_pkt, &src_addr, &dst_addr);
    icmp.set_checksum(csm);
    Ok(())
}