//! The central **abstraction** for network scanning operations.
//!
//! This module defines the unified interface that specific scanning strategies (such as
//! the [`local`] scanner) must implement. It standardizes the lifecycle of
//! network probes, including packet construction, transmission, and response handling.

use std::collections::HashMap;

use async_trait::async_trait;
use mappr_common::network::host::Host;
use mappr_common::network::range::IpCollection;
use pnet::datalink::NetworkInterface;

mod local;

use local::LocalScanner;

pub trait NetworkExplorer: Scout + Identifier {}

pub trait Scanner {
    fn scan(&mut self) -> Vec<Host>;
}

#[async_trait]
trait _Prober {}

pub trait Scout: Scanner {
    fn send_discovery_packets(&mut self) -> anyhow::Result<()>;
}

pub trait Identifier: Scanner { }

pub fn perform_discovery(
    targets: HashMap<NetworkInterface, IpCollection>,
    on_host_found: Option<Box<dyn Fn(usize) + Send + Sync>>,
) -> anyhow::Result<Vec<Host>> {
    let mut handles = Vec::new();
    let global_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let callback = on_host_found.map(|cb| std::sync::Arc::from(cb));

    for (intf, collection) in targets {
        let count_ref = global_count.clone();
        let cb_ref = callback.clone();

        let handle = std::thread::spawn(move || -> anyhow::Result<Vec<Host>> {
            let mut scanner: LocalScanner = LocalScanner::new(intf.clone(), collection, Some(count_ref), cb_ref)?;
            Ok(scanner.scan())
        });
        handles.push(handle);
    }

    let mut hosts = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(Ok(res)) => hosts.extend(res),
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(anyhow::anyhow!("Thread panicked")),
        }
    }

    Ok(hosts)
}
