pub mod arp;
mod ethernet;

use std::net::Ipv4Addr;
use anyhow::{anyhow, bail, Context};
use pnet::datalink::NetworkInterface;
use pnet::packet::arp::ArpPacket;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use pnet::util::MacAddr;
use crate::host::Host;
use crate::net::interface::get_ipv4;

const ETH_HDR_LEN: usize = 14;
const ARP_LEN: usize = 28;
const MIN_ETH_FRAME_NO_FCS: usize = 60;

#[derive(Clone, Copy, Debug)]
pub enum PacketType { ARP }

#[derive(Debug)]
pub enum CraftedPacket {
    ARP([u8; MIN_ETH_FRAME_NO_FCS]),
}

impl CraftedPacket {
    pub fn new(packet_type: PacketType, interface: &NetworkInterface, target_addr: Ipv4Addr)
        -> anyhow::Result<CraftedPacket> {
        match packet_type {
            PacketType::ARP => create_arp_request(interface, target_addr),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self { CraftedPacket::ARP(b) => b }
    }
}

fn create_arp_request(interface: &NetworkInterface, target_addr: Ipv4Addr)
                      -> anyhow::Result<CraftedPacket> {

    let mut pkt = [0u8; MIN_ETH_FRAME_NO_FCS];
    let src_mac = interface.mac.context("missing MAC on interface")?;

    let src_addr = get_ipv4(interface).map_err(|e| anyhow!(e))
        .context("failed to lookup IPv4 address for interface")?;
    ethernet::make_header(&mut pkt, src_mac, MacAddr::broadcast(), EtherTypes::Arp)?;
    arp::request_payload(&mut pkt, src_mac, src_addr, target_addr)?;
    Ok(CraftedPacket::ARP(pkt))
}

pub fn handle_frame(frame: &[u8]) -> anyhow::Result<Option<Host>> {
    let eth = EthernetPacket::new(frame)
        .context("truncated or invalid Ethernet frame")?;

    match eth.get_ethertype() {
        EtherTypes::Arp => {
            let payload = eth.payload();
            let arp = ArpPacket::new(payload)
                .with_context(|| format!(
                    "truncated or invalid ARP packet (payload len {})",
                    payload.len()
                ))?;
            arp::read(&arp) // -> anyhow::Result<Option<Host>>
        }
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

        arp::request_payload(&mut b, src_mac, src_ip, target_ip).unwrap();

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
    fn create_arp_request_errors_when_interface_missing_mac() {
        let iface = NetworkInterface {
            name: "lo".into(),
            description: "".to_string(),
            index: 0,
            mac: None,
            ips: vec![],
            flags: 0,
        };

        let err = create_arp_request(&iface, Ipv4Addr::new(192, 168, 1, 1))
            .unwrap_err();

        // check that the error message contains what we expect
        assert!(
            err.to_string().contains("missing MAC"),
            "unexpected error: {err:?}"
        );
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
