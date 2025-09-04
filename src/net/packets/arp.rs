use std::net::{IpAddr, Ipv4Addr};
use anyhow::{anyhow, Context};
use pnet::datalink::MacAddr;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::Packet;
use crate::host::Host;
use crate::net::utils::{ETH_HDR_LEN, ARP_LEN};

pub fn create_request_payload(
    buffer: &mut [u8],
    src_mac: MacAddr,
    src_addr: Ipv4Addr,
    target_addr: Ipv4Addr,
) -> anyhow::Result<()> {
    if ETH_HDR_LEN + ARP_LEN > buffer.len() {
        return Err(anyhow!("buffer too short for ARP payload"));
    }

    let mut arp_packet = MutableArpPacket::new(&mut buffer[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN])
        .context("failed to create mutable ARP packet")?;

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);
    arp_packet.set_sender_hw_addr(src_mac);
    arp_packet.set_target_hw_addr(MacAddr::new(0, 0, 0, 0, 0, 0));
    arp_packet.set_sender_proto_addr(src_addr);
    arp_packet.set_target_proto_addr(target_addr);

    Ok(())
}

pub fn handle_packet(ethernet_packet: EthernetPacket) -> anyhow::Result<Option<Host>> {
    let arp_packet = ArpPacket::new(ethernet_packet.payload())
        .context(format!(
            "truncated or invalid ARP packet (payload len {})",
            ethernet_packet.payload().len()
        ))?;
    read(&arp_packet)
}

fn read(arp_packet: &ArpPacket) -> anyhow::Result<Option<Host>> {
    if arp_packet.get_operation() == ArpOperations::Reply {
        let host = Host::new(
            IpAddr::V4(arp_packet.get_sender_proto_addr()),
            Some(arp_packet.get_sender_hw_addr()),
        );
        Ok(Some(host))
    } else { Ok(None) }
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
    use std::net::Ipv4Addr;
    use pnet::datalink::MacAddr;
    use crate::net::packets::arp::create_request_payload;
    use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket};
    use pnet::packet::ethernet::EtherTypes;

    #[test]
    fn arp_request_payload_errors_when_buffer_too_small() {
        // one byte short of ETH_HDR_LEN + ARP_LEN
        let mut small = vec![0u8; ETH_HDR_LEN + ARP_LEN - 1];

        let err = create_request_payload(
            &mut small,
            MacAddr::zero(),
            Ipv4Addr::new(1, 2, 3, 4),
            Ipv4Addr::new(5, 6, 7, 8),
        )
            .unwrap_err();

        assert!(
            err.to_string().contains("buffer"),
            "unexpected error: {err:?}"
        );
    }


    #[test]
    fn arp_request_payload_succeeds_with_exact_min_len() {
        // exactly ETH_HDR_LEN + ARP_LEN
        let mut buf = vec![0u8; ETH_HDR_LEN + ARP_LEN];
        let src_mac = MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff);
        let src_ip = Ipv4Addr::new(10, 0, 0, 42);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        create_request_payload(&mut buf, src_mac, src_ip, dst_ip).expect("payload should fit");

        // look only at the ARP slice the function wrote to
        let arp = ArpPacket::new(
            &buf[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN]
        ).expect("valid arp");

        assert_eq!(arp.get_operation(), ArpOperations::Request);
        assert_eq!(arp.get_hardware_type(), ArpHardwareTypes::Ethernet);
        assert_eq!(arp.get_protocol_type(), EtherTypes::Ipv4);
        assert_eq!(arp.get_hw_addr_len(), 6);
        assert_eq!(arp.get_proto_addr_len(), 4);

        assert_eq!(arp.get_sender_hw_addr(), src_mac);
        assert_eq!(arp.get_sender_proto_addr(), src_ip);
        assert_eq!(arp.get_target_proto_addr(), dst_ip);

        // target hw must be zeroed in ARP request
        assert_eq!(arp.get_target_hw_addr(), MacAddr::new(0,0,0,0,0,0));
    }

    #[test]
    fn arp_request_payload_sets_correct_fields_for_varied_inputs() {
        let mut buf = vec![0u8; ETH_HDR_LEN + ARP_LEN];
        let src_mac = MacAddr::new(1, 2, 3, 4, 5, 6);
        let src_ip = Ipv4Addr::new(192, 168, 1, 123);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 1);

        create_request_payload(&mut buf, src_mac, src_ip, dst_ip).unwrap();
        let arp = ArpPacket::new(
            &buf[ETH_HDR_LEN..ETH_HDR_LEN + ARP_LEN]
        ).unwrap();

        assert_eq!(arp.get_sender_hw_addr(), src_mac);
        assert_eq!(arp.get_sender_proto_addr(), src_ip);
        assert_eq!(arp.get_target_proto_addr(), dst_ip);
    }

    #[test]
    fn read_ignores_non_reply_packets_without_panicking() {
        // Build a minimal ARP *Request* packet into a buffer
        let mut buf = vec![0u8; ARP_LEN];
        {
            let mut arp = MutableArpPacket::new(&mut buf).unwrap();
            arp.set_hardware_type(ArpHardwareTypes::Ethernet);
            arp.set_protocol_type(EtherTypes::Ipv4);
            arp.set_hw_addr_len(6);
            arp.set_proto_addr_len(4);
            arp.set_operation(ArpOperations::Request); // not a Reply
            arp.set_sender_hw_addr(MacAddr::new(1,2,3,4,5,6));
            arp.set_target_hw_addr(MacAddr::new(0,0,0,0,0,0));
            arp.set_sender_proto_addr(Ipv4Addr::new(10,0,0,2));
            arp.set_target_proto_addr(Ipv4Addr::new(10,0,0,1));
        }
        let arp = ArpPacket::new(&buf).unwrap();

        // Should just do nothing (no panic)
        read(&arp).expect("Failed to read ARP");
    }
}