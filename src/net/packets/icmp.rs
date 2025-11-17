use crate::net::datalink::ethernet;
use crate::net::packets::ip;
use crate::net::utils::{ETH_HDR_LEN, ICMP_V6_ECHO_REQ_LEN, IP_V6_HDR_LEN};
use anyhow::Context;
use pnet::datalink::MacAddr;
use pnet::packet::Packet;
use pnet::packet::ethernet::EtherTypes;
use pnet::packet::icmpv6::echo_reply::Icmpv6Codes;
use pnet::packet::icmpv6::echo_request::{EchoRequestPacket, MutableEchoRequestPacket};
use pnet::packet::icmpv6::{Icmpv6Packet, Icmpv6Types, checksum};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use std::net::Ipv6Addr;

const TOTAL_LEN: usize = ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN;
const ICMP_PAYLOAD_START: usize = ETH_HDR_LEN + IP_V6_HDR_LEN;
const ICMP_PAYLOAD_END: usize = ETH_HDR_LEN + IP_V6_HDR_LEN + ICMP_V6_ECHO_REQ_LEN;
const PAYLOAD_LENGTH: u16 = ICMP_V6_ECHO_REQ_LEN as u16;
const NEXT_PROTOCOL: IpNextHeaderProtocol = IpNextHeaderProtocols::Icmpv6;

pub fn create_all_nodes_echo_request_v6(
    src_mac: MacAddr,
    src_addr: Ipv6Addr,
) -> anyhow::Result<Vec<u8>> {
    let mut buffer: [u8; 62] = [0u8; TOTAL_LEN];
    let dst_mac: MacAddr = MacAddr::new(0x33, 0x33, 0, 0, 0, 1);
    let dst_addr: Ipv6Addr = Ipv6Addr::new(0xff02, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1);

    ethernet::make_header(&mut buffer, src_mac, dst_mac, EtherTypes::Ipv6)?;
    ip::make_ipv6_header(
        src_addr,
        dst_addr,
        PAYLOAD_LENGTH,
        NEXT_PROTOCOL,
        &mut buffer,
    )?;

    let mut icmp: MutableEchoRequestPacket =
        MutableEchoRequestPacket::new(&mut buffer[ICMP_PAYLOAD_START..ICMP_PAYLOAD_END])
            .context("failed to create echo request packet")?;

    icmp.set_icmpv6_type(Icmpv6Types::EchoRequest);
    icmp.set_icmpv6_code(Icmpv6Codes::NoCode);
    icmp.set_identifier(rand::random());
    icmp.set_sequence_number(0);

    let icmp_imm: EchoRequestPacket = icmp.to_immutable();
    let icmp_pkt: Icmpv6Packet =
        Icmpv6Packet::new(icmp_imm.packet()).context("failed to create ICMPv6 packet")?;
    let csm = checksum(&icmp_pkt, &src_addr, &dst_addr);
    icmp.set_checksum(csm);

    Ok(Vec::from(buffer))
}
