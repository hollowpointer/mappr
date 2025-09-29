use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow;
use anyhow::{bail, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::util::MacAddr;
use crate::cmd::Target;
use crate::net::packets::icmp;
use crate::host::Host;
use crate::net::{packets, range};
use crate::net::datalink::{interface, arp};
use crate::net::range::Ipv4Range;
use crate::print;

pub enum ProbeType {
    Default
}

pub struct SenderContext {
    pub ipv4range: Arc<Ipv4Range>,
    pub src_addr_v4: Ipv4Addr,
    pub src_addr_v6: Ipv6Addr,
    pub mac_addr: MacAddr,
    pub tx: Box<dyn DataLinkSender>
}

impl SenderContext {
    fn new(ipv4range: Arc<Ipv4Range>,
           src_addr_v4: Ipv4Addr,
           src_addr_v6: Ipv6Addr,
           intf: Arc<NetworkInterface>,
           tx: Box<dyn DataLinkSender>)
     -> Self {
        Self {
            ipv4range,
            src_addr_v4,
            src_addr_v6,
            mac_addr: intf.mac.unwrap(),
            tx,
        }
    }
}

pub fn discover_on_eth_channel(ipv4range: Arc<Ipv4Range>,
                               intf: Arc<NetworkInterface>,
                               channel_cfg: Config,
                               probe_type: ProbeType,
                               duration_in_ms: Duration
) -> anyhow::Result<Vec<Host>> {
    let (tx, rx) = open_eth_channel(&intf, &channel_cfg)?;
    let _ = range::ip_range(Target::LAN, &intf); // this shit is to suppress warnings
    let src_addr_v4: Ipv4Addr = interface::get_ipv4(&intf)?;
    let src_addr_v6: Ipv6Addr = interface::get_ipv6(&intf)?;
    let mut send_context: SenderContext = SenderContext::new(ipv4range, src_addr_v4, src_addr_v6, intf, tx);
    match probe_type {
        ProbeType::Default => {
            arp::send_packets(&mut send_context)?;
            icmp::send_echo_request_v6(&mut send_context)?;
        },
    }
    Ok(listen_for_hosts(rx, duration_in_ms))
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