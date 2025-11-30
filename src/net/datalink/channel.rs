use crate::host::{self, InternalHost};
use crate::net::datalink::interface;
use crate::net::packets;
use crate::net::range::Ipv4Range;
use crate::net::sender::SenderConfig;
use crate::net::transport;
use crate::print;
use crate::utils::print::SPINNER;
use anyhow::{self, Context};
use colored::*;
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::util::MacAddr;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};
use threadpool::ThreadPool;

const PROBE_TIMEOUT_MS: u64 = 2000;

struct ChannelRunner<'a> {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    duration: Duration,
    sender_config: &'a SenderConfig,
}

impl<'a> ChannelRunner<'a> {
    pub fn new(
        interface: &datalink::NetworkInterface,
        sender_config: &'a SenderConfig,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = open_eth_channel(interface, &get_config(), datalink::channel)?;
        let duration = Duration::from_millis(PROBE_TIMEOUT_MS);
        Ok(Self {
            tx,
            rx,
            duration,
            sender_config,
        })
    }

    pub fn send_packets(&mut self) -> anyhow::Result<()> {
        let packets: Vec<Vec<u8>> = packets::create_packets(self.sender_config)?;
        for packet in packets {
            self.tx.send_to(&packet, None);
        }
        Ok(())
    }

    pub fn listen(self) -> anyhow::Result<Vec<InternalHost>> {
        listen_for_hosts(self.rx, self.duration, self.sender_config)
    }
}

pub fn discover_subnet() -> anyhow::Result<Vec<InternalHost>> {
    let (interface, mut sender_config) = get_interface_and_sender_config()?;
    let range: Ipv4Range = sender_config.get_ipv4_range()?;
    sender_config.add_targets(range.to_iter());
    let mut runner: ChannelRunner = ChannelRunner::new(&interface, &sender_config)?;
    runner.send_packets()?;
    runner.listen()
}

pub fn discover_via_ip_addr(target_addr: IpAddr) -> anyhow::Result<Option<InternalHost>> {
    let (interface, mut sender_config) = get_interface_and_sender_config()?;
    sender_config.add_target(target_addr);
    let mut runner: ChannelRunner = ChannelRunner::new(&interface, &sender_config)?;
    runner.send_packets()?;
    let hosts: Vec<InternalHost> = runner.listen()?;
    let host: Option<InternalHost> = hosts.into_iter().find(|host| host.ips.contains(&target_addr));
    Ok(host)
}

pub fn discover_via_range(ipv4_range: Ipv4Range) -> anyhow::Result<Vec<InternalHost>> {
    let (interface, mut sender_config) = get_interface_and_sender_config()?;
    sender_config.add_targets(ipv4_range.to_iter());
    let mut runner: ChannelRunner = ChannelRunner::new(&interface, &sender_config)?;
    runner.send_packets()?;
    runner.listen()
}

fn open_eth_channel<F>(
    intf: &NetworkInterface,
    cfg: &Config,
    channel_opener: F,
) -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)>
where F: FnOnce(&NetworkInterface, Config) -> std::io::Result<datalink::Channel>,
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
    sender_config: &SenderConfig,
) -> anyhow::Result<Vec<InternalHost>> {  
    let (tx, rx) = mpsc::channel();
    sniff_and_dispatch(rx_datalink, &tx, duration_in_ms, sender_config)?;
    drop(tx);
    let mut hosts: Vec<InternalHost> = rx.iter().collect();
    host::merge_by_mac(&mut hosts);
    Ok(hosts)
}

fn sniff_and_dispatch(
    mut rx_datalink: Box<dyn DataLinkReceiver>,
    tx: &Sender<InternalHost>,
    duration_in_ms: Duration,
    sender_config: &SenderConfig,
) -> anyhow::Result<()> {
    let deadline: Instant = Instant::now() + duration_in_ms;
    let thread_pool: ThreadPool = ThreadPool::new(50);
    let mut unique_hosts: HashMap<MacAddr, HashSet<IpAddr>> = HashMap::new();

    while Instant::now() < deadline {
        let frame = match rx_datalink.next() {
            Ok(f) => f,
            Err(_) => continue,
        };

        let (mac, ip) = match extract_new_target(frame, &mut unique_hosts) {
            Some(target) => target,
            None => continue,
        };

        process_and_queue_host(
            mac, 
            ip, 
            unique_hosts.len(), 
            tx, 
            &thread_pool, 
            sender_config
        )?;
    }
    
    thread_pool.join();
    Ok(())
}

fn extract_new_target(
    frame: &[u8],
    unique_hosts: &mut HashMap<MacAddr, HashSet<IpAddr>>,
) -> Option<(MacAddr, IpAddr)> {
    let (mac, ip) = packets::handle_frame(frame).ok()??;
    let host_ips = unique_hosts.entry(mac).or_default();

    if host_ips.insert(ip) {
        return Some((mac, ip));
    }

    None
}

fn process_and_queue_host(
    mac: MacAddr,
    ip: IpAddr,
    host_count: usize,
    tx: &Sender<InternalHost>,
    pool: &ThreadPool,
    config: &SenderConfig,
) -> anyhow::Result<()> {
    if let Some(mut host) = process_packet_for_host(mac, ip, config)? {
        update_spinner(host_count);
        let tx_inner: Sender<InternalHost> = tx.clone();        
        pool.execute(move || {
            let _ = transport::try_dns_reverse_lookup(&mut host);
            if let Err(e) = tx_inner.send(host) {
                eprintln!("Failed to send host: {}", e);
            }
        });
    }
    Ok(())
}

fn update_spinner(count: usize) {
    let count_str = count.to_string();
    let colored_count = if count > 0 {
        count_str.color(Color::Green).bold()
    } else {
        count_str.color(Color::Red).bold()
    };
    
    SPINNER.set_message(format!(
        "Scanning network, found {colored_count} hosts so far...",
    ));
}

fn process_packet_for_host(
    target_mac: MacAddr,
    target_addr: IpAddr,
    sender_config: &SenderConfig,
) -> anyhow::Result<Option<InternalHost>> {
    let local_mac: MacAddr = sender_config.get_local_mac()?;
    if target_mac == local_mac {
        return Ok(None)
    }
    if target_addr.is_ipv4() && !sender_config.has_addr(&target_addr) {
        return Ok(None);
    }
    let mut host: InternalHost = host::InternalHost::from(target_mac);
    host.ips.insert(target_addr);
    Ok(Some(host))
}

fn get_interface_and_sender_config() -> anyhow::Result<(NetworkInterface, SenderConfig)> {
    let interface: NetworkInterface = interface::get_lan()?;
    let sender_config: SenderConfig = SenderConfig::from(&interface);
    Ok((interface, sender_config))
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
