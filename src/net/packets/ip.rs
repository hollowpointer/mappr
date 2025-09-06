use anyhow::Context;
use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::Packet;
use crate::host::Host;

const ICMP_NEXT_HEADER_CODE: u8 = 58;

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