use std::{net::{IpAddr, Ipv4Addr}, time::{Duration, Instant}};

use anyhow::Context;
use pnet::{
    packet::{
        Packet, 
        ip::IpNextHeaderProtocols, 
        udp::UdpPacket
    }, 
    transport::{
        self, 
        TransportChannelType, 
        TransportProtocol, 
        TransportReceiver,
        TransportSender, UdpTransportChannelIterator
    }
};
use rand::random_range;

use crate::{host::Host, net::{ip, packets::{dns, udp}}};

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

    fn send_packets<P>(&mut self, packet: P, destination: IpAddr) -> anyhow::Result<()>
    where P: Packet {
        self.tx.send_to(packet, destination)?;
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

pub fn try_dns_reverse_lookup(host: &mut dyn Host) -> anyhow::Result<()> {
    let mut runner: TransportRunner = TransportRunner::new_layer4_udp()?;
    let destination: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)); 
    let ip: IpAddr = host.get_primary_ip().ok_or_else(|| anyhow::anyhow!("No Primary IP"))?;
    let id: u16 = ip::derive_u16_id(&ip);
    let payload: Vec<u8> = dns::create_ptr_packet(ip)?;
    let src_port: u16 = random_range(50_000..u16::max_value());
    let dst_port: u16 = 53;
    let udp_payload: Vec<u8> = udp::create_packet(src_port, dst_port, payload)?;
    let udp_packet: UdpPacket = UdpPacket::new(&udp_payload).context("creating udp packet")?;
    
    runner.send_packets(udp_packet, destination)?;

    if let Some(hostname) = runner.listen_for_dns_responses(src_port, id)? {
        host.set_hostname(hostname);
    }

    Ok(())
}