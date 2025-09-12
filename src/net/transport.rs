use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};
use pnet::transport;
use pnet::transport::{tcp_packet_iter, TransportChannelType, TransportReceiver};
use crate::host::Host;
use crate::net::packets::tcp;
use crate::net::range::Ipv4Range;

pub fn discover_on_transport_channel(buffer_size: usize,
                                     channel_type: TransportChannelType,
                                     ipv4range: Ipv4Range
) -> anyhow::Result<Vec<Host>> {
    let (ts, tr) = transport::transport_channel(buffer_size, channel_type)?;
    let src_addr: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 32);
    tcp::send_syn_packet(ts, src_addr, 7777, ipv4range, 443)?;
    Ok(listen_for_hosts(tr, Duration::from_millis(1000)))
}


fn listen_for_hosts(mut tr: TransportReceiver, duration_in_ms: Duration) -> Vec<Host> {
    let mut hosts: Vec<Host> = Vec::new();
    let mut tcp_packets = tcp_packet_iter(&mut tr);
    let deadline = Instant::now() + duration_in_ms;
    while deadline > Instant::now() {
        match tcp_packets.next() {
            Ok((_, ip_addr)) => {
                let mut host = Host::default();
                if let IpAddr::V4(ip_v4) = ip_addr { host.set_ipv4(ip_v4) };
                if let IpAddr::V6(ip_v6) = ip_addr { host.add_ipv6(ip_v6) }
                hosts.push(host);
            },
            Err(_) => { }
        }
    }
    hosts
}