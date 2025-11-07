use std::{net::{IpAddr, Ipv4Addr, Ipv6Addr}, vec::IntoIter};
use colored::{ColoredString, Colorize};
use pnet::{datalink::{NetworkInterface, interfaces}, util::MacAddr};
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use anyhow::{anyhow, Ok};
use pnet::ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use crate::{GLOBAL_KEY_WIDTH, cmd::Target, net::ip::{self, Ipv6AddressType}, utils::colors};
use crate::print;
use crate::net::ip::IpWithPrefix;

#[derive(Debug, Default, Clone, Copy)]
pub enum InterfaceType {
    Loopback,
    Wired,
    Wireless,
    Tunnel,
    Bridge,
    #[default]
    Unknown
}

#[derive(Clone)]
pub struct Interface {
    pub name: String,
    pub mac_addr: Option<MacAddr>,
    pub ipv4_addr: Vec<IpWithPrefix>,
    pub ipv6_addr: Vec<IpWithPrefix>,
    pub interface_type: InterfaceType
}

impl Default for Interface {
    fn default() -> Self { 
        Self { 
            name: String::new(),
            mac_addr: None,
            ipv4_addr: Vec::new(),
            ipv6_addr: Vec::new(), 
            interface_type: InterfaceType::default() 
        }
    }
}

impl Interface {
    pub fn print(self: &Self, idx: usize) {
        print::println(format!("{} {}", format!("[{}]", idx.to_string().color(colors::ACCENT))
            .color(colors::SEPARATOR), self.name.color(colors::PRIMARY)).as_str());

        let mut lines: Vec<(ColoredString, ColoredString)> = Vec::new();

        for ipv4_addr in &self.ipv4_addr {
            let address: ColoredString = ipv4_addr.ip_addr.to_string().color(colors::IPV4_ADDR);
            let prefix: ColoredString = ipv4_addr.prefix.to_string().color(colors::IPV4_PREFIX);
            let result: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR); 
            lines.push(("IPv4".color(colors::TEXT_DEFAULT), result));
        }

        for ipv6_addr in &self.ipv6_addr {
            let address: ColoredString = ipv6_addr.ip_addr.to_string().color(colors::IPV6_ADDR);
            let prefix: ColoredString = ipv6_addr.prefix.to_string().color(colors::IPV6_PREFIX);
            let result: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR);
            let ipv6_type = match ipv6_addr.ip_addr {
                IpAddr::V4(_) => panic!("This should never panic."),
                IpAddr::V6(ipv6_addr) => ip::get_ipv6_type(&ipv6_addr),
            };
            let key = match ipv6_type {
                Ipv6AddressType::GlobalUnicast  => "GUA",
                Ipv6AddressType::LinkLocal      => "LLA",
                Ipv6AddressType::UniqueLocal    => "ULA",
                _                               => "IPv6"
            };
            lines.push((key.color(colors::TEXT_DEFAULT), result));
        }

        if let Some(mac_addr) = self.mac_addr {
            lines.push(("MAC".color(colors::TEXT_DEFAULT), mac_addr.to_string().color(colors::MAC_ADDR)));
        }
        
        for(i, (key, value)) in lines.iter().enumerate() {
            let last = i + 1 == lines.len();
            let branch = if last { "└─".color(colors::SEPARATOR) } else { "├─".color(colors::SEPARATOR) };
            let dots = ".".repeat(GLOBAL_KEY_WIDTH.get() - key.len() - 1);
            let colon = format!("{}{}", dots.color(colors::SEPARATOR), ":".color(colors::SEPARATOR));
            let output = format!(" {branch} {}{} {}", key, colon, value);
            print::println(&output)
        }
    }
}

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

pub fn get_prefix(interface: &NetworkInterface) -> anyhow::Result<Option<u8>> {
    if let Some(ipv4net) = first_ipv4_net(interface)? {
        Ok(Some(ipv4net.prefix()))
    } else { Ok(None) }
}

