use std::net::Ipv4Addr;
use mac_oui::Oui;
use pnet::datalink::{DataLinkSender, MacAddr, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::EtherTypes;
use crate::cmd::discover::Host;
use crate::net::packets;
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

pub fn send(intf: &NetworkInterface, ip: Ipv4Addr, tx: &mut Box<dyn DataLinkSender>)
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
    use std::net::Ipv4Addr;
    use pnet::datalink::MacAddr;
    use crate::net::packets::{PacketError, ARP_LEN, ETH_HDR_LEN};
    use crate::net::packets::arp::request_payload;

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
}