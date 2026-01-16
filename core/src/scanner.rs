//! The central **abstraction** for network scanning operations.
//!
//! This module defines the unified interface that specific scanning strategies (such as
//! the [`local`] scanner) must implement. It standardizes the lifecycle of
//! network probes, including packet construction, transmission, and response handling.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use is_root::is_root;
use mappr_common::network::host::Host;
use mappr_common::network::interface;
use mappr_common::network::range::IpCollection;
use mappr_common::utils::input::InputHandle;
use pnet::datalink::NetworkInterface;

mod local;
mod routed;
mod handshake;

use local::LocalScanner;

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

pub async fn perform_discovery(
    targets: IpCollection,
) -> anyhow::Result<Vec<Host>> {

    // Thread handles user inputs mid scan
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

    // Non root scans can only do a full tcp connection scan
    if !is_root() {
        return handshake::range_discovery(targets, handshake::prober).await;
    }

    let mut handles = Vec::new();
    let mut hosts: Vec<Host> = Vec::new();

    let intf_ip_map: HashMap<NetworkInterface, IpCollection> =
        interface::map_ips_to_interfaces(targets);

    for (intf, collection) in intf_ip_map {
        let handle = std::thread::spawn(move || -> anyhow::Result<Vec<Host>> {
            let mut scanner: Box<dyn NetworkExplorer> = create_explorer(intf, collection)?;
            scanner.discover_hosts()
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.join() {
            Ok(Ok(res)) => hosts.extend(res),
            Ok(Err(e)) => return Err(e),
            Err(_) => anyhow::bail!("Thread panicked"),
        }
    }

    Ok(hosts)
}

fn create_explorer(intf: NetworkInterface, ips: IpCollection) 
-> anyhow::Result<Box<dyn NetworkExplorer>> 
{
    match interface::is_layer_2_capable(&intf) && interface::is_on_link(&intf, &ips) {
        true => Ok(Box::new(LocalScanner::new(intf, ips)?)),
        false => Ok(Box::new(RoutedScanner::new(ips)?)),
    }
}
