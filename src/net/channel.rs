use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use anyhow::{anyhow, bail, Context, Result};
use mac_oui::Oui;
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use crate::cmd::discover::Host;
use crate::net::packets;
use crate::print;

pub fn discover_hosts_on_eth_channel(
    start: Ipv4Addr,
    end: Ipv4Addr,
    intf: NetworkInterface,
    mut channel_cfg: Config,
    duration_in_ms: Duration,
) -> Result<Vec<Host>> {
    if channel_cfg.read_timeout.is_none() {
        channel_cfg.read_timeout = Some(Duration::from_millis(50));
    }
    let oui_db = Oui::default().map_err(|e| { anyhow!("loading OUI database: {}", e) })?;
    let (mut tx, mut rx) = open_ethernet_channel(&intf, &channel_cfg)?;
    if u32::from(start) > u32::from(end) { bail!("end IP ({end}) must be >= start IP ({start})"); }
    print::print_status("Connection established. Beginning ARP sweep...");
    packets::arp::send_sweep(start, end, &intf, &mut tx);
    let mut hosts: Vec<Host> = Vec::new();
    let deadline = Instant::now() + duration_in_ms;
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Some(host) = packets::handle_frame(&frame, &oui_db).ok() {
                    hosts.extend(host);
                }
            },
            Err(_) => { }
        }
    }
    Ok(hosts)
}

fn open_ethernet_channel(intf: &NetworkInterface, cfg: &Config)
                             -> Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)> {
    let ch = datalink::channel(intf, *cfg)
        .with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => Ok((tx, rx)),
        _ => anyhow::bail!("non-ethernet channel for {}", intf.name),
    }
}