use anyhow::Context;
use pnet::datalink::MacAddr;
use pnet::packet::ethernet::{EtherType, MutableEthernetPacket};

pub fn make_header(
    buffer: &mut [u8],
    src_mac: MacAddr,
    dst_mac: MacAddr,
    et: EtherType,
) -> anyhow::Result<()> {
    let mut eth = MutableEthernetPacket::new(&mut buffer[..])
        .context("failed to create mutable Ethernet packet")?;

    eth.set_source(src_mac);
    eth.set_destination(dst_mac);
    eth.set_ethertype(et);

    Ok(())
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
    const ETH_HDR_LEN: usize = 14;
    use crate::net::datalink::ethernet::make_header;
    use pnet::datalink::MacAddr;
    use pnet::packet::ethernet::{EtherTypes, EthernetPacket};

    #[test]
    fn ethernet_header_sets_fields() {
        let mut b = crate::net::packets::tests::buf();
        let src = MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
        let dst = MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff);

        make_header(&mut b, src, dst, EtherTypes::Ipv4).unwrap();

        let eth = EthernetPacket::new(&b[..ETH_HDR_LEN]).expect("parse eth");
        assert_eq!(eth.get_source(), src);
        assert_eq!(eth.get_destination(), dst);
        assert_eq!(eth.get_ethertype(), EtherTypes::Ipv4);
    }

    #[test]
    fn ethernet_header_errors_when_buffer_too_small() {
        let mut tiny: [u8; 0] = [];

        let err =
            make_header(&mut tiny, MacAddr::zero(), MacAddr::zero(), EtherTypes::Arp).unwrap_err();

        assert!(
            err.to_string().contains("Ethernet"),
            "unexpected error: {err:?}"
        );
    }
}