pub fn get_unique_interfaces(max: usize) -> anyhow::Result<Vec<Interface>> {
    let interfaces: Vec<Interface> = identify_interfaces(interfaces())?;
    let mut unique_interfaces: Vec<Interface> = Vec::with_capacity(max);
    let mut loopback_interfaces: Vec<Interface> = Vec::new();
    let mut wired_interfaces: Vec<Interface> = Vec::new();
    let mut wireless_interfaces: Vec<Interface> = Vec::new();
    let mut tunnel_interfaces: Vec<Interface> = Vec::new();
    let mut bridge_interfaces: Vec<Interface> = Vec::new();
    for interface in interfaces {
        match interface.interface_type {
            InterfaceType::Loopback => loopback_interfaces.push(interface),
            InterfaceType::Wired    => wired_interfaces.push(interface),
            InterfaceType::Wireless => wireless_interfaces.push(interface),
            InterfaceType::Tunnel   => tunnel_interfaces.push(interface),
            InterfaceType::Bridge   => bridge_interfaces.push(interface),
            _                       => continue
        }
    }
    let mut loopback_iter: IntoIter<Interface> = loopback_interfaces.into_iter();
    let mut wired_iter: IntoIter<Interface> = wired_interfaces.into_iter();
    let mut wireless_iter: IntoIter<Interface> = wireless_interfaces.into_iter();
    let mut tunnel_iter: IntoIter<Interface> = tunnel_interfaces.into_iter();
    let mut bridge_iter: IntoIter<Interface> = bridge_interfaces.into_iter();
    while unique_interfaces.len() < max {
        let mut items_added_this_pass: bool = false;
        if let Some(iface) = loopback_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass = true;
            if unique_interfaces.len() == max { break; }
        }
        if let Some(iface) = wired_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass = true;
            if unique_interfaces.len() == max { break; }
        }
        if let Some(iface) = wireless_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass = true;
            if unique_interfaces.len() == max { break; }
        }
        if let Some(iface) = tunnel_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass = true;
            if unique_interfaces.len() == max { break; }
        }
        if let Some(iface) = bridge_iter.next() {
            unique_interfaces.push(iface);
            items_added_this_pass = true;
            if unique_interfaces.len() == max { break; }
        }
        if !items_added_this_pass {
            break;
        }
    }
    Ok(unique_interfaces)
}

fn identify_interfaces(interfaces: Vec<NetworkInterface>) -> anyhow::Result<Vec<Interface>> {
    let mut identified_interfaces: Vec<Interface> = Vec::with_capacity(interfaces.len());
    for interface in interfaces {
        let mut new_interface: Interface = Interface::default();
        new_interface.name = interface.name.clone();
        if let Some(mac_addr) = interface.mac { new_interface.mac_addr = Some(mac_addr) }
        new_interface.interface_type = match true {
            _ if interface.is_loopback()    => InterfaceType::Loopback,
            _ if is_wired(&interface)?      => InterfaceType::Wired,
            _ if is_wireless(&interface)?   => InterfaceType::Wireless,
            _ if is_tunnel(&interface)?     => InterfaceType::Tunnel,
            _ if is_bridge(&interface)?     => InterfaceType::Bridge,
            _                               => InterfaceType::Unknown,
        };
        for ip_network in interface.ips {
            match ip_network {
                IpNetwork::V4(ipv4) => { 
                    let ip_addr: IpWithPrefix = IpWithPrefix::new(IpAddr::V4(ipv4.ip()), ipv4.prefix());
                    new_interface.ipv4_addr.push(ip_addr);
                },
                IpNetwork::V6(ipv6) => {
                    let ip_addr: IpWithPrefix = IpWithPrefix::new(IpAddr::V6(ipv6.ip()), ipv6.prefix());
                    new_interface.ipv6_addr.push(ip_addr);
                }
            }
        }
        identified_interfaces.push(new_interface);
    }
    Ok(identified_interfaces)
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

fn is_wired(interface: &NetworkInterface) -> anyhow::Result<bool> {
    Ok(is_physical(interface)? && !is_wireless(interface)?)
}

// this check is shit and needs improvement
fn is_tunnel(interface: &NetworkInterface) -> anyhow::Result<bool> {
    if is_physical(interface)? || interface.is_loopback() {
        return Ok(false);
    }
    let name = &interface.name;
    Ok(name.contains("tun") ||
       name.contains("tap") ||
       name.contains("gre") ||
       name.contains("ipip") ||
       name.contains("sit") ||
       name.contains("vti"))
}

// this one is shit too as you might tell
fn is_bridge(interface: &NetworkInterface) -> anyhow::Result<bool> {
    Ok(!is_physical(interface)? && !interface.is_loopback() && !is_tunnel(interface)?)
}

/***************************************
   OS dependent functions for PHYSICAL
****************************************/
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

/***************************************
   OS dependent functions for WIRELESS
****************************************/
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