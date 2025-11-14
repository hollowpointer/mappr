use std::net::Ipv6Addr;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;
use colored::{ColoredString, Colorize};
use pnet::{self, datalink::NetworkInterface, ipnetwork::Ipv4Network};
use anyhow;
use pnet::ipnetwork::IpNetwork;
use crate::{net::ip, utils::colors};
use crate::print;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ViabilityError {
    /// The interface is operationally down.
    IsDown,
    /// The interface was filtered out as "not physical" by the provided logic.
    NotPhysical,
    /// The interface does not have a MAC address.
    NoMacAddress,
    /// The interface does not support broadcast (required for ARP).
    NotBroadcast,
    /// The interface is a point-to-point link (e.g., a VPN).
    IsPointToPoint,
    /// The interface has no IPv4 address (for ARP) AND no IPv6 Link-Local (for NDP).
    NoValidLanIp,
}

pub trait NetworkInterfaceExtension {
    fn print_details(&self, idx: usize);
    fn get_ipv4_net(&self) -> Option<Ipv4Network>;
    fn get_link_local_addr(&self) -> Option<Ipv6Addr>;
}


impl NetworkInterfaceExtension for NetworkInterface {
    fn print_details(self: &Self, idx: usize) {
        print::tree_head(idx, &self.name);
        let mut key_value_pair: Vec<(String, ColoredString)> = ip::to_key_value_pair_net(&self.ips);
        if let Some(mac_addr) = self.mac {
            key_value_pair.push(("MAC".to_string(), mac_addr.to_string().color(colors::MAC_ADDR)));
        }
        print::as_tree_one_level(key_value_pair);
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


pub fn get_lan() -> anyhow::Result<NetworkInterface> {
    let interfaces: Vec<NetworkInterface> = pnet::datalink::interfaces();
    print::print_status(format!("Identified {} network interface(s)", interfaces.len()).as_str());

    let interfaces: Vec<NetworkInterface> = interfaces
        .into_iter() 
        .filter_map(|interface| {
            match is_viable_lan_interface(&interface, is_physical) {
                Ok(()) => { Some(interface) }
                Err(_) => { None }
            }
    })
    .collect();

    let interface: NetworkInterface = if let Some(interface) = select_best_lan_interface(interfaces, is_wired) {
        interface
    } else {
        anyhow::bail!("No interfaces available for LAN discovery");
    };
    print_lan_interface_info(&interface);
    Ok(interface)
}


fn is_viable_lan_interface(
    interface: &NetworkInterface,
    is_physical: impl Fn(&NetworkInterface) -> bool,
) -> Result<(), ViabilityError> {
    
    if !interface.is_up() {
        return Err(ViabilityError::IsDown);
    }
    if !is_physical(interface) {
        return Err(ViabilityError::NotPhysical);
    }
    if interface.is_loopback() {
        return Err(ViabilityError::NotPhysical)   
    }
    if interface.mac.is_none() {
        return Err(ViabilityError::NoMacAddress);
    }
    if !interface.is_broadcast() {
        return Err(ViabilityError::NotBroadcast);
    }
    if interface.is_point_to_point() {
        return Err(ViabilityError::IsPointToPoint);
    }
    let has_valid_ip = interface.ips.iter().any(|net| {
        match net {
            IpNetwork::V4(ipv4) => ipv4.ip().is_private(),
            IpNetwork::V6(ipv6) => ipv6.ip().is_unicast_link_local(),
        }
    });
    if !has_valid_ip {
        return Err(ViabilityError::NoValidLanIp);
    }

    Ok(())
}


fn select_best_lan_interface(
    interfaces: Vec<NetworkInterface>,
    is_wired: impl Fn(&NetworkInterface) -> bool
) -> Option<NetworkInterface> 
{
    match interfaces.len() {
        0 => None,
        1 => Some(interfaces[0].clone()),
        _ => {
            print::print_status("More than one candidate found, selecting best option...");
            interfaces.iter()
                .find(|&interface| is_wired(interface))
                .map(|iface_ref_ref| iface_ref_ref.clone())
                .or(Some(interfaces[0].clone()))
        }
    }
}


fn print_lan_interface_info(interface: &NetworkInterface) {
    if let Some(ipv4_net) = interface.get_ipv4_net() {
        let msg: &str = &format!("Selected {} with address {}", interface.name, ipv4_net.ip());
        print::print_status(msg);
        return;
    }
    if let Some(link_local_addr) = interface.get_link_local_addr() {
        let msg: &str = &format!("Selected {} with address {}", interface.name, link_local_addr);
        print::print_status(msg);
        return;
    }
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



// ╔════════════════════════════════════════════╗
// ║ ████████╗███████╗███████╗████████╗███████╗ ║
// ║ ╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝ ║
// ║    ██║   █████╗  ███████╗   ██║   ███████╗ ║
// ║    ██║   ██╔══╝  ╚════██║   ██║   ╚════██║ ║
// ║    ██║   ███████╗███████║   ██║   ███████║ ║
// ║    ╚═╝   ╚══════╝╚══════╝   ╚═╝   ╚══════╝ ║
// ╚════════════════════════════════════════════╝

#[cfg(test)]
mod tests {
    use pnet::util::MacAddr;
    use super::*;

    const IFF_UP: u32 = 1;
    const IFF_BROADCAST: u32 = 1 << 1;
    const IFF_LOOPBACK: u32 = 1 << 3;
    const IFF_POINTTOPOINT: u32 = 1 << 4;
    //const IFF_RUNNING: u32 = 1 << 6;

    fn create_mock_interface(name: &str, mac: Option<MacAddr>, ips: Vec<IpNetwork>, flags: u32) -> NetworkInterface {
        NetworkInterface {
            name: name.to_string(),
            description: "An interface".to_string(),
            index: 0,
            mac,
            ips,
            flags,
        }
    }

    fn default_mac() -> Option<MacAddr> {
        Some(MacAddr(0x1, 0x2, 0x3, 0x4, 0x5, 0x6))
    }

    fn default_ips() -> Vec<IpNetwork> {
        vec![
            IpNetwork::V4("192.168.1.100".parse().unwrap())
        ]
    }

    #[test]
    fn is_viable_lan_interface_should_succeed() {
        let interface: NetworkInterface = create_mock_interface(
            "eth0", 
            default_mac(), 
            default_ips(),
            IFF_UP | IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Ok(()))
    }

    #[test]
    fn is_viable_lan_interface_should_succeed_with_ipv6_link_local() {
        let ipv6_ips = vec![
            IpNetwork::V6("fe80::1234:5678:abcd:ef01".parse().unwrap())
        ];
        let interface: NetworkInterface = create_mock_interface(
            "eth0", 
            default_mac(), 
            ipv6_ips,
            IFF_UP | IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn is_viable_lan_interface_should_fail_with_invalid_ipv6() {
        let invalid_ipv6_ips = vec![
            IpNetwork::V6("2001:db8::1".parse().unwrap()) 
        ];
        let interface: NetworkInterface = create_mock_interface(
            "eth0", 
            default_mac(), 
            invalid_ipv6_ips,
            IFF_UP | IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NoValidLanIp));
    }

    #[test]
    fn is_viable_lan_interface_should_fail_non_physical() {
        let interface: NetworkInterface = create_mock_interface(
            "eth1", 
            default_mac(), 
            default_ips(),
            IFF_UP | IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            false
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NotPhysical))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_no_mac_addr() {
        let interface: NetworkInterface = create_mock_interface(
            "eth0", 
            None, 
            default_ips(),
            IFF_UP | IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NoMacAddress))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_no_ips() {
        let interface: NetworkInterface = create_mock_interface(
            "eth8", 
            default_mac(), 
            vec![],
            IFF_UP | IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NoValidLanIp))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_when_down() {
        let interface: NetworkInterface = create_mock_interface(
            "wlan0", 
            default_mac(), 
            default_ips(),
            IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::IsDown))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_loop_back() {
        let interface: NetworkInterface = create_mock_interface(
            "lo", 
            default_mac(), 
            default_ips(),
            IFF_LOOPBACK | IFF_UP | IFF_BROADCAST
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NotPhysical))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_not_broadcast() {
        let interface: NetworkInterface = create_mock_interface(
            "eth0", 
            default_mac(), 
            default_ips(),
            IFF_UP
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NotBroadcast));
    }

    #[test]
    fn is_viable_lan_interface_should_fail_point_to_point() {
        let interface: NetworkInterface = create_mock_interface(
            "tun0", 
            default_mac(), 
            default_ips(),
            IFF_BROADCAST | IFF_POINTTOPOINT | IFF_UP
        );
        let is_physical = |_: &NetworkInterface| -> bool {
            true
        };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::IsPointToPoint))
    }

    #[test]
    fn select_best_lan_interface_selects_first_interface() {
        let interface: NetworkInterface = create_mock_interface("wlan0", 
            default_mac(), 
            default_ips(), 
            IFF_UP | IFF_BROADCAST
        );
        let is_wired = |interface: &NetworkInterface| -> bool {
            interface.name == "eth0"
        };
        let result = select_best_lan_interface(vec![interface], is_wired);
        assert!(result.is_some(), "Should have selected an interface");
        assert_eq!(result.unwrap().name, "wlan0");        
    }

    #[test]
    fn select_best_lan_interface_selects_wired_over_wireless() {
        let wired_interface: NetworkInterface = create_mock_interface("eth0", 
            default_mac(), 
            default_ips(), 
            IFF_UP | IFF_BROADCAST
        );
        let wireless_interface: NetworkInterface = create_mock_interface("wlan0", 
            default_mac(), 
            default_ips(), 
            IFF_UP | IFF_BROADCAST
        );
        let is_wired = |interface: &NetworkInterface| -> bool {
            interface.name == "eth0"
        };
        let interfaces: Vec<NetworkInterface> = vec![wireless_interface, wired_interface];
        let result = select_best_lan_interface(interfaces, is_wired);
        assert!(result.is_some(), "Should have selected an interface");
        assert_eq!(result.unwrap().name, "eth0");        
    }

    #[test]
    fn select_best_lan_interface_returns_none() {
        let is_wired = |interface: &NetworkInterface| -> bool {
            interface.name == "eth0"
        };
        let interfaces: Vec<NetworkInterface> = vec![];
        let result = select_best_lan_interface(interfaces, is_wired);
        assert!(result.is_none());
    }    
}