use std::net::Ipv6Addr;
use anyhow::Context;
use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ipv6::{Ipv6Packet, MutableIpv6Packet};
use pnet::packet::Packet;
use crate::host::Host;
use crate::net::utils::{ETH_HDR_LEN, ICMP_V6_ECHO_REQ_LEN, IP_V6_HDR_LEN};

const ICMP_NEXT_HEADER_CODE: u8 = 58;

pub fn create_ipv6_header(buf: &mut[u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> anyhow::Result<()> {
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

pub fn handle_v6_packet(ethernet_packet: EthernetPacket) -> anyhow::Result<Option<Host>> {
    let ipv6_packet = Ipv6Packet::new(ethernet_packet.payload())
        .context(format!(
            "truncated or invalid IPv6 packet (payload len {})",
            ethernet_packet.payload().len()
        ))?;
    read_v6(&ipv6_packet)
}

fn read_v6(ipv6_packet: &Ipv6Packet) -> anyhow::Result<Option<Host>> {
    let src_addr = ipv6_packet.get_source();
    let host: Option<Host> = match ipv6_packet.get_next_header() {
        IpNextHeaderProtocol(ICMP_NEXT_HEADER_CODE) => {
            let mut host = Host::default();
            host.add_ipv6(src_addr);
            Some(host)
        },
        _ => { None }
    };
    Ok(host)
}