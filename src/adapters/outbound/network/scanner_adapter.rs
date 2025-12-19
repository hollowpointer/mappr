use std::collections::HashSet;
use std::net::IpAddr;

use anyhow::Context;
use is_root::is_root;
use pnet::datalink::NetworkInterface;

use crate::domain::models::host::{self, Host};
use crate::domain::models::target::Target;
use crate::ports::outbound::network_scanner::NetworkScanner;

// Internal dependencies only!
use crate::engine::{
    datalink::interface::{self, NetworkInterfaceExtension},
    ip,
    scanner,
    sender::SenderConfig,
    tcp_connect,
};
use crate::domain::models::host::InternalHost;

pub struct NetworkScannerAdapter;

#[async_trait::async_trait]
impl NetworkScanner for NetworkScannerAdapter {
    async fn scan(&self, target: Target) -> anyhow::Result<Vec<Box<dyn Host>>> {
        let (targets, lan_interface) = get_targets_and_lan_intf(target)?;

        let hosts: Vec<Box<dyn Host>> = if !is_root() {
            // Non-root fallback
            host::external_to_box(
                tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?,
            )
        } else if let Some(intf) = lan_interface {
            // Root & LAN available -> Advanced Scan
            let mut sender_cfg = SenderConfig::from(&intf);
            sender_cfg.add_targets(targets);

            let discovered_hosts =
                tokio::task::spawn_blocking(move || scanner::discover_lan(intf, sender_cfg))
                    .await??;
            
            // MAP ENGINE HOST TO INTERNAL HOST (Anti-Corruption Layer)
            let internal_hosts: Vec<InternalHost> = discovered_hosts.into_iter().map(|eh| {
                InternalHost {
                    hostname: eh.hostname.unwrap_or_else(|| "No hostname".to_string()),
                    ips: eh.ips.into_iter().collect(), // HashSet -> BTreeSet
                    mac_addr: eh.mac,
                    _ports: std::collections::BTreeSet::new(),
                    vendor: None,
                    network_roles: std::collections::HashSet::new(),
                }
            }).collect();

            host::internal_to_box(internal_hosts)
        } else {
            // Root but no LAN -> Fallback to TCP
            host::external_to_box(
                tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?,
            )
        };
        
        Ok(hosts)
    }
}

// Logic moved from Application to Adapter (Infrastructure concern)
fn get_targets_and_lan_intf(
    target: Target,
) -> anyhow::Result<(HashSet<IpAddr>, Option<NetworkInterface>)> {
    match target {
        Target::LAN => {
            let intf =
                interface::get_lan().context("Failed to detect LAN interface for discovery")?;
            let range = intf
                .get_ipv4_range()
                .context("LAN interface has no valid IPv4 range")?;
            Ok((range.to_iter().collect::<HashSet<_>>(), Some(intf)))
        }
        Target::Host { target_addr } => {
            let intf = if ip::is_private(&target_addr) {
                interface::get_lan().ok()
            } else {
                None
            };
            Ok((HashSet::from([target_addr]), intf))
        }
        Target::Range { ipv4_range } => {
            let targets: HashSet<IpAddr> = ipv4_range.to_iter().collect();
            let start = IpAddr::V4(ipv4_range.start_addr);
            let end = IpAddr::V4(ipv4_range.end_addr);
            let intf = if ip::is_private(&start) && ip::is_private(&end) {
                interface::get_lan().ok()
            } else {
                None
            };
            Ok((targets, intf))
        }
        Target::VPN => anyhow::bail!("Target::VPN is currently unimplemented!"),
    }
}
