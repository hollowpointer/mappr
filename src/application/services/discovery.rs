use std::{collections::HashSet, net::IpAddr};

use anyhow::Context;
use is_root::is_root;
use pnet::datalink::NetworkInterface;

use crate::domain::models::host::{self, Host};
use crate::domain::models::target::Target;
use crate::ports::outbound::vendor_repository::VendorRepository;

use crate::adapters::outbound::network::{
    datalink::interface::{self, NetworkInterfaceExtension},
    ip,
    scanner,
    sender::SenderConfig,
    tcp_connect,
};

pub struct DiscoveryService {
    vendor_repo: Box<dyn VendorRepository>,
}

impl DiscoveryService {
    pub fn new(vendor_repo: Box<dyn VendorRepository>) -> Self {
        Self { vendor_repo }
    }

    pub async fn perform_discovery(&self, target: Target) -> anyhow::Result<Vec<Box<dyn Host>>> {
        let (targets, lan_interface) = get_targets_and_lan_intf(target)?;

        let mut hosts: Vec<Box<dyn Host>> = if !is_root() {
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

            host::internal_to_box(discovered_hosts)
        } else {
            // Root but no LAN -> Fallback to TCP
            host::external_to_box(
                tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?,
            )
        };

        // Enrich with Vendor Data
        self.enrich_vendors(&mut hosts);

        Ok(hosts)
    }

    fn enrich_vendors(&self, hosts: &mut Vec<Box<dyn Host>>) {
        for host in hosts.iter_mut() {
            if let Some(mac) = host.mac_addr() {
                if let Some(vendor) = self.vendor_repo.get_vendor(mac) {
                    host.set_vendor(vendor);
                }
            }
        }
    }
}

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
