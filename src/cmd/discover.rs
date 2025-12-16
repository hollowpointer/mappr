use crate::cmd::Target;
use crate::host::{self, Host, InternalHost};
use crate::net::datalink::channel::EthernetHandle;
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::datalink::{channel, ethernet, interface};
use crate::net::packets::dns;
use crate::net::transport::UdpHandle;
use crate::net::{packets, transport};
use crate::net::sender::SenderConfig;
use crate::net::tcp_connect;
use crate::net::ip;
use crate::print::{self, SPINNER};
use crate::utils::colors;
use crate::utils::timing::ScanTimer;
use anyhow::{self, Context};
use colored::*;
use is_root::is_root;
use pnet::datalink::NetworkInterface;
use pnet::packet::Packet;
use pnet::packet::udp::UdpPacket;
use pnet::util::MacAddr;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::ops::ControlFlow;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_CHANNEL_TIME: Duration = Duration::from_millis(10_000);
const MIN_CHANNEL_TIME: Duration = Duration::from_millis(2_500);
const MAX_SILENCE: Duration = Duration::from_millis(500);

struct LocalRunner {
    hosts_map: HashMap<MacAddr, InternalHost>,
    dns_map: HashMap<u16, MacAddr>,
    sender_cfg: SenderConfig,
    eth_handle: EthernetHandle,
    udp_handle: UdpHandle,
    timer: ScanTimer
}

impl LocalRunner {
    fn new(sender_cfg: SenderConfig, eth_handle: EthernetHandle, udp_handle: UdpHandle) 
    -> anyhow::Result<Self> {
        let timer = ScanTimer::new(MAX_CHANNEL_TIME, MIN_CHANNEL_TIME, MAX_SILENCE);
        Ok(
            Self { 
                hosts_map: HashMap::new(), 
                dns_map: HashMap::new(),
                sender_cfg,
                eth_handle,
                udp_handle,
                timer
            }
        )
    }

    fn send_discovery_packets(&mut self) -> anyhow::Result<()> {
        channel::send_packets(&mut self.eth_handle.tx, &self.sender_cfg)?;
        Ok(())
    }

    fn process_packets(&mut self) -> ControlFlow<()> {
        if self.timer.is_expired() {
            return ControlFlow::Break(());
        }
        let wait = self.timer.next_wait();

        match self.eth_handle.rx.recv_timeout(wait) {
            Ok(bytes) => {
                self.timer.mark_seen();
                self.process_eth_packet(&bytes);
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if self.timer.should_break_on_timeout() {
                    return ControlFlow::Break(());
                }
                return ControlFlow::Continue(());
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return ControlFlow::Break(());
            },
        }

        self.process_udp_packets();
        ControlFlow::Continue(())
    }

    fn process_eth_packet(&mut self, bytes: &[u8]) {
        let Ok(eth_frame) = ethernet::get_packet_from_u8(bytes) else { return };
        let Ok(target_addr) = packets::get_ip_addr_from_eth(&eth_frame) else { return };

        if target_addr.is_ipv4() && !self.sender_cfg.has_addr(&target_addr) { return }

        let target_mac = eth_frame.get_source();
        self.hosts_map
            .entry(target_mac)
            .or_insert_with(|| InternalHost::from(target_mac))
            .ips
            .insert(target_addr);

        report_discovery_progress(self.hosts_map.len());
        self.send_dns_ptr_query(&target_addr, target_mac);
    }

    fn process_udp_packets(&mut self) {
        while let Ok(bytes) = self.udp_handle.rx.try_recv() {
            self.timer.mark_seen();
            let Some(udp_packet) = UdpPacket::new(&bytes) else { continue };
            match udp_packet.get_source() {
                53 => self.handle_dns_response(udp_packet),
                _ => { continue }
            }
        }
    }

    fn handle_dns_response(&mut self, packet: UdpPacket) {
        let Ok(Some((response_id, name))) = dns::get_hostname(packet.payload()) else { return };
        let Some(mac_addr) = self.dns_map.get(&response_id) else { return };
        if let Some(host) = self.hosts_map.get_mut(mac_addr) {
            host.set_hostname(name);
        }
    }

    fn send_dns_ptr_query(&mut self, target_addr: &IpAddr, target_mac: MacAddr) {
        if !target_addr.is_ipv4() && !ip::is_global_unicast(&target_addr) { return }
        let id: u16 = self.dns_map.len() as u16;
        if self.dns_map.contains_key(&id) { return }
        self.dns_map.insert(id, target_mac);
        let id: u16 = (self.dns_map.len() - 1) as u16;
        transport::send_dns_query(dns::create_ptr_packet, id, &target_addr, &mut self.udp_handle.tx);
    }

