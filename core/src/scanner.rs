//! The central **abstraction** for network scanning operations.
//!
//! This module defines the unified interface that specific scanning strategies (such as
//! the [`local`] scanner) must implement. It standardizes the lifecycle of
//! network probes, including packet construction, transmission, and response handling.

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
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
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::scanner::resolver::HostnameResolver;

pub static FOUND_HOST_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);

pub fn increment_host_count() { FOUND_HOST_COUNT.fetch_add(1, Ordering::Relaxed); }
pub fn get_host_count() -> usize { FOUND_HOST_COUNT.load(Ordering::Relaxed) }

#[async_trait]
trait NetworkExplorer {
    async fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>>;
}

pub async fn perform_discovery(targets: IpCollection) -> anyhow::Result<Vec<Host>> {
    spawn_user_input_listener();

    if !is_root() {
        warn!("Root privileges missing, defaulting to unprivileged TCP scan");
        return handshake::range_discovery(targets, handshake::prober).await;
    }
    info!("Root privileges detected, raw socket scan enabled");

    let (dns_tx, dns_rx) = mpsc::unbounded_channel::<IpAddr>();
    let resolver_task = spawn_resolver(dns_rx).await;
    let scanner_handles = spawn_explorers(targets, dns_tx).await;

    let mut hosts = Vec::new();
    for handle in scanner_handles {
        match handle.await {
            Ok(Ok(res)) => hosts.extend(res),
            Ok(Err(e)) => error!("Scanner task failed: {e}"),
            Err(e) => error!("Task panicked: {e}"),
        }
    }

    if let Ok(Some(mut resolver)) = resolver_task.await {
        resolver.resolve_hosts(&mut hosts);
    }

    Ok(hosts)
}

async fn spawn_explorers(
    targets: IpCollection, 
    dns_tx: mpsc::UnboundedSender<IpAddr>
) -> Vec<JoinHandle<anyhow::Result<Vec<Host>>>> {
    let mut handles = Vec::new();
    
    for (intf, (local_ips, routed_ips)) in interface::map_ips_to_interfaces(targets) {
        
        // Local Scanner (ARP/ICMP)
        if !local_ips.is_empty() {
            let tx = dns_tx.clone();
            let intf_c = intf.clone();
            
            let handle = tokio::spawn(async move {
                let mut scanner = LocalScanner::new(intf_c, local_ips, tx)?;
                scanner.discover_hosts().await 
            });
            handles.push(handle);
        }

        // Routed Scanner (TCP Syn Scan)
        if !routed_ips.is_empty() {
            let tx = dns_tx.clone();
            let intf_c = intf.clone();
            
            let handle = tokio::spawn(async move {
                let mut scanner = RoutedScanner::new(intf_c, routed_ips, tx)?;
                scanner.discover_hosts().await
            });
            handles.push(handle);
        }
    }
    handles
}

async fn spawn_resolver(dns_rx: UnboundedReceiver<IpAddr>) -> JoinHandle<Option<HostnameResolver>> {
    tokio::spawn(async move {
        match HostnameResolver::new(dns_rx) {
            Ok(resolver) => {
                info!("Successfully initialized hostname resolver");    
                Some(resolver.run().await)
            },
            Err(e) => {
                error!("Resolver failed to start: {e}");
                None
            }
        }
    })
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