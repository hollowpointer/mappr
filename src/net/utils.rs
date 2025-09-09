use std::net::{IpAddr, Ipv4Addr};
use anyhow::bail;

pub const MIN_ETH_FRAME_NO_FCS: usize = 60;
pub const ARP_LEN: usize = 28;
pub const ETH_HDR_LEN: usize = 14;
pub const IP_V6_HDR_LEN: usize = 40;
pub const ICMP_V6_ECHO_REQ_LEN: usize = 8;

pub fn ip_addr_to_ipv4_addr(ip_addr: IpAddr) -> anyhow::Result<Ipv4Addr> {
    let result: Ipv4Addr;
    match ip_addr {
        IpAddr::V4(v4) => result = v4,
        _ => bail!("passed value is not a ipv4!")
    }
    Ok(result)
}
