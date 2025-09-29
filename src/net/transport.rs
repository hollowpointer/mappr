use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use pnet::transport;
use pnet::transport::{tcp_packet_iter, TransportChannelType, TransportReceiver};
use crate::host::Host;
use crate::net::packets::tcp;
use crate::net::range::Ipv4Range;

pub fn discover_on_transport_channel(ipv4range: Arc<Ipv4Range>,
                                     buffer_size: usize,
                                     src_addr: Ipv4Addr,
                                     channel_type: TransportChannelType,
) -> anyhow::Result<Vec<Host>> {
    let (ts, tr) = transport::transport_channel(buffer_size, channel_type)?;
    tcp::send_syn_packet(ts, src_addr, 7777, &ipv4range, 443)?;
    Ok(listen_for_hosts(tr, src_addr, &ipv4range, Duration::from_millis(1000)))
}


fn listen_for_hosts(
    mut tr: TransportReceiver,
    src_addr: Ipv4Addr,
    ipv4range: &Ipv4Range,
    duration_in_ms: Duration,
) -> Vec<Host> {
    let mut found_ips: HashSet<IpAddr> = HashSet::new();
    let mut tcp_packets = tcp_packet_iter(&mut tr);
    let deadline = Instant::now() + duration_in_ms;

    while deadline > Instant::now() {
        if let Ok((_, ip_addr)) = tcp_packets.next() {
            if let IpAddr::V4(ip_v4) = ip_addr {
                if ipv4range.contains(&ip_v4) && src_addr != ip_v4 {
                    found_ips.insert(ip_addr);
                }
            }
            else if let IpAddr::V6(_) = ip_addr {
                found_ips.insert(ip_addr);
            }
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    found_ips.into_iter().map(Host::from).collect()
}