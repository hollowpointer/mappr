use crate::cmd::Target;
use crate::host::{self, Host, InternalHost};
use crate::net::datalink::channel::ChannelRunner;
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::datalink::interface;
use crate::net::{packets, transport};
use crate::net::sender::SenderConfig;
use crate::net::tcp_connect;
use crate::net::ip;
use crate::print::{self, SPINNER};
use anyhow::{self, Context};
use is_root::is_root;
use pnet::datalink::NetworkInterface;
use pnet::util::MacAddr;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

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
    // Phase 1: Scan network and build basic MAC/IP map
    let mut hosts_map = collect_active_hosts(intf, sender_cfg)?;
    // Phase 2: Parallel rDNS lookups
    enrich_with_hostnames(&mut hosts_map);
    Ok(hosts_map.into_values().collect())
}

fn collect_active_hosts(
    intf: &NetworkInterface,
    sender_cfg: &SenderConfig,
) -> anyhow::Result<HashMap<MacAddr, InternalHost>> {
    let mut runner = ChannelRunner::new(intf, sender_cfg)
        .context("Failed to initialize channel runner")?;

    runner.send_packets()?;
    let eth_frames = runner.receive_eth_frames();

    let mut hosts_map: HashMap<MacAddr, InternalHost> = HashMap::new();

    for frame in eth_frames {
        let target_mac = frame.get_source();
        
        let target_addr = match packets::get_ip_addr_from_eth(&frame) {
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

    Ok(hosts_map)
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
