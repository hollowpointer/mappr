use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};
use anyhow;
use anyhow::{bail, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::util::MacAddr;
use crate::net::datalink::arp;
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::packets;
use crate::host::Host;
use crate::net::range::Ipv4Range;
use crate::print;

const PROBE_TIMEOUT_MS: u64 = 2000;

pub fn discover_via_eth(
    interface: NetworkInterface,
    src_addr_v4: Option<Ipv4Addr>,
    ipv4_range: Option<Ipv4Range>,
    link_local_addr: Option<Ipv6Addr>,
) -> anyhow::Result<Vec<Host>> {
    let (mut tx, rx) = open_eth_channel(&interface, &get_config())?;
    let duration_in_ms: Duration = Duration::from_millis(PROBE_TIMEOUT_MS);
    let src_mac: MacAddr = interface.mac.unwrap(); // This is safe, a MAC addr is a requirement
    let packets: Vec<Vec<u8>> = packets::create_packets(src_mac, src_addr_v4, ipv4_range, link_local_addr)?;
    for packet in packets { tx.send_to(&packet, None); }
    Ok(listen_for_hosts(rx, duration_in_ms))
}

pub fn discover_via_ip_addr(
    intf: NetworkInterface,
    dst_addr: IpAddr,
) -> anyhow::Result<Option<Host>> {
    let (mut tx, rx) = open_eth_channel(&intf, &get_config())?;
    let duration_in_ms: Duration = Duration::from_millis(PROBE_TIMEOUT_MS);
    let src_mac: MacAddr = intf.mac.unwrap();
    let packet: Vec<u8> = match dst_addr {
        IpAddr::V4(dst_addr) => {
            if let Some(ipv4_net) = intf.get_ipv4_net() {
                let src_addr: Ipv4Addr = ipv4_net.ip();
                let dst_mac: MacAddr = MacAddr::broadcast();
                arp::create_packet(src_mac, dst_mac, src_addr, dst_addr)?
            } else { anyhow::bail!("Cannot perform ipv4 host discovery: interface does not have a ipv4") }
        },
        IpAddr::V6(_) => {
            if let Some(_) = intf.get_link_local_addr() {
                anyhow::bail!("Ipv6 host discovery not possible as of now, please implement NDP")
            } else { anyhow::bail!("Cannot perform ipv6 host discovery: interface does not have a ipv6") }
        },
    };
    tx.send_to(&packet, None);
    let host: Option<Host> = listen_for_hosts(rx, duration_in_ms)
        .into_iter()
        .find(|host| host.ips().contains(&dst_addr));
    Ok(host)
}

fn open_eth_channel(intf: &NetworkInterface, cfg: &Config)
    -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)> {
    let ch = datalink::channel(intf, *cfg).with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => {
            print::print_status("Connection established. Beginning sweep...");
            Ok((tx, rx))
        },
        _ => bail!("non-ethernet channel for {}", intf.name),
    }
}

fn listen_for_hosts(mut rx: Box<dyn DataLinkReceiver>, duration_in_ms: Duration) -> Vec<Host> {
    let mut hosts: Vec<Host> = Vec::new();
    let deadline = Instant::now() + duration_in_ms;
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Some(host) = packets::handle_frame(&frame).ok()
                { hosts.extend(host); }
            },
            Err(_) => { }
        }
    }
    hosts
}

fn get_config() -> Config {
    Config {
        read_timeout: Some(Duration::from_millis(50)),
        ..Default::default()
    }
}