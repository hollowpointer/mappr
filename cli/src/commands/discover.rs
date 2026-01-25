use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::time::{Duration, Instant};

use anyhow;
use colored::*;
use tracing::info_span;

use crate::{mprint, terminal::{colors, format, print, spinner}};
use mappr_common::{config::Config, network::host::Host, success};
use mappr_common::network::range::IpCollection;
use mappr_core::scanner;

type Detail = (String, ColoredString);

pub async fn discover(ips: IpCollection, cfg: &Config) -> anyhow::Result<()> {
    let span = info_span!("discovery", indicatif.pb_show = true);
    let guard = span.enter();

    let running: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    let spinner_handle = spinner::start_discovery_spinner(span.clone(), running.clone());

    let start_time: Instant = Instant::now();
    let mut hosts: Vec<Host> = scanner::perform_discovery(ips, cfg).await?;

    running.store(false, Ordering::Relaxed);
    let _ = spinner_handle.join();

    drop(guard);

    discovery_ends(&mut hosts, start_time.elapsed(), cfg);
    Ok(())
}

fn discovery_ends(hosts: &mut [Host], total_time: Duration, cfg: &Config) {
    if hosts.is_empty() {
        no_hosts_found(cfg);
        return;
    }

    if cfg.quiet > 0 {
        mprint!();
    }

    print::header("Network Discovery", cfg.quiet);
    hosts.sort_by_key(|host| *host.ips.iter().next().unwrap_or(&host.ip));
    print_hosts(hosts, cfg);
    print_summary(hosts.len(), total_time, cfg);
}

fn no_hosts_found(cfg: &Config) {
    print::header("ZERO HOSTS DETECTED", cfg.quiet);
    print::no_results();
}

fn print_hosts(hosts: &mut [Host], cfg: &Config) {
    for (idx, host) in hosts.iter().enumerate() {
        match cfg.quiet {
            2 => { },
            _ => print_host_tree(host, idx, cfg)
        }
        if idx + 1 != hosts.len() {
            mprint!();
        }
    }
}

fn print_summary(hosts_len: usize, total_time: Duration, cfg: &Config) {
    let active_hosts: ColoredString = format!("{hosts_len} active hosts").bold().green();
    let total_time: ColoredString = format!("{:.2}s", total_time.as_secs_f64()).bold().yellow();
    let output: &ColoredString = &format!("Discovery Complete: {active_hosts} identified in {total_time}")
                .color(colors::TEXT_DEFAULT);

    match cfg.quiet {
        0 => {
            print::fat_separator();
            print::centerln(output);
        },
        _ => {
            mprint!();
            success!("{}", output)
        },
    }

}

fn print_host_tree(host: &Host, idx: usize, cfg: &Config) {
    let hostname = host.hostname.as_deref().unwrap_or("No hostname");
    print::tree_head(idx, hostname);
    let mut details: Vec<Detail> = format::ip_to_detail(&host.ips, cfg);

    if let Some(mac_detai) = format::mac_to_detail(&host.mac, cfg) {
        details.push(mac_detai);
    }

    if let Some(vendor_detail) = format::vendor_to_detail(&host.vendor) {
        details.push(vendor_detail);
    }

    if !host.network_roles.is_empty() {
        let joined_roles: String = host
            .network_roles
            .iter()
            .map(|role| format!("{:?}", role))
            .collect::<Vec<String>>()
            .join(", ");

        let roles_detail: (String, ColoredString) = ("Roles".to_string(), joined_roles.normal());

        details.push(roles_detail);
    }

    print::as_tree_one_level(details);
}
