//! The central **abstraction** for network scanning operations.
//!
//! This module defines the unified interface that specific scanning strategies (such as
//! the [`local`] scanner) must implement. It standardizes the lifecycle of
//! network probes, including packet construction, transmission, and response handling.
//!
//! **Architectural Note:**
//! High-level modules should strictly depend on this abstraction rather than concrete
//! submodules. This pattern ensures **decoupling**, allowing this module to orchestrate
//! the logic and dynamically dispatch requests to the appropriate underlying scanning technique.

use std::collections::HashMap;

use async_trait::async_trait;
use mappr_common::network::host::Host;
use mappr_common::network::range::IpCollection;
use pnet::datalink::NetworkInterface;
use tracing::error;

mod local;

/// Discovery super trait,
pub trait NetworkExplorer: Scanner + Scout + Identifier {}

/// Defines the lifecycle and state-retrieval logic for a network scan.
pub trait Scanner {
    /// Initializes the low-level listener (e.g., opening a raw socket or a pcap handle).
    ///
    /// This should be called before sending any packets to ensure responses
    /// are not missed due to race conditions.
    /// Initializes the low-level listener (e.g., opening a raw socket or a pcap handle).
    ///
    /// This should be called before sending any packets to ensure responses
    /// are not missed due to race conditions.
    fn start_listening(&mut self);

    /// Gracefully terminates the scan and consumes the scanner to return the discovered results.
    ///
    /// This method is responsible for cleaning up resources, closing sockets, and
    /// aggregating all captured host data into a final vector.
    fn finish(self) -> Vec<Host>;
}

/// Defines the strategy for probing transport-layer ports to identify active services.
#[async_trait]
trait _Prober {}

/// A "scout" is responsible for sending discovery packets to potential hosts.
pub trait Scout {
    fn send_discovery_packets(&mut self) -> anyhow::Result<()>;
}

/// An "identifier" is responsible for resolving host details (hostname, vendor, etc.).
pub trait Identifier {}

/// Executes a full network discovery cycle against the specified targets.
use local::LocalScanner;

/// Executes a full network discovery cycle against the specified targets.
pub fn perform_discovery(
    targets: HashMap<NetworkInterface, IpCollection>,
    on_host_found: Option<Box<dyn Fn(usize) + Send + Sync>>,
) -> Vec<Host> {
    let mut handles = Vec::new();
    let global_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let callback = on_host_found.map(|cb| std::sync::Arc::from(cb));

    for (intf, collection) in targets {
        let count_ref = global_count.clone();
        let cb_ref = callback.clone();

        let handle = std::thread::spawn(move || -> Vec<Host> {
            match LocalScanner::new(intf.clone(), collection, Some(count_ref), cb_ref) {
                Ok(mut scanner) => {
                    scanner.start_listening();
                    if let Err(e) = scanner.send_discovery_packets() {
                        error!("Failed to send discovery packets: {}", e);
                    }
                    scanner.finish()
                }
                Err(e) => {
                    error!(
                        "Failed to initialize scanner for interface {}: {}",
                        intf.name, e
                    );
                    Vec::new()
                }
            }
        });
        handles.push(handle);
    }

    let mut hosts = Vec::new();
    for handle in handles {
        if let Ok(res) = handle.join() {
            hosts.extend(res);
        }
    }
    hosts
}
