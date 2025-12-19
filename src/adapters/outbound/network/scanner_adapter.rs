use std::collections::HashSet;
use std::net::IpAddr;

use anyhow::Context;
use is_root::is_root;
use pnet::datalink::NetworkInterface;

use crate::domain::models::host::Host;
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

pub struct NetworkScannerAdapter;

#[async_trait::async_trait]
impl NetworkScanner for NetworkScannerAdapter {
    async fn scan(&self, target: Target) -> anyhow::Result<Vec<Host>> {
        let (targets, lan_interface) = get_targets_and_lan_intf(target)?;

        let hosts: Vec<Host> = if !is_root() {
            // Non-root fallback - only gets IPs
            let external_hosts: Vec<Host> = tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?;
            external_hosts
        } else if let Some(intf) = lan_interface {
            // Root & LAN available -> Advanced Scan
            let mut sender_cfg = SenderConfig::from(&intf);
            sender_cfg.add_targets(targets);

            let discovered_hosts =
                tokio::task::spawn_blocking(move || scanner::discover_lan(intf, sender_cfg))
                    .await??;
            
            // MAP ENGINE HOST TO HOST
            discovered_hosts.into_iter().map(|eh| {
                let primary_ip = eh.ips.iter()
                    .find(|ip| ip.is_ipv4())
                    .or_else(|| eh.ips.iter().next())
                    .cloned()
                    .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(0,0,0,0)));
                let mut host = Host::new(primary_ip);
                host.ips = eh.ips.into_iter().collect();
                host.mac = Some(eh.mac);
                if let Some(h) = eh.hostname {
                    host.hostname = Some(h);
                }
                host
            }).collect()
        } else {
            // Root but no LAN -> Fallback to TCP
              let external_hosts: Vec<Host> = tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?;
              external_hosts
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
