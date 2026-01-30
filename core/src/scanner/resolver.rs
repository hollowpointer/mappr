use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

use anyhow::{Context, ensure};
use zond_common::{info, network::host::Host, utils};
use zond_protocols::{
    dns,
    mdns::{self, MdnsMetadata},
    udp,
};
use pnet::packet::{Packet, udp::UdpPacket};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::network::transport::{self, TransportHandle, TransportType};

const DNS_PORT: u16 = 53;
const MDNS_PORT: u16 = 5353;

pub struct HostnameResolver {
    udp_handle: TransportHandle,
    hostname_map: HashMap<IpAddr, String>,
    dns_map: HashMap<u16, IpAddr>,
    dns_rx: UnboundedReceiver<IpAddr>,
    id_counter: AtomicU16,
}

impl HostnameResolver {
    pub fn new(dns_rx: UnboundedReceiver<IpAddr>) -> anyhow::Result<Self> {
        Ok(Self {
            udp_handle: transport::start_packet_capture(TransportType::UdpLayer4)?,
            hostname_map: HashMap::new(),
            dns_map: HashMap::new(),
            dns_rx,
            id_counter: AtomicU16::new(0),
        })
    }

    pub async fn run(mut self) -> Self {
        loop {
            tokio::select! {
                res = self.dns_rx.recv() => {
                    match res {
                        Some(ip) => {
                            let _ = self.send_dns_query(&ip).await;
                        }
                        None => break,
                    }
                }
                pkt = self.udp_handle.rx.recv() => {
                    if let Some((bytes, _addr)) = pkt {
                        let _ = self.process_udp_packets(&bytes);
                    }
                }
            }
        }

        if !self.dns_map.is_empty() {
            let _ = tokio::time::timeout(Duration::from_millis(250), async {
                while !self.dns_map.is_empty() {
                    if let Some((bytes, _addr)) = self.udp_handle.rx.recv().await {
                        let _ = self.process_udp_packets(&bytes);
                    }
                }
            })
            .await;
        }

        self
    }

    async fn send_dns_query(&mut self, ip: &IpAddr) -> anyhow::Result<()> {
        ensure!(is_queryable(ip), "{ip} cannot be queried");
        let id: u16 = self.get_next_trans_id();
        self.dns_map.insert(id, *ip);
        let (dns_addr, dns_port) = get_dns_server_socket(ip);

        let bytes: Vec<u8> = dns::create_ptr_packet(ip, id)?;
        let src_port: u16 = rand::random_range(50_000..u16::MAX);
        let udp_bytes: Vec<u8> = udp::create_packet(src_port, dns_port, bytes)?;
        let tx = self.udp_handle.tx.clone();
        tokio::task::spawn_blocking(move || {
            let udp_pkt = UdpPacket::new(&udp_bytes)
                .context("creating udp packet")
                .unwrap();
            let mut sender = tx.lock().unwrap();
            sender.send_to(udp_pkt, dns_addr)
        })
        .await??;
        Ok(())
    }

    fn process_udp_packets(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let udp_packet = UdpPacket::new(bytes).context("truncated or invalid UDP packet")?;
        match udp_packet.get_source() {
            DNS_PORT => self.process_dns_packet(udp_packet)?,
            MDNS_PORT => self.process_mdns_packet(udp_packet)?,
            _ => {}
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

    fn process_mdns_packet(&mut self, packet: UdpPacket) -> anyhow::Result<()> {
        let mdns_resource: MdnsMetadata = mdns::extract_resource(packet.payload())?;
        info!("{:?}", mdns_resource.hostname);
        info!("{:?}", mdns_resource.ips);
        Ok(())
    }

    pub fn resolve_hosts(&mut self, hosts: &mut Vec<Host>) {
        for host in hosts {
            for ip in &host.ips {
                if let Some(hostname) = self.hostname_map.remove(ip)
                    && host.hostname.is_none()
                {
                    host.hostname = Some(hostname);
                }
            }
        }
    }

    fn get_next_trans_id(&self) -> u16 {
        self.id_counter.fetch_add(1, Ordering::Relaxed)
    }
}

fn is_queryable(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V6(ipv6_addr) => utils::ip::is_global_unicast(ipv6_addr),
        IpAddr::V4(_ipv4_addr) => {
            // Future refinement: check for private ranges/localhost here
            true
        }
    }
}

// This is fragile and needs rework
fn get_dns_server_socket(ip: &IpAddr) -> (IpAddr, u16) {
    let ip_addr: IpAddr = {
        if utils::ip::is_private(ip) {
            IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))
        } else {
            IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))
        }
    };
    (ip_addr, DNS_PORT)
}
