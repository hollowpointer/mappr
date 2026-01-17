use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::{bail, ensure};
use async_trait::async_trait;
use pnet::{datalink::NetworkInterface, packet::tcp::TcpPacket};
use tracing::info;

use mappr_common::network::{host::Host, range::IpCollection};
use mappr_protocols as protocol;

use crate::network::transport::{self, TransportHandle, TransportType};

use super::NetworkExplorer;

pub struct RoutedScanner {
    src_v4: Option<Ipv4Addr>,
    src_v6: Option<Ipv6Addr>,
    ips: IpCollection,
    tcp_handle: TransportHandle,
}

#[async_trait]
impl NetworkExplorer for RoutedScanner {
    fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>> {
        self.send_discovery_packets()?;
        info!("Scanning externally...");
        Ok(vec![])
    }
}

impl RoutedScanner {
    pub fn new(intf: NetworkInterface, ips: IpCollection) -> anyhow::Result<Self> {
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

        Ok(Self { src_v4, src_v6, ips, tcp_handle })
    }

    fn send_discovery_packets(&mut self) -> anyhow::Result<()> {
        let src_port: u16 = rand::random_range(50_000..u16::max_value());
        for dst_ip in self.ips.clone() {
            let src_ip: IpAddr = match dst_ip {
                IpAddr::V4(_) => {
                    if self.src_v4.is_none() {
                        bail!("interface has no ipv4 address")
                    }
                    IpAddr::V4(self.src_v4.unwrap())
                },
                IpAddr::V6(_) => {
                    if self.src_v6.is_none() {
                        bail!("interface has no ipv6 address")
                    }
                    IpAddr::V6(self.src_v6.unwrap())
                }
            };
            
            let packet: Vec<u8> = protocol::tcp::create_packet(&src_ip, &dst_ip, src_port, 80)?;
            if let Some(packet) = TcpPacket::new(&packet) {
                let _ = self.tcp_handle.tx.send_to(packet, dst_ip);
            }
        }
        Ok(())
    }
}