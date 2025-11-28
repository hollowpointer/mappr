use std::{collections::HashMap, net::{IpAddr, Ipv4Addr}, time::{Duration, Instant}};

use anyhow::Context;
use pnet::{
    packet::{
        Packet, 
        dns::DnsPacket, 
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

use crate::{host::{ExternalHost, Host}, net::{ip, packets::{dns, udp}}};

const TRANSPORT_BUFFER_SIZE: usize = 4096;
const CHANNEL_TYPE_UDP: TransportChannelType = TransportChannelType::Layer4(
    TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp)
);

struct TransportRunner {
    tx: TransportSender,
    rx: TransportReceiver,
    duration_in_ms: Duration
}

impl TransportRunner {
    fn new_layer4_udp() -> anyhow::Result<Self> {
        let (tx, rx) = transport::transport_channel(TRANSPORT_BUFFER_SIZE, CHANNEL_TYPE_UDP)?;
        let duration_in_ms: Duration = Duration::from_millis(1000);
        Ok(Self { tx, rx, duration_in_ms })
    }

    fn send_packets<P>(&mut self, packet: P, destination: IpAddr) -> anyhow::Result<()>
    where P: Packet {
        self.tx.send_to(packet, destination)?;
        Ok(())
    }

    fn listen_for_dns_responses(&mut self, id_map: &HashMap<u16, IpAddr>) -> anyhow::Result<HashMap<IpAddr, String>> {
        let mut results: HashMap<IpAddr, String> = HashMap::new();
        let deadline: Instant = Instant::now() + self.duration_in_ms;
        let mut udp_iterator: UdpTransportChannelIterator = transport::udp_packet_iter(&mut self.rx);

        while Instant::now() < deadline {
            let (udp_packet, _) = match udp_iterator.next() {
                Ok(pkt) => pkt,
                Err(_) => continue,
            };

            if udp_packet.get_source() != 53 { continue; }

            if let Ok(Some((response_id, name))) = dns::get_hostname(udp_packet.payload()) {
                if let Some(original_ip) = id_map.get(&response_id) {
                    results.insert(*original_ip, name);
                }
            }
        }
        Ok(results)
    }
}

pub fn try_dns_reverse_lookup(hosts: &mut [Box<dyn Host>]) -> anyhow::Result<()> {
    let mut runner = TransportRunner::new_layer4_udp()?;
    let destination = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)); 

    let mut id_map: HashMap<u16, IpAddr> = HashMap::new();

    for host in hosts.iter() {
        let ip = host.get_primary_ip().ok_or_else(|| anyhow::anyhow!("No Primary IP"))?;

        // Generate ID and store mapping
        let id = ip::derive_u16_id(&ip);
        id_map.insert(id, ip);

        // Construct Packet
        let payload: Vec<u8> = dns::create_ptr_packet(ip)?;
        let src_port: u16 = random_range(50_000..u16::max_value());
        let dst_port: u16 = 53;
        
        let udp_payload = udp::create_packet(src_port, dst_port, payload)?;
        let udp_packet = UdpPacket::new(&udp_payload).context("creating udp packet")?;
        
        runner.send_packets(udp_packet, destination)?;
    }

    let results: HashMap<IpAddr, String> = runner.listen_for_dns_responses(&id_map)?;
    for host in hosts.iter_mut() {
        if let Some(ip) = host.get_primary_ip() {
            if let Some(hostname) = results.get(&ip) {
                host.set_hostname(hostname.clone());
            }
        }
    }

    Ok(())
}