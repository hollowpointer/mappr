//! The central **abstraction** for network scanning operations.
//!
//! This module defines the unified interface that specific scanning strategies (such as
//! the [`local`] scanner) must implement. It standardizes the lifecycle of
//! network probes, including packet construction, transmission, and response handling.

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use is_root::is_root;
use mappr_common::network::host::Host;
use mappr_common::network::interface;
use mappr_common::network::range::IpCollection;
use mappr_common::utils::input::InputHandle;

mod handshake;
mod local;
mod resolver;
mod routed;

use local::LocalScanner;
use routed::RoutedScanner;
use tracing::{error, info, warn};

use crate::scanner::resolver::HostnameResolver;

pub static FOUND_HOST_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);

pub fn increment_host_count() { FOUND_HOST_COUNT.fetch_add(1, Ordering::Relaxed); }
pub fn get_host_count() -> usize { FOUND_HOST_COUNT.load(Ordering::Relaxed) }

trait NetworkExplorer {
    fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>>;
}

pub async fn perform_discovery(targets: IpCollection) -> anyhow::Result<Vec<Host>> {
    spawn_user_input_listener();

    if !is_root() {
        warn!("Root privileges missing, defaulting to unprivileged TCP scan");
        return handshake::range_discovery(targets, handshake::prober).await;
    }
    info!("Root privileges detected, raw socket scan enabled");

    let (dns_tx, dns_rx) = mpsc::channel();
    let resolver = spawn_resolver(dns_rx);
    let handles = spawn_explorers(targets, &dns_tx);
    drop(dns_tx);

    let mut hosts: Vec<Host> = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(Ok(res)) => hosts.extend(res),
            Ok(Err(e)) => error!("Scanner thread failed: {}", e),
            Err(_) => anyhow::bail!("Thread panicked"),
        }
    }
    
    if let Some(handle) = resolver {
        match handle.join() {
            Ok(mut r) => r.resolve_hosts(&mut hosts),
            Err(e) => error!("Failed to join resolver: {:?}", e),
        }
    }

    Ok(hosts)
}

fn spawn_explorers(targets: IpCollection, dns_tx: &Sender<IpAddr>) 
-> Vec<JoinHandle<Result<Vec<Host>, anyhow::Error>>> {
    let mut handles = Vec::new();
    
    for (intf, (local_ips, routed_ips)) in interface::map_ips_to_interfaces(targets) {
        // Local Scanner (ARP/ICMP)
        if !local_ips.is_empty() {
            let intf_c = intf.clone();
            let dns_tx_local = dns_tx.clone(); 
            let handle = std::thread::spawn(move || {
                let mut scanner = LocalScanner::new(intf_c, local_ips, dns_tx_local)?;
                scanner.discover_hosts()
            });
            handles.push(handle);
        }

        // Routed Scanner (Syn Scan via Gateway)
        if !routed_ips.is_empty() {
            let dns_tx_routed = dns_tx.clone();
            let handle = std::thread::spawn(move || {
                let mut scanner = RoutedScanner::new(intf, routed_ips, dns_tx_routed)?;
                scanner.discover_hosts()
            });
            handles.push(handle);
        }
    }
    handles
}

fn spawn_resolver(dns_rx: Receiver<IpAddr>) -> Option<JoinHandle<HostnameResolver>> {
    match HostnameResolver::new(dns_rx) {
        Ok(resolver) => {
            info!("Successfully initialized hostname resolver");
            Some(resolver.spawn())
        }
        Err(e) => {
            error!("Critical failure during hostname resolver startup: {e}");
            None
        }
    }
}

fn spawn_user_input_listener() {
    std::thread::spawn(|| {
        let mut input_handle = InputHandle::new();
        input_handle.start();
        loop {
            if input_handle.should_interrupt() {
                STOP_SIGNAL.store(true, Ordering::Relaxed);
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
}