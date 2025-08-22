use std::net::Ipv4Addr;
use pnet::datalink::{interfaces, NetworkInterface};
use crate::cmd::Target;
use std::path::Path;
use pnet::ipnetwork::{IpNetwork, Ipv4Network};

pub fn select(target: Target) -> NetworkInterface {
    match target {
        Target::LAN => select_lan(),
        Target::Host { addr } => select_host(addr).unwrap(),
        _ => { select_lan() }
    }
}

pub fn get_ipv4(interface: &NetworkInterface) -> Result<Ipv4Addr, String> {
    first_ipv4_net(interface).map(|net| net.ip())
}

pub fn get_prefix(interface: &NetworkInterface) -> Result<u8, String> {
    first_ipv4_net(interface).map(|net| net.prefix())
}

fn select_lan() -> NetworkInterface {
    let interfaces = interfaces();
    let candidates: Vec<_> = interfaces
        .iter()
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
    wired_over_wireless(candidates)
}

fn select_host(addr: Ipv4Addr) -> Option<NetworkInterface> {
    match addr.octets()[0] {
        10 | 172 | 192 => Some(select_lan()),
        _ => None
    }
}

fn wired_over_wireless(mut candidates: Vec<&NetworkInterface>) -> NetworkInterface {
    candidates.sort_by_key(|k| is_wireless(k));
    candidates
        .first()
        .cloned()
        .cloned()
        .expect("no suitable network interfaces found")
}

fn first_ipv4_net(interface: &NetworkInterface) -> Result<Ipv4Network, String> {
    if let Some(ip_net) = interface.ips.first() {
        if let IpNetwork::V4(v4_net) = ip_net {
            Ok(*v4_net)
        } else {
            Err("Interface does not have an IPv4 address".into())
        }
    } else {
        Err("Interface has no IP address at all".into())
    }
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
    Path::new(&format!("sys/class/net/{}/wireless", interface)).exists()
}