use std::{collections::HashSet, net::{IpAddr, Ipv4Addr, Ipv6Addr}, sync::{atomic::Ordering}, time::{Duration, Instant}};

use anyhow::ensure;
use async_trait::async_trait;
use pnet::{datalink::NetworkInterface, packet::tcp::TcpPacket};
use tokio::sync::mpsc::UnboundedSender;
use mappr_common::error;

use mappr_common::{network::{host::Host, range::IpCollection}};
use mappr_protocols as protocol;

use crate::network::transport::{self, TransportHandle, TransportType};

use super::NetworkExplorer;

// this shit needs improvement
const MIN_SCAN_DURATION: Duration = Duration::from_millis(200);
const MAX_SCAN_DURATION: Duration = Duration::from_millis(3000);
const MS_PER_IP: f64 = 0.5;

pub struct RoutedScanner {
    src_v4: Option<Ipv4Addr>,
    src_v6: Option<Ipv6Addr>,
    responded_ips: HashSet<IpAddr>,
    ips: IpCollection,
    tcp_handle: TransportHandle,
    dns_tx: Option<UnboundedSender<IpAddr>>
}

#[async_trait]
impl NetworkExplorer for RoutedScanner {
    async fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>> {
        if let Err(e) = self.send_discovery_packets() {
            error!("Failed to send packets: {e}");
        }

        let deadline: Instant = calculate_deadline(self.ips.len());

        while self.should_continue(deadline) {
            match self.tcp_handle.rx.try_recv() {
                Ok((_bytes, ip)) => {
                    if !self.ips.contains(&ip) {
                        continue;
                    }
                    if self.responded_ips.insert(ip) {
                        let _ = self.dns_tx.as_ref().map(|tx| tx.send(ip));
                        super::increment_host_count();
                    }
                }
                Err(_) => { }
            }
        }

        let hosts: Vec<Host> = self.responded_ips
            .drain()
            .map(|ip| Host::new(ip))
            .collect();

        Ok(hosts)
    }
}

impl RoutedScanner {
    pub fn new(intf: NetworkInterface, ips: IpCollection, dns_tx: Option<UnboundedSender<IpAddr>>) -> anyhow::Result<Self> {
        let tcp_handle: TransportHandle = transport::start_packet_capture(TransportType::TcpLayer4)?;

        let src_v4: Option<Ipv4Addr> = intf.ips.iter().find_map(|ip_net| {
            match ip_net.ip() {
                IpAddr::V4(ipv4) => Some(ipv4),
                _ => None,
            }
        });

        let src_v6: Option<Ipv6Addr> = intf.ips.iter().find_map(|ip_net| {
            match ip_net.ip() {
                IpAddr::V6(ipv6) => Some(ipv6),
                _ => None,
            }
        });

        ensure!(src_v4.is_some() || src_v6.is_some(), "interface has no ip addresses");

        let responded_ips: HashSet<IpAddr> = HashSet::new();

        Ok(Self { 
            src_v4, 
            src_v6, 
            responded_ips, 
            ips, 
            tcp_handle,
            dns_tx
         })
    }

    fn send_discovery_packets(&mut self) -> anyhow::Result<()> {
        let src_port: u16 = rand::random_range(50_000..u16::max_value());
        for dst_ip in self.ips.iter() {
            let src_ip: IpAddr = match dst_ip {
                IpAddr::V4(_) => {
                    ensure!(self.src_v4.is_some(), "interface has no ipv4 address");
                    IpAddr::V4(self.src_v4.unwrap())
                },
                IpAddr::V6(_) => {
                    ensure!(self.src_v6.is_some(), "interface has no ipv6 address");
                    IpAddr::V6(self.src_v6.unwrap())
                }
            };
            
            let packet: Vec<u8> = protocol::tcp::create_packet(&src_ip, &dst_ip, src_port, 443)?;
            if let Some(packet) = TcpPacket::new(&packet) {
                let _ = self.tcp_handle.tx.send_to(packet, dst_ip);
            }
        }
        Ok(())
    }

    fn should_continue(&self, deadline: Instant) -> bool {
        let has_time: bool = Instant::now() < deadline;
        let not_stopped: bool = !super::STOP_SIGNAL.load(Ordering::Relaxed);
        let work_remains: bool = self.ips.len() > self.responded_ips.len();

        has_time && not_stopped && work_remains
    }
}

fn calculate_deadline(ips_len: usize) -> Instant {
    let variable_ms = (ips_len as f64 * MS_PER_IP) as u64;

    let scan_duration = (MIN_SCAN_DURATION + Duration::from_millis(variable_ms))
        .clamp(MIN_SCAN_DURATION, MAX_SCAN_DURATION);

    Instant::now() + scan_duration
}