use anyhow;
use pnet::datalink::{Config, NetworkInterface};
use pnet::util::MacAddr;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use anyhow::Context;
use is_root::is_root;
use crate::host::Host;
use crate::cmd::Target;
use crate::net::*;
use crate::net::channel::discover_hosts_on_eth_channel;
use crate::net::interface;
use crate::net::tcp::handshake_discovery;
use crate::{host, print};

pub async fn discover(target: Target) -> anyhow::Result<()> {
    let hosts: Option<Vec<Host>> = match target {
        Target::LAN => {
            print::print_status("Initializing LAN discovery...");
            let intf = interface::select(Target::LAN);
            let (start, end) = range::ip_range(Target::LAN, &intf)?;
            Some(discover_lan(start, end, intf).await?)
        },
        _ => { None }
    };
    print::separator("Network Discovery");
    if let Some(hosts) = hosts {
        let hosts = host::merge_by_mac_addr(hosts);
        for (idx, h) in hosts.into_iter().enumerate() {
            h.print_lan(idx as u32);
        }
    }
    Ok(())
}

async fn discover_lan(start_addr: Ipv4Addr, end_addr: Ipv4Addr, intf: NetworkInterface)
                      -> anyhow::Result<Vec<Host>> {
    let mut hosts: Vec<Host> = Vec::new();
    if !is_root() {
        let addresses = handshake_discovery(start_addr, end_addr).await?;
        for address in addresses {
            let mac_addr: Option<MacAddr> = None;
            let host = Host::new(IpAddr::V4(address), mac_addr);
            hosts.push(host);
        }
        return Ok(hosts)
    }
    let mut channel_cfg: Config = Config::default();
    channel_cfg.read_timeout = Some(Duration::from_millis(100));
    print::print_status("Establishing Ethernet connection...");
    hosts = discover_hosts_on_eth_channel(
        start_addr,
        end_addr,
        intf,
        channel_cfg,
        Duration::from_millis(500),
    ).context("discovering via ethernet channel")?;
    Ok(hosts)
}