use mappr_common::network::{host::Host, range::IpCollection};
use pnet::datalink::NetworkInterface;
use tracing::info;

use super::NetworkExplorer;

pub struct RoutedScanner {
    _intf: NetworkInterface,
    _ips: IpCollection
}

impl NetworkExplorer for RoutedScanner {
    fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>> {
        info!("Scanning externally...");
        Ok(vec![])
    }
}

impl RoutedScanner {
    pub fn new(_intf: NetworkInterface, _ips: IpCollection) -> Self {
        Self { _intf, _ips }
    }
}