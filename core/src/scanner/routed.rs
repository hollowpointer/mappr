use async_trait::async_trait;
use mappr_common::network::{host::Host, range::IpCollection};
use tracing::info;

use crate::network::transport::{self, TransportHandle, TransportType};

use super::NetworkExplorer;

pub struct RoutedScanner {
    _ips: IpCollection,
    _tcp_handle: TransportHandle,
}

#[async_trait]
impl NetworkExplorer for RoutedScanner {
    fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>> {
        self.send_discovery_packets();
        info!("Scanning externally...");
        Ok(vec![])
    }
}

impl RoutedScanner {
    pub fn new(_ips: IpCollection) -> anyhow::Result<Self> {
        let _tcp_handle: TransportHandle = transport::start_packet_capture(TransportType::TcpLayer4)?;

        Ok(Self { _ips, _tcp_handle })
    }

    fn send_discovery_packets(&mut self) {

    }
}