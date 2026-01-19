use std::{
    collections::HashMap, 
    net::{IpAddr, Ipv4Addr}, 
    sync::{atomic::{AtomicU16, Ordering}, mpsc::{Receiver, TryRecvError}}, time::Duration
};
use anyhow::{Context, ensure};
use mappr_common::{network::host::Host, utils};
use mappr_protocols::{dns, udp};
use pnet::packet::{udp::UdpPacket, Packet};

use crate::network::transport::{self, TransportHandle, TransportType};

const DNS_PORT: u16 = 53;
const MDNS_PORT: u16 = 5353;

pub struct HostnameResolver {
    udp_handle: TransportHandle,
    hostname_map: HashMap<IpAddr, String>,
    dns_map: HashMap<u16, IpAddr>,
    dns_rx: Receiver<IpAddr>,
    id_counter: AtomicU16,
}

impl HostnameResolver {
    pub fn new(dns_rx: Receiver<IpAddr>) -> anyhow::Result<Self> {
        Ok(Self {
            udp_handle: transport::start_packet_capture(TransportType::UdpLayer4)?,
            hostname_map: HashMap::new(),
            dns_map: HashMap::new(),
            dns_rx,
            id_counter: AtomicU16::new(0)
        })
    }

pub fn spawn(mut self) -> std::thread::JoinHandle<Self> {
    std::thread::spawn(move || {
        let mut channel_open = true;
        loop {
            if channel_open {
                while let Ok(ip) = self.dns_rx.try_recv() {
                    if !self.hostname_map.contains_key(&ip) {
                        let _ = self.send_dns_query(&ip);
                    }
                }

                if let Err(TryRecvError::Disconnected) = self.dns_rx.try_recv() {
                    channel_open = false;
                }
            }

            match self.udp_handle.rx.recv_timeout(Duration::from_millis(100)) {
                Ok((bytes, _addr)) => {
                    let _ = self.process_udp_packets(&bytes);
                }
                Err(_) => {
                    if !channel_open {
                        break;
                    }
                }
            }
        }
        self
    })
}

    fn send_dns_query(&mut self, ip: &IpAddr) -> anyhow::Result<()> {
        ensure!(is_queryable(ip), "{ip} cannot be queried");
        let id: u16 = self.get_next_trans_id();
        self.dns_map.insert(id, *ip);
        let (dns_addr, dns_port) = get_dns_server_socket(&ip);

        let bytes: Vec<u8> = match utils::ip::is_private(&dns_addr) {
            true => dns::create_ptr_packet(ip, id)?,
            false => vec![],
        };

        let src_port: u16 = rand::random_range(50_000..u16::max_value());
        let udp_bytes: Vec<u8> = udp::create_packet(src_port, dns_port, bytes)?;
        let udp_pkt = UdpPacket::new(&udp_bytes).context("creating udp packet")?;
        
        self.udp_handle.tx.send_to(udp_pkt, dns_addr)?;
        Ok(())
    }

    fn process_udp_packets(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let udp_packet = UdpPacket::new(&bytes).context("truncated or invalid UDP packet")?;
        match udp_packet.get_source() {
            DNS_PORT => self.process_dns_packet(udp_packet)?,
            MDNS_PORT => { /* Implement mDNS next */ }
            _ => { },
        }
        Ok(())
    }

    fn process_dns_packet(&mut self, packet: UdpPacket) -> anyhow::Result<()> {
        let (response_id, hostname) = dns::get_hostname(packet.payload())?;
        if let Some(ip) = self.dns_map.remove(&response_id) {
            self.hostname_map.insert(ip, hostname);
        }
        Ok(())
    }

    pub fn resolve_hosts(&mut self, hosts: &mut Vec<Host>) {
        for host in hosts {
            if let Some(hostname) = self.hostname_map.remove(&host.ip) {
                host.hostname = Some(hostname);
            }
        }
    }

    fn get_next_trans_id(&self) -> u16 {
        self.id_counter.fetch_add(1, Ordering::Relaxed)
    }
}

fn is_queryable(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V6(ipv6_addr) => {
            utils::ip::is_global_unicast(ipv6_addr)
        },
        IpAddr::V4(_ipv4_addr) => {
            // Future refinement: check for private ranges/localhost here
            true 
        }
    }
}

// This is fragile and needs rework
fn get_dns_server_socket(ip: &IpAddr) -> (IpAddr, u16) {
    let port: u16 = 53;
    let ip_addr: IpAddr = {
        if utils::ip::is_private(&ip) {
            IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))
        } else {
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))
        }
    };
    (ip_addr, port)
}