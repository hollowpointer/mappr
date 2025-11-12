use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};
use anyhow;
use anyhow::{bail, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::ipnetwork::Ipv4Network;
use pnet::util::MacAddr;
use crate::net::datalink::interface;
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::packets::{self, PacketType};
use crate::host::InternalHost;
use crate::print;

const PROBE_TIMEOUT_MS: u64 = 2000;

pub struct SenderContext {
    pub src_mac: MacAddr,
    pub ipv4_net: Option<Ipv4Network>,
    pub link_local: Option<Ipv6Addr>,
    pub dst_addr_v4: Option<Ipv4Addr>,
    pub dst_addr_v6: Option<Ipv6Addr>
}

impl From<&NetworkInterface> for SenderContext {
    fn from(interface: &NetworkInterface) -> Self {
        Self {
            src_mac: interface.mac.unwrap(), // This is safe!!
            ipv4_net: interface.get_ipv4_net(),
            link_local: interface.get_link_local_addr(),
            dst_addr_v4: None,
            dst_addr_v6: None
        }
    }
}

pub fn discover_via_eth() -> anyhow::Result<Vec<InternalHost>> {
    let (interface, sender_context) = get_interface_and_sender_context();
    let (mut tx, rx) = open_eth_channel(&interface, &get_config())?;
    let duration_in_ms: Duration = Duration::from_millis(PROBE_TIMEOUT_MS);
    let packet_types: Vec<PacketType> = vec![PacketType::Arp, PacketType::Icmpv6];
    let packets: Vec<Vec<u8>> = packets::create_multiple_packets(&sender_context, packet_types)?;
    for packet in packets { 
        tx.send_to(&packet, None); 
    }
    Ok(listen_for_hosts(rx, duration_in_ms, sender_context.src_mac))
}

pub fn discover_via_ip_addr(dst_addr: IpAddr) -> anyhow::Result<Option<InternalHost>> {
    let (interface, mut sender_context) = get_interface_and_sender_context();
    let (mut tx, rx) = open_eth_channel(&interface, &get_config())?;
    let duration_in_ms: Duration = Duration::from_millis(PROBE_TIMEOUT_MS);
    let packet: Vec<u8> = match dst_addr {
        IpAddr::V4(dst_addr_v4) => {
            sender_context.dst_addr_v4 = Some(dst_addr_v4);
            packets::create_single_packet(&sender_context, PacketType::Arp)?
        },
        IpAddr::V6(dst_addr_v6) => {
            sender_context.dst_addr_v6 = Some(dst_addr_v6);
            packets::create_single_packet(&sender_context, PacketType::Ndp)?
        },
    };
    tx.send_to(&packet, None);
    let host: Option<InternalHost> = listen_for_hosts(rx, duration_in_ms, sender_context.src_mac)
        .into_iter()
        .find(|host| host.ips.contains(&dst_addr));
    Ok(host)
}

fn open_eth_channel(intf: &NetworkInterface, cfg: &Config)
    -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)> {
    let ch = datalink::channel(intf, *cfg).with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => {
            print::print_status("Connection established successfully.");
            Ok((tx, rx))
        },
        _ => bail!("non-ethernet channel for {}", intf.name),
    }
}

fn listen_for_hosts(mut rx: Box<dyn DataLinkReceiver>, duration_in_ms: Duration, src_mac: MacAddr) -> Vec<InternalHost> {
    let mut hosts: Vec<InternalHost> = Vec::new();
    let deadline = Instant::now() + duration_in_ms;
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Some(host) = packets::handle_frame(&frame).ok() {
                    if host.mac_addr != src_mac {
                        hosts.push(host); 
                    }
                }
            },
            Err(_) => { }
        }
    }
    hosts
}

fn get_interface_and_sender_context() -> (NetworkInterface, SenderContext) {
    let interface: NetworkInterface = interface::get_lan();
    let sender_context: SenderContext = SenderContext::from(&interface);
    (interface, sender_context)
}

fn get_config() -> Config {
    Config {
        read_timeout: Some(Duration::from_millis(50)),
        ..Default::default()
    }
}