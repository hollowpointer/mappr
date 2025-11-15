use std::net::{IpAddr, Ipv4Addr};
use anyhow::Context;
use pnet::datalink::MacAddr;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use crate::net::datalink::ethernet;
use crate::net::utils::{ETH_HDR_LEN, ARP_LEN, MIN_ETH_FRAME_NO_FCS};

pub fn create_packet(src_mac: MacAddr, dst_mac: MacAddr, src_addr: Ipv4Addr, dst_addr: Ipv4Addr)
                     -> anyhow::Result<Vec<u8>> {
    let mut buffer = [0u8; MIN_ETH_FRAME_NO_FCS];
    ethernet::make_header(&mut buffer, src_mac, MacAddr::broadcast(), EtherTypes::Arp)?;
    let mut arp_packet = MutableArpPacket::new(&mut buffer[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN])
        .context("failed to create mutable ARP packet")?;
    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);
    arp_packet.set_sender_hw_addr(src_mac);
    arp_packet.set_target_hw_addr(dst_mac);
    arp_packet.set_sender_proto_addr(src_addr);
    arp_packet.set_target_proto_addr(dst_addr);
    Ok(Vec::from(buffer))
}

pub fn get_ip_addr(ethernet_packet: EthernetPacket) -> anyhow::Result<IpAddr> {
    let arp_packet = ArpPacket::new(ethernet_packet.payload())
        .context(format!(
            "truncated or invalid ARP packet (payload len {})",
            ethernet_packet.payload().len()
        ))?;
    Ok(IpAddr::V4(arp_packet.get_sender_proto_addr()))
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
    use pnet::packet::arp::{ArpOperations, ArpPacket};
    use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
    use pnet::packet::Packet;
    use pnet::util::MacAddr;
    use std::net::Ipv4Addr;
    use pnet::packet::arp::ArpHardwareTypes;

    fn build_mock_arp_packet(sender_ip: Ipv4Addr, payload_size: usize) -> Vec<u8> {
        let buf_len = ETH_HDR_LEN + payload_size;
        let mut buffer = vec![0u8; buf_len];

        {
            let mut eth_pkt: MutableEthernetPacket = MutableEthernetPacket::new(&mut buffer).unwrap();
            eth_pkt.set_destination(MacAddr::broadcast());
            eth_pkt.set_source(MacAddr::new(0x01, 0x02, 0x03, 0x04, 0x05, 0x06));
            eth_pkt.set_ethertype(EtherTypes::Arp);
        }

        if payload_size >= ARP_LEN {
            let mut arp_pkt =
                MutableArpPacket::new(&mut buffer[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN]).unwrap();

            arp_pkt.set_hardware_type(ArpHardwareTypes::Ethernet);
            arp_pkt.set_protocol_type(EtherTypes::Ipv4);
            arp_pkt.set_hw_addr_len(6);
            arp_pkt.set_proto_addr_len(4);
            arp_pkt.set_operation(ArpOperations::Reply); // Or Request, doesn't matter
            arp_pkt.set_sender_hw_addr(MacAddr::new(0x01, 0x02, 0x03, 0x04, 0x05, 0x06));
            arp_pkt.set_sender_proto_addr(sender_ip);
            arp_pkt.set_target_hw_addr(MacAddr::zero());
            arp_pkt.set_target_proto_addr(Ipv4Addr::new(192, 168, 1, 1));
        }
        buffer
    }

    #[test]
    fn create_arp_request_packet() {
        let src_mac = MacAddr::new(0x01, 0x02, 0x03, 0x04, 0x05, 0x06);
        let dst_mac = MacAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF);
        let src_addr = Ipv4Addr::new(192, 168, 1, 10);
        let dst_addr = Ipv4Addr::new(192, 168, 1, 1);

        let buffer = create_packet(src_mac, dst_mac, src_addr, dst_addr)
            .expect("Packet creation failed");

        let eth_packet = EthernetPacket::new(&buffer)
            .expect("Failed to parse Ethernet packet");
        
        assert_eq!(eth_packet.get_destination(), MacAddr::broadcast()); 
        assert_eq!(eth_packet.get_source(), src_mac);
        assert_eq!(eth_packet.get_ethertype(), EtherTypes::Arp);
        let arp_payload = eth_packet.payload();
        let arp_packet = ArpPacket::new(arp_payload)
            .expect("Failed to parse ARP packet");
        assert_eq!(arp_packet.get_operation(), ArpOperations::Request);
        assert_eq!(arp_packet.get_hardware_type(), ArpHardwareTypes::Ethernet);
        assert_eq!(arp_packet.get_protocol_type(), EtherTypes::Ipv4);
        assert_eq!(arp_packet.get_hw_addr_len(), 6);
        assert_eq!(arp_packet.get_proto_addr_len(), 4);
        assert_eq!(arp_packet.get_sender_hw_addr(), src_mac);
        assert_eq!(arp_packet.get_sender_proto_addr(), src_addr);
        assert_eq!(arp_packet.get_target_hw_addr(), dst_mac);
        assert_eq!(arp_packet.get_target_proto_addr(), dst_addr);
    }
    
    #[test]
    fn test_get_ip_addr_success() {
        let expected_ip = Ipv4Addr::new(192, 168, 1, 123);
        let valid_arp_payload_size = ARP_LEN;
        let buffer = build_mock_arp_packet(expected_ip, valid_arp_payload_size);
        let ethernet_packet = EthernetPacket::new(&buffer).unwrap();
        let result = get_ip_addr(ethernet_packet);
        assert!(result.is_ok());
        let ip = result.unwrap();
        assert_eq!(ip, IpAddr::V4(expected_ip));
    }

    #[test]
    fn test_get_ip_addr_truncated_payload() {
        let truncated_payload_size = 10;
        let buffer = build_mock_arp_packet(Ipv4Addr::UNSPECIFIED, truncated_payload_size);
        let ethernet_packet = EthernetPacket::new(&buffer).unwrap();
        assert_eq!(ethernet_packet.payload().len(), truncated_payload_size);
        let result = get_ip_addr(ethernet_packet);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("truncated or invalid ARP packet"));
        assert!(err_msg.contains("(payload len 10)"));
    }

    #[test]
    fn test_get_ip_addr_wrong_payload_type() {
        const IPV4_PAYLOAD_SIZE: usize = 20; // 20 bytes for a header
        let mut buffer = build_mock_arp_packet(Ipv4Addr::UNSPECIFIED, IPV4_PAYLOAD_SIZE);
        let mut eth_pkt = MutableEthernetPacket::new(&mut buffer).unwrap();
        eth_pkt.set_ethertype(EtherTypes::Ipv4);
        let ethernet_packet = EthernetPacket::new(&buffer).unwrap();
        assert_eq!(ethernet_packet.payload().len(), IPV4_PAYLOAD_SIZE);
        let result = get_ip_addr(ethernet_packet);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("truncated or invalid ARP packet (payload len 20)"));
    }
}