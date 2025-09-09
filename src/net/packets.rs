pub mod arp;
mod ethernet;
pub mod icmp;
mod ip;

use anyhow::{bail, Context};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use crate::host::Host;

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
mod tests {
    use super::*;
    use pnet::packet::ethernet::EtherTypes;
    use pnet::util::MacAddr;
    use crate::net::utils::MIN_ETH_FRAME_NO_FCS;

    const ARP_LEN: usize = 28;
    const ETH_HDR_LEN: usize = 14;

    pub(crate) fn buf() -> [u8; MIN_ETH_FRAME_NO_FCS] {
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
