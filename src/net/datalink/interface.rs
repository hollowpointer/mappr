use std::net::{Ipv4Addr, Ipv6Addr};
use pnet::datalink::{interfaces, NetworkInterface};
use crate::cmd::Target;
use std::path::Path;
use anyhow::anyhow;
use pnet::ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use crate::print;

pub fn select(target: Target) -> NetworkInterface {
    print::print_status("Searching for a suitable network interface...");
    match target {
        Target::LAN => select_lan(),
        Target::Host { addr } => select_host(addr).unwrap(),
        _ => { select_lan() }
    }
}

pub fn get_ipv4(interface: &NetworkInterface) -> anyhow::Result<Ipv4Addr> {
    first_ipv4_net(interface).map(|net| net.ip())
}

pub fn get_ipv6(interface: &NetworkInterface) -> anyhow::Result<Ipv6Addr> {
    first_ipv6_net(interface).map(|net| net.ip())
}

pub fn get_prefix(interface: &NetworkInterface) -> anyhow::Result<u8> {
    first_ipv4_net(interface).map(|net| net.prefix())
}

fn select_lan() -> NetworkInterface {
    let interfaces = interfaces();
    let msg = format!("Identified {} network interface(s)", interfaces.len());
    print::print_status(&msg);
    let candidates: Vec<_> = interfaces
        .into_iter()
        .filter(|i| {
            i.is_up()
                && i.mac.is_some()
                && i.is_broadcast()
                && is_physical(i)
                && !i.is_loopback()
                && !i.is_point_to_point()
                && i.ips.iter().any(|ip| ip.is_ipv4())
        })
        .collect();
    if candidates.len() > 1 {
        print::print_status("More than one candidate found, selecting best option...");
        return wired_over_wireless(candidates);
    }
    let intf = candidates.first().unwrap().clone();
    let msg = format!("Selected {} with address {}", intf.name, get_ipv6(&intf).unwrap());
    print::print_status(&msg);
    intf
}

fn select_host(addr: Ipv4Addr) -> Option<NetworkInterface> {
    match addr.octets()[0] {
        10 | 172 | 192 => Some(select_lan()),
        _ => None
    }
}

fn wired_over_wireless(mut candidates: Vec<NetworkInterface>) -> NetworkInterface {
    candidates.sort_by_key(|k| is_wireless(k));
    let intf = candidates.first().unwrap().clone();
    let msg = format!("Selected {} with address {}", intf.name, get_ipv4(&intf).unwrap());
    print::print_status(&msg);
    candidates
        .first()
        .cloned()
        .expect("no suitable network interfaces found")
}

fn first_ipv4_net(interface: &NetworkInterface) -> anyhow::Result<Ipv4Network> {
    interface.ips.iter().find_map(|ip| match ip {
        IpNetwork::V4(n) => Some(*n),
        _ => None,
    }).ok_or_else(|| anyhow!("Interface does not have an IPv4 address"))
}

fn first_ipv6_net(interface: &NetworkInterface) -> anyhow::Result<Ipv6Network> {
    interface.ips.iter().find_map(|ip| match ip {
        IpNetwork::V6(n) => Some(*n),
        _ => None,
    }).ok_or_else(|| anyhow!("Interface does not have an IPv6 address"))
}

/*********************************
OS dependent functions for PHYSICAL
**********************************/
#[cfg(target_os = "linux")]
fn is_physical(interface: &NetworkInterface) -> bool {
    Path::new(&format!("/sys/class/net/{}/device", interface.name)).exists()
}

#[cfg(target_os = "windows")]
fn is_physical(interface: &NetworkInterface) -> bool {
    true
}

#[cfg(target_os = "macos")]
fn is_physical(interface: &NetworkInterface) -> bool {
    true
}

/*********************************
OS dependent functions for WIRELESS
**********************************/
#[cfg(target_os = "linux")]
fn is_wireless(interface: &NetworkInterface) -> bool {
    Path::new(&format!("sys/class/net/{}/wireless", interface.name)).exists()
}