    fn get_hosts(self) -> Vec<InternalHost> {
        return self.hosts_map.into_values().collect()
    }
}

pub async fn discover(target: Target) -> anyhow::Result<()> {
    SPINNER.set_message("Performing discovery...");
    print::print_status("Initializing discovery...");

    let start_time: Instant = Instant::now();
    let (targets, lan_interface) = get_targets_and_lan_intf(target)?;

    if !is_root() {
        print::print_status("No root privileges. Falling back to non-privileged TCP scan...");
        let mut hosts = host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?
        );
        return Ok(discovery_ends(&mut hosts, start_time.elapsed())?);
    }

    print::print_status("Root privileges detected. Using advanced techniques...");

    let mut hosts = if let Some(intf) = lan_interface {
        let mut sender_cfg = SenderConfig::from(&intf);
        sender_cfg.add_targets(targets);
        host::internal_to_box(discover_lan(intf, sender_cfg)?)
    } else {
        host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?
        )
    };

    Ok(discovery_ends(&mut hosts, start_time.elapsed())?)
}

fn get_targets_and_lan_intf(target: Target) -> anyhow::Result<(HashSet<IpAddr>, Option<NetworkInterface>)> {
    match target {
        Target::LAN => {
            let intf = interface::get_lan().context("Failed to detect LAN interface for discovery")?;
            let range = intf.get_ipv4_range().context("LAN interface has no valid IPv4 range")?;
            Ok((range.to_iter().collect::<HashSet<_>>(), Some(intf)))
        },
        Target::Host { target_addr } => {
            let intf = if ip::is_private(&target_addr) { interface::get_lan().ok() } else { None };
            Ok((HashSet::from([target_addr]), intf))
        },
        Target::Range { ipv4_range } => {
            let targets: HashSet<IpAddr> = ipv4_range.to_iter().collect();
            let start = IpAddr::V4(ipv4_range.start_addr);
            let end = IpAddr::V4(ipv4_range.end_addr);
            let intf = if ip::is_private(&start) && ip::is_private(&end) {
                interface::get_lan().ok() 
            } else { 
                None
            };
            Ok((targets, intf))
        },
        Target::VPN => anyhow::bail!("Target::VPN is currently unimplemented!"),
    }
}

pub fn discover_lan(intf: NetworkInterface, sender_cfg: SenderConfig) -> anyhow::Result<Vec<InternalHost>> {
    let eth_handle: EthernetHandle = channel::start_capture(&intf)?;
    let udp_handle: UdpHandle = transport::start_capture()?;
    let mut local_runner: LocalRunner = LocalRunner::new(sender_cfg, eth_handle, udp_handle)?;
    local_runner.send_discovery_packets()?;

    loop {
        if let ControlFlow::Break(_) = local_runner.process_packets() {
            break;
        }
    }

    Ok(local_runner.get_hosts())
}

fn report_discovery_progress(count: usize) {
    SPINNER.set_message(
        format!(
            "Identified {} so far...", 
            format!("{} hosts", count).green().bold()
        )
        .color(colors::TEXT_DEFAULT)
        .to_string()
    );
}

fn discovery_ends(hosts: &mut Vec<Box<dyn Host>>, total_time: Duration) -> anyhow::Result<()>  {
    if hosts.len() == 0 {
        return Ok(no_hosts_found());
    }
    print::header("Network Discovery");
    hosts.sort_by_key(|host| host.get_primary_ip());
    for (idx, host) in hosts.iter().enumerate() {
        host.print_details(idx);
        if idx + 1 != hosts.len() {
            print::println("");
        }
    }
    print::fat_separator();
    let active_hosts: ColoredString = format!("{} active hosts", hosts.len()).bold().green();
    let total_time: ColoredString = format!("{:.2}s", total_time.as_secs_f64()).bold().yellow();
    print::centerln(&format!("Discovery Complete: {} identified in {}", active_hosts, total_time).color(colors::TEXT_DEFAULT));
    print::end_of_program();
    SPINNER.finish_and_clear();
    Ok(())
}

fn no_hosts_found() {
    print::header("ZERO HOSTS DETECTED");
    print::no_results();
    print::end_of_program();
    SPINNER.finish_and_clear();
}
