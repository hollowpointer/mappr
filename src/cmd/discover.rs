use crate::cmd::Target;
use crate::host::{self, Host, InternalHost};
use crate::net::datalink::channel::EthernetHandle;
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::datalink::{channel, interface};
use crate::net::ip;
use crate::net::runner::local::LocalRunner;
use crate::net::sender::SenderConfig;
use crate::net::tcp_connect;
use crate::net::transport::{self, UdpHandle};
use crate::terminal::spinner::SPINNER;
use crate::terminal::{colors, print};
use crate::utils::input::InputHandle;
use crate::utils::timing::ScanTimer;
use anyhow::{self, Context};
use colored::*;
use is_root::is_root;
use pnet::datalink::NetworkInterface;
use std::collections::HashSet;
use std::net::IpAddr;
use std::ops::ControlFlow;
use std::time::{Duration, Instant};

const MAX_CHANNEL_TIME: Duration = Duration::from_millis(7_500);
const MIN_CHANNEL_TIME: Duration = Duration::from_millis(2_500);
const MAX_SILENCE: Duration = Duration::from_millis(500);

pub async fn discover(target: Target) -> anyhow::Result<()> {
    SPINNER.set_message("Performing discovery...");
    print::print_status("Initializing discovery...");

    let start_time: Instant = Instant::now();
    let (targets, lan_interface) = get_targets_and_lan_intf(target)?;

    if !is_root() {
        print::print_status("No root privileges. Falling back to non-privileged TCP scan...");
        let mut hosts = host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?,
        );
        return Ok(discovery_ends(&mut hosts, start_time.elapsed())?);
    }

    print::print_status("Root privileges detected. Using advanced techniques...");

    let mut hosts = if let Some(intf) = lan_interface {
        let mut sender_cfg = SenderConfig::from(&intf);
        sender_cfg.add_targets(targets);

        let discovered_hosts =
            tokio::task::spawn_blocking(move || discover_lan(intf, sender_cfg)).await??;

        host::internal_to_box(discovered_hosts)
    } else {
        host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?,
        )
    };

    Ok(discovery_ends(&mut hosts, start_time.elapsed())?)
}

fn get_targets_and_lan_intf(
    target: Target,
) -> anyhow::Result<(HashSet<IpAddr>, Option<NetworkInterface>)> {
    match target {
        Target::LAN => {
            let intf =
                interface::get_lan().context("Failed to detect LAN interface for discovery")?;
            let range = intf
                .get_ipv4_range()
                .context("LAN interface has no valid IPv4 range")?;
            Ok((range.to_iter().collect::<HashSet<_>>(), Some(intf)))
        }
        Target::Host { target_addr } => {
            let intf = if ip::is_private(&target_addr) {
                interface::get_lan().ok()
            } else {
                None
            };
            Ok((HashSet::from([target_addr]), intf))
        }
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
        }
        Target::VPN => anyhow::bail!("Target::VPN is currently unimplemented!"),
    }
}

pub fn discover_lan(
    intf: NetworkInterface,
    sender_cfg: SenderConfig,
) -> anyhow::Result<Vec<InternalHost>> {
    let eth_handle: EthernetHandle = channel::start_capture(&intf)?;
    let udp_handle: UdpHandle = transport::start_capture()?;
    let input_handle: InputHandle = InputHandle::new();
    let timer = ScanTimer::new(MAX_CHANNEL_TIME, MIN_CHANNEL_TIME, MAX_SILENCE);

    let mut local_runner: LocalRunner =
        LocalRunner::new(sender_cfg, input_handle, eth_handle, udp_handle, timer)?;

    local_runner.send_discovery_packets()?;
    local_runner.start_input_listener();

    loop {
        if let ControlFlow::Break(_) = local_runner.process_packets() {
            break;
        }
    }

    Ok(local_runner.get_hosts())
}

fn discovery_ends(hosts: &mut Vec<Box<dyn Host>>, total_time: Duration) -> anyhow::Result<()> {
    if hosts.is_empty() {
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
    print::centerln(
        &format!(
            "Discovery Complete: {} identified in {}",
            active_hosts, total_time
        )
        .color(colors::TEXT_DEFAULT),
    );
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
