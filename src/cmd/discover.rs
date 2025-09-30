use std::sync::Arc;
use anyhow;
use pnet::datalink::{Config, NetworkInterface};
use std::time::Duration;
use anyhow::{bail, Context};
use is_root::is_root;
use crate::host::Host;
use crate::cmd::Target;
use crate::net::datalink::channel::ProbeType;
use crate::net::datalink::{channel, interface};
use crate::net::range;
use crate::net::packets::tcp::handshake_range_discovery;
use crate::{host, print, SPINNER};
use crate::net::range::Ipv4Range;

pub async fn discover(target: Target) -> anyhow::Result<()> {
    let hosts: Vec<Host> = match target {
        Target::LAN => {
            SPINNER.set_message("Performing LAN discovery...");
            print::print_status("Initializing LAN discovery...");
            let intf: NetworkInterface = interface::select(Target::LAN);
            let ipv4range: Ipv4Range = Ipv4Range::from_tuple(range::interface_range_v4(&intf)?);
            let hosts = discover_lan(Arc::new(ipv4range), Arc::new(intf.clone()), ProbeType::Default).await?;
            SPINNER.finish_and_clear();
            hosts.into_iter().filter(|h| { h.get_mac_addr() != intf.mac } ).collect::<Vec<Host>>()
        },
        _ => { bail!("this target is currently unimplemented!") }
    };
    print::separator("Network Discovery");
    host::print(hosts, target)?;
    Ok(())
}

const READ_TIMEOUT_MS: u64 = 50;
const PROBE_TIMEOUT_MS: u64 = 500;

async fn discover_lan(ipv4range: Arc<Ipv4Range>, intf: Arc<NetworkInterface>, probe_type: ProbeType)
    -> anyhow::Result<Vec<Host>> {
    if !is_root() { return handshake_range_discovery(ipv4range).await.context("handshake discovery (non-root)"); }
    let eth_range: Arc<Ipv4Range> = ipv4range.clone();
    let eth_intf: Arc<NetworkInterface> = intf.clone();
    let channel_cfg = Config { read_timeout: Some(Duration::from_millis(READ_TIMEOUT_MS)), ..Default::default() };
    print::print_status("Establishing Ethernet connection...");
    channel::discover_on_eth_channel(
        eth_range,
        eth_intf,
        channel_cfg,
        probe_type,
        Duration::from_millis(PROBE_TIMEOUT_MS),
    ).context("discovering via ethernet channel")
}