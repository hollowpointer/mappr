use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::time::{Duration, Instant};

use anyhow;
use colored::*;
use tracing::info_span;

use crate::terminal::{colors, format, print, spinner};
use mappr_common::{config::Config, network::host::Host, success};
use mappr_common::network::range::IpCollection;
use mappr_core::scanner;

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
        print::println("");
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
            print::println("");
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
            print::println("");
            success!("{}", output)
        },
    }

}

fn print_host_tree(host: &Host, idx: usize, cfg: &Config) {
    let hostname = host.hostname.as_deref().unwrap_or("No hostname");
    print::tree_head(idx, hostname);
    let mut key_value_pair: Vec<(String, ColoredString)> = format::ip_to_key_value_pair(&host.ips, cfg);

    if let Some(mac_key_value) = format::mac_to_key_value_pair(&host.mac, cfg) {
        key_value_pair.push(mac_key_value);
    }

    if let Some(vendor_key_value) = format::vendor_to_key_value_pair(&host.vendor) {
        key_value_pair.push(vendor_key_value);
    }

    if !host.network_roles.is_empty() {
        let joined_roles: String = host
            .network_roles
            .iter()
            .map(|role| format!("{:?}", role))
            .collect::<Vec<String>>()
            .join(", ");

        let roles_key_value: (String, ColoredString) = ("Roles".to_string(), joined_roles.normal());

        key_value_pair.push(roles_key_value);
    }

    print::as_tree_one_level(key_value_pair);
}
