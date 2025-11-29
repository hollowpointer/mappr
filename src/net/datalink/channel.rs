use crate::host::{self, InternalHost};
use crate::net::datalink::interface;
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::packets::{self, PacketType};
use crate::net::range::{self, Ipv4Range};
use crate::net::transport;
use crate::print;
use anyhow::{self, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::ipnetwork::Ipv4Network;
use pnet::util::MacAddr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};
use threadpool::ThreadPool;

const PROBE_TIMEOUT_MS: u64 = 2000;

struct ChannelRunner<'a> {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    duration: Duration,
    sender_context: &'a SenderContext,
}

impl<'a> ChannelRunner<'a> {
    pub fn new(
        interface: &datalink::NetworkInterface,
        sender_context: &'a SenderContext,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = open_eth_channel(interface, &get_config(), datalink::channel)?;
        let duration = Duration::from_millis(PROBE_TIMEOUT_MS);
        Ok(Self {
            tx,
            rx,
            duration,
            sender_context,
        })
    }

    pub fn send_discovery_packets(&mut self) -> anyhow::Result<()> {
        let packets = packets::create_discovery_packets(self.sender_context)?;
        for packet in packets {
            self.tx.send_to(&packet, None);
        }
        Ok(())
    }

    pub fn send_single_packet(&mut self, packet_type: PacketType) -> anyhow::Result<()> {
        let packet = packets::create_single_packet(self.sender_context, packet_type)?;
        self.tx.send_to(&packet, None);
        Ok(())
    }

    pub fn listen(self) -> anyhow::Result<Vec<InternalHost>> {
        listen_for_hosts(self.rx, self.duration, self.sender_context)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SenderContext {
    pub src_mac: MacAddr,
    pub ipv4_net: Option<Ipv4Network>,
    pub ipv4_range: Option<Ipv4Range>,
    pub link_local: Option<Ipv6Addr>,
    pub dst_addr_v4: Option<Ipv4Addr>,
    pub dst_addr_v6: Option<Ipv6Addr>,
}

impl From<&NetworkInterface> for SenderContext {
    fn from(interface: &NetworkInterface) -> Self {
        Self {
            src_mac: interface.mac.expect("Caller must verify interface has MAC"),
            ipv4_net: interface.get_ipv4_net(),
            link_local: interface.get_link_local_addr(),
            ipv4_range: None,
            dst_addr_v4: None,
            dst_addr_v6: None,
        }
    }
}

pub fn discover_via_eth() -> anyhow::Result<Vec<InternalHost>> {
    let (interface, sender_context) = get_interface_and_sender_context()?;
    let mut runner: ChannelRunner = ChannelRunner::new(&interface, &sender_context)?;
    runner.send_discovery_packets()?;
    runner.listen()
}

pub fn discover_via_ip_addr(dst_addr: IpAddr) -> anyhow::Result<Option<InternalHost>> {
    let (interface, mut sender_context) = get_interface_and_sender_context()?;
    let packet_type: PacketType = match dst_addr {
        IpAddr::V4(dst_addr_v4) => {
            sender_context.dst_addr_v4 = Some(dst_addr_v4);
            PacketType::Arp
        }
        IpAddr::V6(dst_addr_v6) => {
            sender_context.dst_addr_v6 = Some(dst_addr_v6);
            PacketType::Ndp
        }
    };
    let mut runner: ChannelRunner<'_> = ChannelRunner::new(&interface, &sender_context)?;
    runner.send_single_packet(packet_type)?;
    let hosts: Vec<InternalHost> = runner.listen()?;
    let host: Option<InternalHost> = hosts.into_iter().find(|host| host.ips.contains(&dst_addr));
    Ok(host)
}

pub fn discover_via_range(ipv4_range: Ipv4Range) -> anyhow::Result<Vec<InternalHost>> {
    let (interface, mut sender_context) = get_interface_and_sender_context()?;
    sender_context.ipv4_range = Some(ipv4_range);
    let mut runner: ChannelRunner = ChannelRunner::new(&interface, &sender_context)?;
    runner.send_discovery_packets()?;
    runner.listen()
}

fn open_eth_channel<F>(
    intf: &NetworkInterface,
    cfg: &Config,
    channel_opener: F,
) -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)>
where
    F: FnOnce(&NetworkInterface, Config) -> std::io::Result<datalink::Channel>,
{
    let ch: Channel =
        channel_opener(intf, *cfg).with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => {
            print::print_status("Connection established successfully");
            Ok((tx, rx))
        }
        _ => anyhow::bail!("non-ethernet channel for {}", intf.name),
    }
}

fn listen_for_hosts(
    rx_datalink: Box<dyn DataLinkReceiver>,
    duration_in_ms: Duration,
    sender_context: &SenderContext,
) -> anyhow::Result<Vec<InternalHost>> {  
    let (tx, rx) = mpsc::channel();
    sniff_and_dispatch(rx_datalink, &tx, duration_in_ms, sender_context);
    drop(tx);
    let mut hosts: Vec<InternalHost> = rx.iter().collect();
    host::merge_by_mac(&mut hosts);
    Ok(hosts)
}

fn sniff_and_dispatch(
    mut rx_datalink: Box<dyn DataLinkReceiver>,
    tx: &Sender<InternalHost>,
    duration_in_ms: Duration,
    sender_context: &SenderContext,
) {
    let deadline: Instant = Instant::now() + duration_in_ms;
    let thread_pool: ThreadPool = ThreadPool::new(50);
    while Instant::now() < deadline {
        let frame: &[u8] = match rx_datalink.next() {
            Ok(frame) => frame,
            Err(_) => continue,
        };

        let (mac_addr, ip_addr) =
            if let Ok(Some((mac, ip))) = packets::handle_frame(frame) {
                (mac, ip)
            } else {
                continue;
            };

        if let Some(mut host) = process_packet_for_host(mac_addr, ip_addr, sender_context) {
            let tx_inner: mpsc::Sender<InternalHost> = tx.clone();
            thread_pool.execute(move || {
                thread::spawn(move || {
                    let _ = transport::try_dns_reverse_lookup(&mut host);
                    tx_inner.send(host).unwrap(); 
                });
            });
        }
    }
    thread_pool.join();
}

fn process_packet_for_host(
    mac_addr: MacAddr,
    ip_addr: IpAddr,
    sender_context: &SenderContext,
) -> Option<InternalHost> {
    if mac_addr == sender_context.src_mac {
        return None;
    }
    let is_valid: bool = match ip_addr {
        IpAddr::V4(ipv4_addr) => range::in_optional_range(&ipv4_addr, &sender_context.ipv4_range),
        IpAddr::V6(_) => true,
    };
    if is_valid {
        let mut host: InternalHost = host::InternalHost::from(mac_addr);
        host.ips.insert(ip_addr);
        Some(host)
    } else {
        None
    }
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
        let mock_opener_success =
            |i: &NetworkInterface, _cfg: Config| -> std::io::Result<datalink::Channel> {
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
        let mock_opener_fail =
            |_: &NetworkInterface, _: Config| -> std::io::Result<datalink::Channel> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Mock I/O Error",
                ))
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
