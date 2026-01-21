use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::time::{Duration, Instant};

use anyhow;
use colored::*;
use tracing::info_span;

use crate::terminal::{colors, format, print, spinner};
use mappr_common::{config::Config, network::host::Host};
use mappr_common::network::range::IpCollection;
use mappr_common::network::target::{self, Target};
use mappr_core::scanner;

pub async fn discover(target: Target, cfg: &Config) -> anyhow::Result<()> {
    let ips: IpCollection = target::to_collection(target)?;
    let span = info_span!("discovery", indicatif.pb_show = true);
    let guard = span.enter();

    let running: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    let spinner_handle = spinner::start_discovery_spinner(span.clone(), running.clone());

    let start_time: Instant = Instant::now();
    let mut hosts: Vec<Host> = scanner::perform_discovery(ips, cfg).await?;

    running.store(false, Ordering::Relaxed);
    let _ = spinner_handle.join();

    drop(guard);

    discovery_ends(&mut hosts, start_time.elapsed())
}

fn discovery_ends(hosts: &mut [Host], total_time: Duration) -> anyhow::Result<()> {
    if hosts.is_empty() {
        no_hosts_found();
        return Ok(());
    }
    print::header("Network Discovery");
    hosts.sort_by_key(|host| *host.ips.iter().next().unwrap_or(&host.ip));
    for (idx, host) in hosts.iter().enumerate() {
        print_host_details(host, idx);
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
    Ok(())
}

fn no_hosts_found() {
    print::header("ZERO HOSTS DETECTED");
    print::no_results();
}

fn print_host_details(host: &Host, idx: usize) {
    let hostname = host.hostname.as_deref().unwrap_or("No hostname");
    print::tree_head(idx, hostname);
    let mut key_value_pair = format::ip_to_key_value_pair(&host.ips);

    if let Some(mac) = host.mac {
        let mac_key_value: (String, ColoredString) =
            ("MAC".to_string(), mac.to_string().color(colors::MAC_ADDR));
        key_value_pair.push(mac_key_value);
    }

    if let Some(vendor) = &host.vendor {
        let vendor_key_value: (String, ColoredString) = (
            "Vendor".to_string(),
            vendor.to_string().color(colors::MAC_ADDR),
        );
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
