use std::net::Ipv6Addr;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use colored::{ColoredString, Colorize};
use pnet::{self, datalink::NetworkInterface, ipnetwork::Ipv4Network};
use anyhow;
use pnet::ipnetwork::IpNetwork;
use crate::{GLOBAL_KEY_WIDTH, net::{ip::{self, Ipv6AddressType}}, utils::colors};
use crate::print;

pub trait NetworkInterfaceExtension {
    fn print_details(&self, idx: usize);
    fn get_ipv4_net(&self) -> Option<Ipv4Network>;
    fn get_link_local_addr(&self) -> Option<Ipv6Addr>;
}

impl NetworkInterfaceExtension for NetworkInterface {

    fn print_details(self: &Self, idx: usize) {
        print::println(format!("{} {}", format!("[{}]", idx.to_string().color(colors::ACCENT))
            .color(colors::SEPARATOR), self.name.color(colors::PRIMARY)).as_str());

        let mut lines: Vec<(ColoredString, ColoredString)> = Vec::new();

        for ip_network in &self.ips {
            match ip_network {
                IpNetwork::V4(ipv4_network) => {
                    let address: ColoredString = ipv4_network.ip().to_string().color(colors::IPV4_ADDR);
                    let prefix: ColoredString = ipv4_network.prefix().to_string().color(colors::IPV4_PREFIX);
                    let result: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR); 
                    lines.push(("IPv4".color(colors::TEXT_DEFAULT), result));
                },
                IpNetwork::V6(ipv6_network) => {
                    let address: ColoredString = ipv6_network.ip().to_string().color(colors::IPV6_ADDR);
                    let prefix: ColoredString = ipv6_network.prefix().to_string().color(colors::IPV6_PREFIX);
                    let result: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR);
                    let ipv6_type = ip::get_ipv6_type(&ipv6_network.ip());
                    let key = match ipv6_type {
                        Ipv6AddressType::GlobalUnicast  => "GUA",
                        Ipv6AddressType::LinkLocal      => "LLA",
                        Ipv6AddressType::UniqueLocal    => "ULA",
                        _                               => "IPv6"
                    };
                    lines.push((key.color(colors::TEXT_DEFAULT), result));
                },
            }
        }

        if let Some(mac_addr) = self.mac {
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
    
    fn get_ipv4_net(&self) -> Option<Ipv4Network> {
        self.ips.iter().find_map(|&ip| match ip {
            IpNetwork::V4(net) => Some(net),
            _ => None,
        })
    }

    fn get_link_local_addr(&self) -> Option<Ipv6Addr> {
        self.ips.iter().find_map(|ip| match ip {
            IpNetwork::V6(net) if net.ip().is_unicast_link_local() => Some(net.ip()),
            _ => None,
        })
    }

}

pub fn get_prioritized_interfaces(max: usize) -> anyhow::Result<Vec<NetworkInterface>> {
    let interfaces: Vec<NetworkInterface> = pnet::datalink::interfaces();

    let loopback_iter = interfaces
        .iter()
        .filter(|interface| interface.is_loopback());
    let wired_iter = interfaces
        .iter()
        .filter(|interface| is_wired(interface));
    let wireless_iter = interfaces
        .iter()
        .filter(|interface| is_wireless(interface));
    let tunnel_iter = interfaces
        .iter()
        .filter(|interface| is_tunnel(interface));
    let bridge_iter = interfaces
        .iter()
        .filter(|interface| is_bridge(interface));

    let prioritized_iter = loopback_iter
        .chain(wired_iter)
        .chain(wireless_iter)
        .chain(tunnel_iter)
        .chain(bridge_iter);

    let result_interfaces: Vec<NetworkInterface> =
        prioritized_iter.take(max).cloned().collect();

    Ok(result_interfaces)
}

pub fn get_lan() -> NetworkInterface {
    let interfaces: Vec<NetworkInterface> = pnet::datalink::interfaces();
    print::print_status(format!("Identified {} network interface(s)", interfaces.len()).as_str());
    let candidates: Vec<NetworkInterface> = interfaces
        .into_iter()
        .filter(|interface| {
                interface.is_up() &&
                is_physical(interface) &&
                interface.mac.is_some() &&
                interface.is_broadcast() &&
               !interface.is_point_to_point() &&
                interface.ips.iter().any(|net| {
                        net.is_ipv4() ||
                        match net {
                            IpNetwork::V4(_) => { false },
                            IpNetwork::V6(ipv6) => ipv6.ip().is_unicast_link_local(),
                        }
                    }
                )
            }
        )
        .collect();
    let interface = match candidates.len() {
        0 => panic!("No suitable wired LAN interface found."),
        1 => { candidates[0].clone() },
        _ => {
            print::print_status("More than one candidate found, selecting best option...");
            candidates.iter()
                .find(|&interface| is_wired(interface))
                .map(|iface_ref_ref| iface_ref_ref.clone())
                .unwrap_or(candidates[0].clone())
                .clone()
        }
    };
    if let Some(ipv4_net) = interface.get_ipv4_net() {
        let msg: &str = &format!("Selected {} with address {}", interface.name, ipv4_net.ip());
        print::print_status(msg);
    }
    interface
}

fn is_wired(interface: &NetworkInterface) -> bool {
    is_physical(interface) && 
   !is_wireless(interface)
}

// this check is shit and needs improvement
fn is_tunnel(interface: &NetworkInterface) -> bool {
    if is_physical(interface) || interface.is_loopback() { return false; }
    let tunnel_names: Vec<&str> = vec!["tun", "tap", "gre", "ipip", "sit", "vti"];
    tunnel_names.iter().any(|tunnel_name| interface.name.contains(tunnel_name))
}

// this one is shit too as you might tell
fn is_bridge(interface: &NetworkInterface) -> bool {
    !is_physical(interface) && !interface.is_loopback() && !is_tunnel(interface)
}

/***************************************
   OS dependent functions for PHYSICAL
****************************************/
#[cfg(target_os = "linux")]
fn is_physical(interface: &NetworkInterface) -> bool {
    Path::new(&format!("/sys/class/net/{}/device", interface.name)).exists()
}

#[cfg(target_os = "macos")]
fn is_physical(interface: &NetworkInterface) -> bool {
    match Command::new("networksetup")
        .arg("-listallhardwareports")
        .output() 
    {
        Ok(output) => {
            if output.status.success() {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                let expected = format!("Device: {}", interface.name);
                stdout_str.contains(&expected)
            } else { false }
        },
        Err(_) => { false }
    }
}

#[cfg(target_os = "windows")]
fn is_physical(interface: &NetworkInterface) -> bool {
    true
}

/***************************************
   OS dependent functions for WIRELESS
****************************************/
#[cfg(target_os = "linux")]
fn is_wireless(interface: &NetworkInterface) -> bool {
    Path::new(&format!("sys/class/net/{}/wireless", interface.name)).exists()
}

#[cfg(target_os = "macos")]
fn is_wireless(interface: &NetworkInterface) -> bool {
    let output = Command::new("networksetup")
        .arg("-getairportnetwork")
        .arg(&interface.name)
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}