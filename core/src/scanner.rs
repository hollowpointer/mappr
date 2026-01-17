//! The central **abstraction** for network scanning operations.
//!
//! This module defines the unified interface that specific scanning strategies (such as
//! the [`local`] scanner) must implement. It standardizes the lifecycle of
//! network probes, including packet construction, transmission, and response handling.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use is_root::is_root;
use mappr_common::network::host::Host;
use mappr_common::network::interface;
use mappr_common::network::range::IpCollection;
use mappr_common::utils::input::InputHandle;

mod local;
mod routed;
mod handshake;

use local::LocalScanner;
use tracing::{info, warn};

use crate::scanner::routed::RoutedScanner;

pub static FOUND_HOST_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);

pub fn increment_host_count() {
    FOUND_HOST_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn get_host_count() -> usize {
    FOUND_HOST_COUNT.load(Ordering::Relaxed)
}

trait NetworkExplorer {
   fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>>;
}

macro_rules! spawn_scanner {
    ($intf:expr, $ips:expr, $handles:expr, $scanner:ident) => {
        let scan_intf = $intf.clone();
        let handle = std::thread::spawn(move || -> anyhow::Result<Vec<Host>> {
            let mut scanner = $scanner::new(scan_intf, $ips)?;
            scanner.discover_hosts()
        }); 
        $handles.push(handle);
    };
}

pub async fn perform_discovery(
    targets: IpCollection,
) -> anyhow::Result<Vec<Host>> {

    // Listens for user inputs mid scan
    std::thread::spawn(|| {
        let mut input_handle: InputHandle = InputHandle::new();
        input_handle.start();
        loop {
            if input_handle.should_interrupt() {
                STOP_SIGNAL.store(true, Ordering::Relaxed);
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });

    // Non root users can only scan via full tcp handshakes
    if !is_root() {
        warn!("Root privileges missing, defaulting to unprivileged TCP scan");
        return handshake::range_discovery(targets, handshake::prober).await;
    }

    info!("Root privileges detected, raw socket scan enabled");

    let mut handles = Vec::new();
    let mut hosts: Vec<Host> = Vec::new();

    for (intf, (local_ips, routed_ips)) in interface::map_ips_to_interfaces(targets) {
        // Local Scanner (ARP/ICMP)
        if !local_ips.is_empty() {
            spawn_scanner!(intf, local_ips, handles, LocalScanner);
        }

        // Routed Scanner (Syn Scan via Gateway)
        if !routed_ips.is_empty() {
            spawn_scanner!(intf, routed_ips, handles, RoutedScanner);
        }
    }

    for handle in handles {
        match handle.join() {
            Ok(Ok(res)) => hosts.extend(res),
            Ok(Err(e)) => warn!("Scanner thread failed: {}", e),
            Err(_) => anyhow::bail!("Thread panicked"),
        }
    }

    Ok(hosts)
}