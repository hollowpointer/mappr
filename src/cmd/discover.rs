use crate::cmd::Target;
use crate::host::{self, Host, InternalHost};
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::datalink::{channel, ethernet, interface};
use crate::net::{packets, transport};
use crate::net::sender::SenderConfig;
use crate::net::tcp_connect;
use crate::net::ip;
use crate::print::{self, SPINNER};
use anyhow::{self, Context};
use is_root::is_root;
use pnet::datalink::{self, NetworkInterface};
use pnet::util::MacAddr;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const MAX_CHANNEL_TIME_MS: u64 = 3000;
const MAX_SILENCE_TIME_MS: u64 = 200;

pub async fn discover(target: Target) -> anyhow::Result<()> {
    SPINNER.set_message("Performing discovery...");
    print::print_status("Initializing discovery...");

    let (targets, lan_interface) = get_targets_and_lan_intf(target)?;

    if !is_root() {
        print::print_status("No root privileges. Falling back to non-privileged TCP scan...");
        let mut hosts = host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?
        );
        return Ok(discovery_ends(&mut hosts)?);
    }

    print::print_status("Root privileges detected. Using advanced techniques...");

    let mut hosts = if let Some(intf) = lan_interface {
        let mut sender_cfg = SenderConfig::from(&intf);
        sender_cfg.add_targets(targets);
        host::internal_to_box(discover_lan(&intf, &sender_cfg)?)
    } else {
        host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?
        )
    };

    Ok(discovery_ends(&mut hosts)?)
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

pub fn discover_lan(
    intf: &NetworkInterface, 
    sender_cfg: &SenderConfig
) -> anyhow::Result<Vec<InternalHost>> {

    let mut hosts_map: HashMap<MacAddr, InternalHost> = HashMap::new();

    let (tx, rx) = mpsc::channel();
    let (eth_tx, mut eth_rx) = channel::open_eth_channel(intf, datalink::channel)?;

    thread::spawn(move || {
        let deadline: Instant = Instant::now() + Duration::from_millis(MAX_CHANNEL_TIME_MS);
        let mut receive_window: Instant = Instant::now() + Duration::from_millis(MAX_SILENCE_TIME_MS);
        while Instant::now() < deadline && Instant::now() < receive_window {
            if let Ok(frame) = eth_rx.next() {
                let _ = tx.send(frame.to_vec());
                receive_window = Instant::now() + Duration::from_millis(MAX_SILENCE_TIME_MS);
            }
        }
    });

    let _ = channel::send_packets(eth_tx, sender_cfg);

    while let Ok(frame_bytes) = rx.recv() {
        if let Ok(eth_frame) = ethernet::get_packet_from_u8(&frame_bytes) {
            let target_mac = eth_frame.get_source();
    
            let target_addr = match packets::get_ip_addr_from_eth(&eth_frame) {
                Ok(ip) => ip,
                Err(_) => continue,
            };

            if target_addr.is_ipv4() && !sender_cfg.has_addr(&target_addr) {
                continue;
            }

            hosts_map
                .entry(target_mac)
                .or_insert_with(|| InternalHost::from(target_mac))
                .ips
                .insert(target_addr);
        }
    }

    enrich_with_hostnames(&mut hosts_map);
    Ok(hosts_map.into_values().collect())
}

fn enrich_with_hostnames(hosts_map: &mut HashMap<MacAddr, InternalHost>) {
    hosts_map.par_iter_mut().for_each(|(_, host)| {
        if let Some(&target_addr) = host.ips.iter().find(|ip| ip.is_ipv4()) {
            if let Ok(hostname) = transport::get_hostname_via_rdns(target_addr) {
                host.set_hostname(hostname);
            }
        }
    });
}

fn discovery_ends(hosts: &mut Vec<Box<dyn Host>>) -> anyhow::Result<()>  {
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
