use std::net::{IpAddr, Ipv4Addr};

use anyhow::Context;
use pnet::{
    packet::{
        Packet, 
        dns::DnsPacket, 
        ip::IpNextHeaderProtocols, udp::UdpPacket
    }, 
    transport::{
        self, 
        TransportChannelType, 
        TransportProtocol, 
        TransportReceiver,
        TransportSender
    }
};
use rand::random_range;

use crate::{host::Host, net::packets::{dns, udp}};

const TRANSPORT_BUFFER_SIZE: usize = 4096;
const CHANNEL_TYPE_UDP: TransportChannelType = TransportChannelType::Layer4(
    TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp)
);

struct TransportRunner {
    tx: TransportSender,
    rx: TransportReceiver
}

impl TransportRunner {
    fn new_layer4_udp() -> anyhow::Result<Self> {
        let (tx, rx) = transport::transport_channel(TRANSPORT_BUFFER_SIZE, CHANNEL_TYPE_UDP)?;
        Ok(Self { tx, rx })
    }
    fn send_packets<P>(&mut self, packet: P, destination: IpAddr) -> anyhow::Result<()>
    where P: Packet {
        self.tx.send_to(packet, destination)?;
        Ok(())
    }
}

pub fn try_dns_reverse_lookup(hosts: &mut [Box<dyn Host>]) -> anyhow::Result<()> {
    let mut runner: TransportRunner = TransportRunner::new_layer4_udp()?;
    let destination: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)); // Proper implementation will come later
    let ips: Vec<IpAddr> = hosts.iter().filter_map(|host| host.get_primary_ip()).collect();
    for ip in ips {
        let payload: Vec<u8> = dns::create_ptr_packet(ip)?;
        let src_port: u16 = random_range(50_000..u16::max_value());
        let dst_port: u16 = 53;
        let udp_packet: Vec<u8> = udp::create_packet(src_port, dst_port, payload)?;
        let udp_packet: UdpPacket = UdpPacket::new(&udp_packet).context("creating udp packet")?;
        runner.send_packets(udp_packet, destination)?;
    }
    Ok(())
}