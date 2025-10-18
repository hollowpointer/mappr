use std::{net::{Ipv4Addr, Ipv6Addr}, vec::IntoIter};
use pnet::datalink::{interfaces, NetworkInterface};
use crate::{cmd::Target};
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use anyhow::{anyhow, Ok};
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

pub fn get_ipv4(interface: &NetworkInterface) -> anyhow::Result<Option<Ipv4Addr>> {
    if let Some(ipv4net) = first_ipv4_net(interface)? {
        Ok(Some(ipv4net.ip()))
    } else { Ok(None) }
}

pub fn get_ipv6(interface: &NetworkInterface) -> anyhow::Result<Option<Ipv6Addr>> {
    if let Some(ipv6net) = first_ipv6_net(interface)? {
        Ok(Some(ipv6net.ip()))
    } else { Ok(None) }
}

pub fn get_unique_interfaces(max: usize) -> anyhow::Result<Vec<NetworkInterface>> {
    let interfaces: Vec<NetworkInterface> = interfaces();
    let mut unique_interfaces: Vec<NetworkInterface> = Vec::with_capacity(max);
    let mut wired_iter: IntoIter<NetworkInterface> = get_all_wired(&interfaces)?.into_iter();
    let mut wireless_iter: IntoIter<NetworkInterface> = get_all_wireless(&interfaces)?.into_iter();
    let mut tunnel_iter: IntoIter<NetworkInterface> = get_all_tunnel(&interfaces)?.into_iter();

    while unique_interfaces.len() < max {
        let mut items_added_this_pass = 0;
        if let Some(iface) = wired_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass += 1;
            if unique_interfaces.len() == max { break; }
        }
        if let Some(iface) = wireless_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass += 1;
            if unique_interfaces.len() == max { break; }
        }
        if let Some(iface) = tunnel_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass += 1;
            if unique_interfaces.len() == max { break; }
        }
        if items_added_this_pass == 0 {
            break;
        }
    }
    Ok(unique_interfaces)
}

pub fn get_link_local_addr(interface: &NetworkInterface) -> Option<Ipv6Addr> {
    interface.ips.iter().find_map(|ip_network| {
        match ip_network {
            // Get the specific IP address from the Ipv6Network using .ip()
            IpNetwork::V6(ipv6_network) => {
                let addr = ipv6_network.ip();
                // Check if that one address is link-local
                if addr.is_unicast_link_local() {
                    Some(addr)
                } else {
                    None
                }
            }
            // Ignore IPv4 addresses
            IpNetwork::V4(_) => None,
        }
    })
}

pub fn get_prefix(interface: &NetworkInterface) -> anyhow::Result<Option<u8>> {
    if let Some(ipv4net) = first_ipv4_net(interface)? {
        Ok(Some(ipv4net.prefix()))
    } else { Ok(None) }
}

fn get_all_wired(interfaces: &[NetworkInterface]) -> anyhow::Result<Vec<NetworkInterface>> {
    let mut res = Vec::with_capacity(interfaces.len());
    for i in interfaces {
        if is_physical(i)? && !is_wireless(i)? {
            res.push(i.clone());
        }
    }
    Ok(res)
}

fn get_all_wireless(interfaces: &[NetworkInterface]) -> anyhow::Result<Vec<NetworkInterface>> {
    let mut res = Vec::with_capacity(interfaces.len());
    for i in interfaces {
        if is_physical(i)? && is_wireless(i)? {
            res.push(i.clone());
        }
    }
    Ok(res)
}

// info: It will miss TAP-style tunnels since they are not point-to-point, needs refinement
fn get_all_tunnel(interfaces: &Vec<NetworkInterface>) -> anyhow::Result<Vec<NetworkInterface>> {
    let mut res = Vec::with_capacity(interfaces.len());
    for i in interfaces {
        if !is_physical(i)? && i.is_point_to_point() {
            res.push(i.clone());
        }
    }
    Ok(res)
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
                && is_physical(i).unwrap_or(false)
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
    if let Some(ipv4) = get_ipv4(&intf).expect("retrieving ipv4 of interface") {
        print::print_status(format!("Selected {} with address {}", intf.name, ipv4).as_str());
    }
    intf
}

fn select_host(addr: Ipv4Addr) -> Option<NetworkInterface> {
    match addr.octets()[0] {
        10 | 172 | 192 => Some(select_lan()),
        _ => None
    }
}

fn wired_over_wireless(mut candidates: Vec<NetworkInterface>) -> NetworkInterface {
    candidates.sort_by_key(|k| is_wireless(k).unwrap_or(false));
    let intf = candidates.first().unwrap().clone();
    if let Some(ipv4) = get_ipv4(&intf).expect("retrieving ipv4 of interface") {
        print::print_status(format!("Selected {} with address {}", intf.name, ipv4).as_str());
    }    
    candidates
        .first()
        .cloned()
        .expect("no suitable network interfaces found")
}

fn first_ipv4_net(interface: &NetworkInterface) -> anyhow::Result<Option<Ipv4Network>> {
    let ipv4 = interface.ips.iter().find_map(|ip| match ip {
        IpNetwork::V4(n) => Some(*n),
        _ => None,
    }).ok_or_else(|| anyhow!("Interface does not have an IPv4 address"))?;
    Ok(Some(ipv4))
}

fn first_ipv6_net(interface: &NetworkInterface) -> anyhow::Result<Option<Ipv6Network>> {
    let ipv6 = interface.ips.iter().find_map(|ip| match ip {
        IpNetwork::V6(n) => Some(*n),
        _ => None,
    }).ok_or_else(|| anyhow!("Interface does not have an IPv6 address"))?;
    Ok(Some(ipv6))
}



/*********************************
OS dependent functions for PHYSICAL
**********************************/
#[cfg(target_os = "linux")]
fn is_physical(interface: &NetworkInterface) -> anyhow::Result<bool> {
    Ok(Path::new(&format!("/sys/class/net/{}/device", interface.name)).exists())
}

#[cfg(target_os = "macos")]
fn is_physical(interface: &NetworkInterface) -> anyhow::Result<bool> {
    let output = Command::new("networksetup")
        .arg("-listallhardwareports")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("'networksetup' command failed: {}", stderr);
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let expected = format!("Device: {}", interface.name);
    Ok(stdout_str.contains(&expected))
}

#[cfg(target_os = "windows")]
fn is_physical(interface: &NetworkInterface) -> bool {
    true
}

/*********************************
OS dependent functions for WIRELESS
**********************************/
#[cfg(target_os = "linux")]
fn is_wireless(interface: &NetworkInterface) -> anyhow::Result<bool> {
    Ok(Path::new(&format!("sys/class/net/{}/wireless", interface.name)).exists())
}

#[cfg(target_os = "macos")]
fn is_wireless(interface: &NetworkInterface) -> anyhow::Result<bool> {
    let output = Command::new("networksetup")
        .arg("-getairportnetwork")
        .arg(&interface.name)
        .output()?;

    Ok(output.status.success())
}