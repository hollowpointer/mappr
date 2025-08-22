use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use mac_oui::Oui;
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use crate::net::packets;

pub fn handle_channel(start: Ipv4Addr, end: Ipv4Addr, intf: NetworkInterface, channel_cfg: Config) -> Result<()> {
    let oui_db = Oui::default().expect("Failed to load OUI DB");

    let (mut tx, mut rx) = open_ethernet_channel(&intf, &channel_cfg)?;

    for ip in u32::from(start)..=u32::from(end) {
        packets::arp::send(&intf, Ipv4Addr::from(ip), &mut tx).expect("Failed to perform ARP sweep");
    }

    let deadline = Instant::now() + Duration::from_millis(3000);
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => { packets::handle_frame(&frame, &oui_db).ok(); },
            Err(_) => { }
        }
    }

    Ok(())
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