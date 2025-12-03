use std::{net::IpAddr, time::{Duration, Instant}};

use anyhow::Context;
use pnet::{packet::{Packet, ip::IpNextHeaderProtocols, udp::UdpPacket}, 
    transport::{self, TransportChannelType, TransportProtocol, TransportReceiver,TransportSender, UdpTransportChannelIterator}
};
use rand::random_range;

use crate::net::{ip, packets::{dns, udp}};

const TRANSPORT_BUFFER_SIZE: usize = 4096;
const CHANNEL_TYPE_UDP: TransportChannelType = TransportChannelType::Layer4(
    TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp)
);

struct TransportRunner {
    tx: TransportSender,
    rx: TransportReceiver,
    timeout: Duration
}

impl TransportRunner {
    fn new_layer4_udp() -> anyhow::Result<Self> {
        let (tx, rx) = transport::transport_channel(TRANSPORT_BUFFER_SIZE, CHANNEL_TYPE_UDP)?;
        let timeout: Duration = Duration::from_millis(1000);
        Ok(Self { tx, rx, timeout })
    }

    fn send_packet<P>(&mut self, packet: P, dst_addr: IpAddr) -> anyhow::Result<()>
    where P: Packet {
        self.tx.send_to(packet, dst_addr)?;
        Ok(())
    }

    fn listen_for_dns_responses(&mut self, port: u16, id: u16) -> anyhow::Result<Option<String>> {
        let deadline: Instant = Instant::now() + self.timeout;
        let mut udp_iterator: UdpTransportChannelIterator = transport::udp_packet_iter(&mut self.rx);
        while Instant::now() < deadline {
            let (udp_packet, _) = match udp_iterator.next() {
                Ok(pkt) => pkt,
                Err(_) => continue,
            };

            if udp_packet.get_source() != 53 { continue; }

            if let Ok(Some((response_id, name))) = dns::get_hostname(udp_packet.payload()) {
                if response_id == id && udp_packet.get_destination() == port {
                    return Ok(Some(name))
                }
            }
        }
        Ok(None)
    }
}

pub fn get_hostname_via_rdns(ip_addr: IpAddr) -> anyhow::Result<String> {
    let mut runner: TransportRunner = TransportRunner::new_layer4_udp()?;
    let id: u16 = ip::derive_u16_id(&ip_addr);
    let ptr_packet: Vec<u8> = dns::create_ptr_packet(&ip_addr)?;
    let src_port: u16 = random_range(50_000..u16::max_value());
    let (dst_addr, dst_port) = dns::get_dns_server_socket_addr(&ip_addr)?;
    let udp_packet: Vec<u8> = udp::create_packet(src_port, dst_port, ptr_packet)?;
    let udp_packet: UdpPacket = UdpPacket::new(&udp_packet).context("creating udp packet")?;

    runner.send_packet(udp_packet, dst_addr)?;
    if let Some(hostname) = runner.listen_for_dns_responses(src_port, id)? {
        return Ok(hostname)
    }
    Err(anyhow::anyhow!("No hostname found"))
}