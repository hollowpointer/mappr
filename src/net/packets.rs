use thiserror::Error;
use std::net::Ipv4Addr;
use pnet::datalink::NetworkInterface;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket};
use pnet::packet::ethernet::{EtherType, EtherTypes, MutableEthernetPacket};
use pnet::util::MacAddr;
use crate::net::interface::get_ipv4;

const ETH_HDR_LEN: usize = 14;
const ARP_LEN: usize = 28;
const MIN_ETH_FRAME_NO_FCS: usize = 60;

pub enum PacketType { ARP }

#[derive(Debug, Error)]
pub enum PacketError {
    #[error("Interface missing MAC address")]
    MissingMac,

    #[error("Ethernet buffer too small")]
    EthernetBuffer,

    #[error("ARP buffer too small or invalid slice")]
    ArpBuffer,

    #[error("IPv4 address retrieval failed: {0}")]
    IpLookup(String),
}

pub enum Packet {
    ARP([u8; MIN_ETH_FRAME_NO_FCS]),
}

impl Packet {
    pub fn new(packet_type: PacketType, interface: &NetworkInterface, target_addr: Ipv4Addr)
        -> Result<Packet, PacketError> {
        match packet_type {
            PacketType::ARP => create_arp_request(interface, target_addr),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self { Packet::ARP(b) => b }
    }
}

fn create_arp_request(interface: &NetworkInterface, target_addr: Ipv4Addr)
    -> Result<Packet, PacketError> {

    let mut pkt = [0u8; MIN_ETH_FRAME_NO_FCS];
    let src_mac = interface.mac.ok_or(PacketError::MissingMac)?;
    let src_addr = get_ipv4(&interface).map_err(PacketError::IpLookup)?;

    ethernet_header(&mut pkt, src_mac, MacAddr::broadcast(), EtherTypes::Arp)?;
    arp_request_payload(&mut pkt, src_mac, src_addr, target_addr)?;
    Ok(Packet::ARP(pkt))
}

fn ethernet_header(buffer: &mut [u8], src_mac: MacAddr, dst_mac: MacAddr, et: EtherType)
    -> Result<(), PacketError>{
    let mut eth =
        MutableEthernetPacket::new(&mut buffer[..]).ok_or(PacketError::EthernetBuffer)?;
    eth.set_source(src_mac);
    eth.set_destination(dst_mac);
    eth.set_ethertype(et);
    Ok(())
}

fn arp_request_payload(buffer: &mut [u8], src_mac: MacAddr, src_addr: Ipv4Addr, target_addr: Ipv4Addr)
    -> Result<(), PacketError>{
    if ETH_HDR_LEN + ARP_LEN > buffer.len() { return Err(PacketError::ArpBuffer); }
    let mut arp = MutableArpPacket::new
        (&mut buffer[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN]).ok_or(PacketError::ArpBuffer)?;
    arp.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp.set_protocol_type(EtherTypes::Ipv4);
    arp.set_hw_addr_len(6);
    arp.set_proto_addr_len(4);
    arp.set_operation(ArpOperations::Request);
    arp.set_sender_hw_addr(src_mac);
    arp.set_target_hw_addr(MacAddr::new(0,0,0,0,0,0));
    arp.set_sender_proto_addr(src_addr);
    arp.set_target_proto_addr(target_addr);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::packet::arp::ArpPacket;
    use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
    use pnet::util::MacAddr;
    use std::net::Ipv4Addr;

    fn buf() -> [u8; MIN_ETH_FRAME_NO_FCS] {
        [0u8; MIN_ETH_FRAME_NO_FCS]
    }

    #[test]
    fn ethernet_header_sets_fields() {
        let mut b = buf();
        let src = MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
        let dst = MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff);

        ethernet_header(&mut b, src, dst, EtherTypes::Ipv4).unwrap();

        let eth = EthernetPacket::new(&b[..ETH_HDR_LEN]).expect("parse eth");
        assert_eq!(eth.get_source(), src);
        assert_eq!(eth.get_destination(), dst);
        assert_eq!(eth.get_ethertype(), EtherTypes::Ipv4);
    }

    #[test]
    fn ethernet_header_errors_when_buffer_too_small() {
        let mut tiny: [u8; 0] = [];
        let err = ethernet_header(&mut tiny, MacAddr::zero(), MacAddr::zero(), EtherTypes::Arp)
            .unwrap_err();
        matches!(err, PacketError::EthernetBuffer);
    }

    #[test]
    fn arp_request_payload_sets_fields() {
        let mut b = buf();

        // ethernet header can be anything; ARP parser reads after ETH_HDR_LEN.
        ethernet_header(&mut b, MacAddr::zero(), MacAddr::broadcast(), EtherTypes::Arp).unwrap();

        let src_mac = MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01);
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let target_ip = Ipv4Addr::new(192, 168, 1, 20);

        arp_request_payload(&mut b, src_mac, src_ip, target_ip).unwrap();

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
    fn arp_request_payload_errors_when_buffer_too_small() {
        // one byte short of ETH_HDR_LEN + ARP_LEN
        let mut small = vec![0u8; ETH_HDR_LEN + ARP_LEN - 1];
        let err = arp_request_payload(
            &mut small,
            MacAddr::zero(),
            Ipv4Addr::new(1, 2, 3, 4),
            Ipv4Addr::new(5, 6, 7, 8),
        )
            .unwrap_err();
        matches!(err, PacketError::ArpBuffer);
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
            .err()
            .expect("expected error");
        assert!(matches!(err, PacketError::MissingMac));
    }
}
