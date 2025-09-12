use anyhow;
use pnet::datalink::{Config, NetworkInterface};
use std::time::Duration;
use anyhow::{bail, Context};
use is_root::is_root;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::{TransportChannelType, TransportProtocol};
use crate::host::Host;
use crate::cmd::Target;
use crate::net::datalink::channel::{discover_on_eth_channel, ProbeType};
use crate::net::datalink::interface;
use crate::net::range;
use crate::net::packets::tcp::handshake_range_discovery;
use crate::{host, print};
use crate::net::range::Ipv4Range;
use crate::net::transport::discover_on_transport_channel;

pub async fn discover(target: Target) -> anyhow::Result<()> {
    let hosts: Vec<Host> = match target {
        Target::LAN => {
            print::print_status("Initializing LAN discovery...");
            let intf: NetworkInterface = interface::select(Target::LAN);
            let ipv4range: Ipv4Range = Ipv4Range::from_tuple(range::interface_range_v4(&intf)?);
            let hosts = discover_lan(ipv4range, intf.clone(), ProbeType::Default).await?;
            hosts.into_iter().filter(|h| { h.mac_addr != intf.mac } ).collect::<Vec<Host>>()
        },
        _ => { bail!("this target is currently unimplemented!") }
    };
    print::separator("Network Discovery");
    host::print(hosts, target)?;
    Ok(())
}

const READ_TIMEOUT_MS: u64 = 50;
const PROBE_TIMEOUT_MS: u64 = 500;

async fn discover_lan(ipv4range: Ipv4Range, intf: NetworkInterface, probe_type: ProbeType)
    -> anyhow::Result<Vec<Host>> {
    if !is_root() { return handshake_range_discovery(ipv4range).await.context("handshake discovery (non-root)"); }
    let channel_cfg = Config { read_timeout: Some(Duration::from_millis(READ_TIMEOUT_MS)), ..Default::default() };
    print::print_status("Establishing Ethernet connection...");
    let hosts: Vec<Host> = [
        discover_on_eth_channel(
            ipv4range.clone(),
            intf,
            channel_cfg,
            probe_type,
            Duration::from_millis(PROBE_TIMEOUT_MS),
        ).context("discovering via ethernet channel")?,
        discover_on_transport_channel(
            512,
            TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Tcp)),
            ipv4range,
        )?,
    ].into_iter().flatten().collect();
    Ok(hosts)
}