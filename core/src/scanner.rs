//! The central **abstraction** for network scanning operations.
//!
//! This module defines the unified interface that specific scanning strategies (such as
//! the [`local`] scanner) must implement. It standardizes the lifecycle of
//! network probes, including packet construction, transmission, and response handling.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use mappr_common::network::host::Host;
use mappr_common::network::range::IpCollection;
use pnet::datalink::NetworkInterface;

mod local;

use local::LocalScanner;

pub static FOUND_HOST_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn increment_host_count() {
    FOUND_HOST_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn get_host_count() -> usize {
    FOUND_HOST_COUNT.load(Ordering::Relaxed)
}

pub trait NetworkExplorer {
   fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>>;
}

pub fn perform_discovery(
    targets: HashMap<NetworkInterface, IpCollection>,
) -> anyhow::Result<Vec<Host>> {
    let mut handles = Vec::new();

    for (intf, collection) in targets {
        let handle = std::thread::spawn(move || -> anyhow::Result<Vec<Host>> {
            let mut scanner: Box<dyn NetworkExplorer> = create_explorer(intf, collection)?;
            Ok(scanner.discover_hosts()?)
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

fn create_explorer(intf: NetworkInterface, ip_collection: IpCollection) 
-> anyhow::Result<Box<dyn NetworkExplorer>> 
{
    Ok(Box::new(LocalScanner::new(intf, ip_collection)?))
}