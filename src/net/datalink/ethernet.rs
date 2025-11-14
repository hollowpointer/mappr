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