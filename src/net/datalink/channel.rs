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
use crate::host::{self, InternalHost};
use crate::net::range::{self, Ipv4Range};
use crate::print;

const PROBE_TIMEOUT_MS: u64 = 2000;

pub struct SenderContext {
    pub src_mac: MacAddr,
    pub ipv4_net: Option<Ipv4Network>,
    pub ipv4_range: Option<Ipv4Range>,
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
            ipv4_range: None,
            dst_addr_v4: None,
            dst_addr_v6: None
        }
    }
}


pub fn discover_via_eth() -> anyhow::Result<Vec<InternalHost>> {
    let (interface, sender_context) = get_interface_and_sender_context()?;
    let (mut tx, rx) = open_eth_channel(&interface, &get_config(), datalink::channel)?;
    let duration_in_ms: Duration = Duration::from_millis(PROBE_TIMEOUT_MS);
    let packet_types: Vec<PacketType> = vec![PacketType::Arp, PacketType::Icmpv6];
    let packets: Vec<Vec<u8>> = packets::create_multiple_packets(&sender_context, packet_types)?;
    for packet in packets { 
        tx.send_to(&packet, None); 
    }
    Ok(listen_for_hosts(rx, duration_in_ms, &sender_context))
}


pub fn discover_via_ip_addr(dst_addr: IpAddr) -> anyhow::Result<Option<InternalHost>> {
    let (interface, mut sender_context) = get_interface_and_sender_context()?;
    let (mut tx, rx) = open_eth_channel(&interface, &get_config(), datalink::channel)?;
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
    let host: Option<InternalHost> = listen_for_hosts(rx, duration_in_ms, &sender_context)
        .into_iter()
        .find(|host| host.ips.contains(&dst_addr));
    Ok(host)
}


pub fn discover_via_range(ipv4_range: Ipv4Range) -> anyhow::Result<Vec<InternalHost>> {
    let (interface, mut sender_context) = get_interface_and_sender_context()?;
    sender_context.ipv4_range = Some(ipv4_range);
    let (mut tx, rx) = open_eth_channel(&interface, &get_config(), datalink::channel)?;
    let duration_in_ms: Duration = Duration::from_millis(PROBE_TIMEOUT_MS);
    let packet_types: Vec<PacketType> = vec![PacketType::Arp];
    let packets: Vec<Vec<u8>> = packets::create_multiple_packets(&sender_context, packet_types)?;
    for packet in packets { 
        tx.send_to(&packet, None); 
    }
    Ok(listen_for_hosts(rx, duration_in_ms, &sender_context))
}


fn open_eth_channel<F>(intf: &NetworkInterface, cfg: &Config, channel_opener: F) 
    -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)> 
where F: FnOnce(&NetworkInterface, Config) -> std::io::Result<datalink::Channel>
{
    let ch: Channel = channel_opener(intf, *cfg).with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => {
            print::print_status("Connection established successfully.");
            Ok((tx, rx))
        },
        _ => bail!("non-ethernet channel for {}", intf.name),
    }
}


fn listen_for_hosts(mut rx: Box<dyn DataLinkReceiver>, duration_in_ms: Duration, sender_context: &SenderContext) -> Vec<InternalHost> {
    let mut hosts: Vec<InternalHost> = Vec::new();
    let deadline = Instant::now() + duration_in_ms;
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Ok(Some(host)) = packets::handle_frame(frame, sender_context) {
                    hosts.push(host);
                }
            },
            Err(_) => { }
        }
    }
    host::merge_by_mac(&mut hosts);
    hosts
}


fn get_interface_and_sender_context() -> anyhow::Result<(NetworkInterface, SenderContext)> {
    let interface: NetworkInterface = interface::get_lan()?;
    let sender_context: SenderContext = SenderContext::from(&interface);
    Ok((interface, sender_context))
}


fn get_config() -> Config {
    Config {
        read_timeout: Some(Duration::from_millis(50)),
        ..Default::default()
    }
}



// ╔════════════════════════════════════════════╗
// ║ ████████╗███████╗███████╗████████╗███████╗ ║
// ║ ╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝ ║
// ║    ██║   █████╗  ███████╗   ██║   ███████╗ ║
// ║    ██║   ██╔══╝  ╚════██║   ██║   ╚════██║ ║
// ║    ██║   ███████╗███████║   ██║   ███████║ ║
// ║    ╚═╝   ╚══════╝╚══════╝   ╚═╝   ╚══════╝ ║
// ╚════════════════════════════════════════════╝

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::datalink::dummy;
    use pnet::datalink::{Config, NetworkInterface};

    #[test]
    fn open_eth_channel_should_succeed_on_ethernet_channel() {
        let dummy_intf: NetworkInterface = dummy::dummy_interface(0);
        let cfg = Config::default();
        let mock_opener_success = |i: &NetworkInterface, _cfg: Config| 
        -> std::io::Result<datalink::Channel> {
            let dummy_cfg = pnet::datalink::dummy::Config::default();
            datalink::dummy::channel(i, dummy_cfg)
        };
        let result = open_eth_channel(&dummy_intf, &cfg, mock_opener_success);
        assert!(result.is_ok());
    }

    #[test]
    fn open_eth_channel_should_fail_on_io_error() {
        let dummy_intf: NetworkInterface = dummy::dummy_interface(0);
        let cfg: Config = Config::default();
        let mock_opener_fail = |_: &NetworkInterface, _: Config| 
        -> std::io::Result<datalink::Channel> {
            Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Mock I/O Error"))
        };
        let result = open_eth_channel(&dummy_intf, &cfg, mock_opener_fail);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("opening on eth0"));
            let cause: Option<&std::io::Error> = e.downcast_ref::<std::io::Error>();
            assert!(cause.is_some(), "Error cause was not an std::io::Error");
            assert_eq!(cause.unwrap().to_string(), "Mock I/O Error");
            assert_eq!(cause.unwrap().kind(), std::io::ErrorKind::PermissionDenied);
        } else {
            panic!("Test failed: expected Err, got Ok");
        }
    }
}