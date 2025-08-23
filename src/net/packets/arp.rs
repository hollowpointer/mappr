use std::net::Ipv4Addr;
use mac_oui::Oui;
use pnet::datalink::{DataLinkSender, MacAddr, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::EtherTypes;
use crate::cmd::discover::Host;
use crate::net::packets;
use crate::net::packets::{PacketError, ARP_LEN, ETH_HDR_LEN};
use crate::net::range::ip_iter;
use crate::print;

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

pub fn send_sweep(start: Ipv4Addr, end: Ipv4Addr, intf: &NetworkInterface, tx: &mut Box<dyn DataLinkSender>) {
    print::separator("ARP Network Scan");
    for ip in ip_iter((start, end)) {
        send(&intf, ip, tx).expect("Failed to perform ARP sweep");
    }
}

fn send(intf: &NetworkInterface, ip: Ipv4Addr, tx: &mut Box<dyn DataLinkSender>)
            -> anyhow::Result<()> {
    let pkt = packets::CraftedPacket::new(packets::PacketType::ARP, &intf, ip)?;
    if let Some(Err(e)) = tx.send_to(pkt.bytes(), Some(intf.clone())) {
        eprintln!("send {ip} failed: {e}");
    }
    Ok(())
}

pub fn read(arp: &ArpPacket, oui_db: &Oui) {
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
            vendor,
            arp.get_sender_proto_addr(),
            arp.get_sender_hw_addr()
        );
        host.print_lan();
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
    use std::net::Ipv4Addr;
    use pnet::datalink::{NetworkInterface, MacAddr};
    use crate::net::packets::{PacketError, ARP_LEN, ETH_HDR_LEN};
    use crate::net::packets::arp::request_payload;
    use std::io;
    use std::sync::{Arc, Mutex};
    use pnet::ipnetwork::{IpNetwork, Ipv4Network};
    use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket};
    use pnet::packet::ethernet::EtherTypes;

    // ---- Fake sender to spy on send_sweep ----
    struct FakeSender {
        sent: Arc<Mutex<usize>>,
        fail_first: bool,
        calls: usize,
    }

    impl FakeSender {
        fn new(fail_first: bool) -> (Box<dyn DataLinkSender>, Arc<Mutex<usize>>) {
            let sent = Arc::new(Mutex::new(0usize));
            let s = FakeSender { sent: sent.clone(), fail_first, calls: 0 };
            (Box::new(s), sent)
        }
    }

    impl DataLinkSender for FakeSender {
        fn build_and_send(
            &mut self,
            _num_packets: usize,
            _packet_size: usize,
            _func: &mut dyn for<'a> FnMut(&'a mut [u8]),
        ) -> Option<io::Result<()>> {
            // not used by our code-path
            Some(Ok(()))
        }

        fn send_to(
            &mut self,
            _packet: &[u8],
            _dst: Option<NetworkInterface>,
        ) -> Option<io::Result<()>> {
            self.calls += 1;
            *self.sent.lock().unwrap() += 1;
            if self.fail_first && self.calls == 1 {
                return Some(Err(io::Error::new(io::ErrorKind::Other, "boom")));
            }
            Some(Ok(()))
        }
    }

    fn dummy_iface() -> NetworkInterface {
        NetworkInterface {
            name: "test0".into(),
            description: "".to_string(),
            index: 1,
            mac: Some(MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff)),
            ips: vec![IpNetwork::V4(
                Ipv4Network::new(Ipv4Addr::new(192, 168, 1, 100), 24).unwrap()
            )],
            flags: 0,
        }
    }

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
    fn send_sweep_calls_send_to_for_each_ip_even_if_one_fails() {
        // 3 IPs in range
        let start = Ipv4Addr::new(192, 168, 1, 1);
        let end   = Ipv4Addr::new(192, 168, 1, 3);

        let intf = dummy_iface();
        let (mut tx, sent_counter) = FakeSender::new(true);

        // should not panic and should attempt all three sends
        send_sweep(start, end, &intf, &mut tx);

        let sent = *sent_counter.lock().unwrap();
        assert_eq!(sent, 3, "expected one send per IP in range");
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