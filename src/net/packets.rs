pub mod icmp;
mod ip;
pub mod tcp;

use std::net::{Ipv4Addr, Ipv6Addr};
use anyhow::{Context, Ok, bail};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::util::MacAddr;
use crate::host::Host;
use crate::net::datalink::arp;
use crate::net::range::{self, Ipv4Range};
use crate::utils::print;

pub fn create_packets(
    src_mac: MacAddr,
    src_addr_v4: Option<Ipv4Addr>,
    ipv4_range: Option<Ipv4Range>,
    link_local_addr: Option<Ipv6Addr>,
) -> anyhow::Result<Vec<Vec<u8>>> {
    match (src_addr_v4, ipv4_range, link_local_addr) {
        // Case 1: Full IPv4 and IPv6 capabilities
        (Some(src_addr_v4), Some(ipv4_range), Some(src_addr_v6)) => {
            print::print_status("Root: Performing L2/L3 (IPv4 + IPv6) scan...");
            let dst_mac: MacAddr = MacAddr::broadcast();
            let mut packets: Vec<Vec<u8>> = range::ip_iter(&ipv4_range)
            .map(|dst_addr| {
                arp::create_packet(src_mac, dst_mac, src_addr_v4, dst_addr)
            })
            .collect::<Result<Vec<Vec<u8>>, _>>()?;
            packets.extend(icmp::create_all_nodes_echo_request_v6(src_mac, src_addr_v6));
            Ok(packets)
        },
        // Case 2: Only IPv4 capabilities
        (Some(src_addr_v4), Some(ipv4_range), None) => {
            print::print_status("Root: Performing L2/L3 (IPv4-only) scan...");
            let dst_mac: MacAddr = MacAddr::broadcast();
            let packets: Vec<Vec<u8>> = range::ip_iter(&ipv4_range)
            .map(|dst_addr| {
                arp::create_packet(src_mac, dst_mac, src_addr_v4, dst_addr)
            })
            .collect::<Result<Vec<Vec<u8>>, _>>()?;
            Ok(packets)
        },

        // Case 3: Only IPv6 capabilities (e.g., IPv4 missing address or range)
        // This arm catches (None, _, Some) and (Some, None, Some)
        (_, _, Some(src_addr_v6)) => {
            print::print_status("Performing L2/L3 (IPv6-only) scan...");
            let packets: Vec<Vec<u8>> = vec![icmp::create_all_nodes_echo_request_v6(src_mac, src_addr_v6)?];
            Ok(packets)
        },
        // Case 4: No usable addresses found
        _ => {
            anyhow::bail!("No usable IPv4 or IPv6 addresses found on interface.")
        }
    }
}

pub fn handle_frame(frame: &[u8]) -> anyhow::Result<Option<Host>> {
    let eth = EthernetPacket::new(frame)
        .context("truncated or invalid Ethernet frame")?;
    let mac_addr = eth.get_source();
    let host = match eth.get_ethertype() {
        EtherTypes::Arp => { arp::handle_packet(eth)? },
        EtherTypes::Ipv6 => { ip::handle_v6_packet(eth)? },
        other => bail!("unsupported ethertype: 0x{:04x}", other.0),
    };
    if let Some(mut host) = host { host.set_mac_addr(mac_addr)?; Ok(Some(host)) } else { Ok(None) }
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
pub mod tests {
    use super::*;
    use pnet::packet::ethernet::EtherTypes;
    use pnet::util::MacAddr;
    use crate::net::datalink::ethernet;
    use crate::net::utils::MIN_ETH_FRAME_NO_FCS;

    const ARP_LEN: usize = 28;
    const ETH_HDR_LEN: usize = 14;

    pub fn buf() -> [u8; MIN_ETH_FRAME_NO_FCS] {
        [0u8; MIN_ETH_FRAME_NO_FCS]
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
