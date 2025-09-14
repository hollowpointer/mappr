use tokio::net::TcpStream;

use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::thread;
use std::time::Duration;
use anyhow::Context;
use pnet::packet::tcp::{ipv4_checksum, MutableTcpPacket, TcpFlags};
use pnet::transport::TransportSender;
use tokio::time::timeout;
use crate::host::Host;
use crate::net::range::{ip_iter, Ipv4Range};
use crate::net::utils::{MIN_TCP_HEADER_SIZE};
use crate::print;

pub fn send_syn_packet(mut ts: TransportSender,
                       src_addr: Ipv4Addr,
                       src_port: u16,
                       ipv4range: &Ipv4Range,
                       dst_port: u16
) -> anyhow::Result<()> {
    let mut buffer = [0u8; MIN_TCP_HEADER_SIZE];
    let mut tcp = MutableTcpPacket::new(
        &mut buffer[..]
    ).context("creating tcp packet")?;
    tcp.set_source(src_port);
    tcp.set_destination(dst_port);
    tcp.set_sequence(1);
    tcp.set_data_offset(5);
    tcp.set_flags(TcpFlags::SYN);

    let len = ip_iter(&ipv4range).count() as u64;
    let progress_bar = print::create_progressbar(len, "SYN".to_string());
    for dst_addr in ip_iter(&ipv4range) {
        tcp.set_checksum(ipv4_checksum(&tcp.to_immutable(), &src_addr, &dst_addr));
        match ts.send_to(&tcp, IpAddr::from(dst_addr)) { Ok(_) | Err(_) => {}, }
        progress_bar.inc(1);
        thread::sleep(Duration::from_millis(5));
    };
    Ok(())
}

pub async fn handshake_range_discovery(ipv4range: Ipv4Range) -> anyhow::Result<Vec<Host>> {
    let mut result: Vec<Host> = Vec::new();
    for ip in ip_iter(&ipv4range) {
        if let Some(found) = handshake_probe(ip).await? { result.push(found); }
    }
    Ok(result)
}

async fn handshake_probe(addr: Ipv4Addr) -> anyhow::Result<Option<Host>> {
    let sa = SocketAddrV4::new(addr, 443);
    let mut host: Host = Host::default();
    match timeout(Duration::from_millis(100), TcpStream::connect(sa)).await {
        Ok(Ok(_)) | Ok(Err(_)) => {
            host.set_ipv4(*sa.ip());
            Ok(Some(host))
        },
        Err(_elapsed) => Ok(None),
    }
}