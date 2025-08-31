use std::net::Ipv4Addr;
use mac_oui::Oui;
use pnet::datalink::MacAddr;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::EtherTypes;
use crate::cmd::discover::Host;
use crate::net::packets::{PacketError, ARP_LEN, ETH_HDR_LEN};

pub fn request_payload(buffer: &mut [u8], src_mac: MacAddr, src_addr: Ipv4Addr, target_addr: Ipv4Addr)
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

pub fn read(arp: &ArpPacket, oui_db: &Oui) -> Option<Host> {
    if arp.get_operation() == ArpOperations::Reply {
        let vendor: String = match oui_db.lookup_by_mac(&arp.get_sender_hw_addr().to_string()) {
            Ok(Some(entry)) => entry.company_name.clone(),
            Ok(None)        => "Unknown".to_string(),
            Err(e) => {
                eprintln!("OUI lookup failed: {e}");
                "Unknown".to_string()
            }
        };
        let host = Host::new(
            arp.get_sender_proto_addr(),
            Some(vendor),
            Some(arp.get_sender_hw_addr())
        );
        Some(host)
    } else { None }
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
    use crate::net::packets::{PacketError, ARP_LEN, ETH_HDR_LEN};
    use crate::net::packets::arp::request_payload;
    use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket};
    use pnet::packet::ethernet::EtherTypes;

    #[test]
    fn arp_request_payload_errors_when_buffer_too_small() {
        // one byte short of ETH_HDR_LEN + ARP_LEN
        let mut small = vec![0u8; ETH_HDR_LEN + ARP_LEN - 1];
        let err = request_payload(
            &mut small,
            MacAddr::zero(),
            Ipv4Addr::new(1, 2, 3, 4),
            Ipv4Addr::new(5, 6, 7, 8),
        )
            .unwrap_err();
        matches!(err, PacketError::ArpBuffer);
    }

    #[test]
    fn arp_request_payload_succeeds_with_exact_min_len() {
        // exactly ETH_HDR_LEN + ARP_LEN
        let mut buf = vec![0u8; ETH_HDR_LEN + ARP_LEN];
        let src_mac = MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff);
        let src_ip = Ipv4Addr::new(10, 0, 0, 42);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        request_payload(&mut buf, src_mac, src_ip, dst_ip).expect("payload should fit");

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

        request_payload(&mut buf, src_mac, src_ip, dst_ip).unwrap();
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

        // Use an empty-ish OUI DB; if mac_oui::Oui::new() is available, use it;
        // otherwise fall back to Default (some versions impl Default).
        let oui_db = Oui::default().unwrap();

        // Should just do nothing (no panic)
        read(&arp, &oui_db);
    }
}