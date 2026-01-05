use std::collections::HashMap;
use std::net::{IpAddr, UdpSocket};
use pnet::datalink::{self, NetworkInterface};
use pnet::ipnetwork::{IpNetwork, Ipv4Network};
use rayon::prelude::*;
#[cfg(target_os = "macos")]
use macos_impl::{is_physical, is_wireless};
#[cfg(target_os = "linux")]
use linux_impl::{is_physical, is_wireless};

use crate::network::range::IpCollection;

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

/// Finds the primary LAN network and returns the ipv4 network
pub fn get_lan_network() -> anyhow::Result<Option<Ipv4Network>> {
    let interfaces: Vec<NetworkInterface> = pnet::datalink::interfaces();
    // print::print_status(format!("Identified {} network interface(s)", interfaces.len()).as_str());

    let interfaces: Vec<NetworkInterface> = interfaces
        .into_iter()
        .filter_map(
            |interface| match is_viable_lan_interface(&interface, is_physical) {
                Ok(()) => Some(interface),
                Err(_) => None,
            },
        )
        .collect();

    let interface: NetworkInterface =
        if let Some(interface) = select_best_lan_interface(interfaces, is_wired) {
            interface
        } else {
            anyhow::bail!("No interfaces available for LAN discovery");
        };

    let private_v4_net: Option<Ipv4Network> = interface.ips.iter().find_map(|net| {
        match net {
            IpNetwork::V4(v4) if v4.ip().is_private() => Some(*v4),
            _ => None,
        }
    });
    
    Ok(private_v4_net)    
}

/// Returns a list of prioritized network interfaces (e.g. wired first, then wireless, etc.)
pub fn get_prioritized_interfaces(limit: usize) -> anyhow::Result<Vec<NetworkInterface>> {
    let mut interfaces: Vec<NetworkInterface> = pnet::datalink::interfaces()
        .into_iter()
        .filter(|i| i.is_up() && !i.is_loopback() && !i.ips.is_empty())
        .collect();

    // Sort: Wired < Wireless (Approximate by name for now as we don't have is_wired easily exposed in this function scope without cfg)
    // Actually we have is_wired in the module but it takes extensive cfg.
    // For simplicity of this refactor, just return them. 
    // Ideally we would reuse select_best logic but that returns ONE.
    // Let's just return all valid ones up to limit.
    
    // Sort logic (simple):
    interfaces.sort_by_key(|i| if i.name.starts_with("e") { 0 } else { 1 });

    Ok(interfaces.into_iter().take(limit).collect())
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
        return Err(ViabilityError::NotPhysical);
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
    let has_valid_ip = interface.ips.iter().any(|net| match net {
        IpNetwork::V4(ipv4) => ipv4.ip().is_private(),
        IpNetwork::V6(ipv6) => ipv6.ip().is_unicast_link_local(),
    });
    if !has_valid_ip {
        return Err(ViabilityError::NoValidLanIp);
    }

    Ok(())
}

fn select_best_lan_interface(
    interfaces: Vec<NetworkInterface>,
    is_wired: impl Fn(&NetworkInterface) -> bool,
) -> Option<NetworkInterface> {
    match interfaces.len() {
        0 => None,
        1 => Some(interfaces[0].clone()),
        _ => {
            // print::print_status("More than one candidate found, selecting best option...");
            interfaces
                .iter()
                .find(|&interface| is_wired(interface))
                .map(|iface_ref_ref| iface_ref_ref.clone())
                .or(Some(interfaces[0].clone()))
        }
    }
}



/// Maps target IPs to the local interface used to route to them.
/// Supports both individual IPs and Ranges for efficiency.
pub fn map_ips_to_interfaces(mut collection: IpCollection) -> HashMap<NetworkInterface, IpCollection> {
    let interfaces: Vec<NetworkInterface> = datalink::interfaces()
        .into_iter()
        .filter(|i| i.is_up() && !i.is_loopback() && !i.ips.is_empty())
        .collect();

    let ip_to_idx: HashMap<IpAddr, usize> = interfaces.iter()
        .enumerate()
        .flat_map(|(idx, iface)| iface.ips.iter().map(move |ip_net| (ip_net.ip(), idx)))
        .collect();

    let mut result_map: HashMap<usize, IpCollection> = HashMap::new();

    // 1. Process Ranges (Optimization)
    // We attempt to keep ranges intact if they fit within a single interface's subnet.
    // If a range isn't fully contained, we decompose it into singles for robust routing.
    for range in collection.ranges {
        let start = range.start_addr;
        let end = range.end_addr;
        
        let mut owner_idx: Option<usize> = None;
        
        // Find if any interface strictly contains this entire range
        for (idx, iface) in interfaces.iter().enumerate() {
            let contains_range = iface.ips.iter().any(|ip_net| {
               match ip_net {
                   IpNetwork::V4(v4) => {
                       // Check if start and end are in this subnet
                       v4.contains(start) && v4.contains(end)
                   },
                   _ => false
               }
            });
            
            if contains_range {
                owner_idx = Some(idx);
                break;
            }
        }

        if let Some(idx) = owner_idx {
            result_map.entry(idx).or_default().add_range(range);
        } else {
            // Range splits across boundaries or is unroutable via simple subnet check.
            // Fallback: Expand to singles and let the single-IP routing logic handle it.
            // This ensures correctness at the cost of the optimization for this edge case.
            for ip in range.to_iter() {
                collection.singles.insert(ip);
            }
        }
    }

    // 2. Process Singles (Parallelized)
    type ThreadSockets = (Option<UdpSocket>, Option<UdpSocket>);

    // Extract singles for processing
    let singles: Vec<IpAddr> = collection.singles.into_iter().collect();

    let routed_singles: Vec<(usize, IpAddr)> = singles.par_iter()
        .map_init(
            || -> ThreadSockets { (None, None) }, 
            |sockets, &target_ip| {
                if let Some(idx) = find_local_index(&interfaces, target_ip) {
                    return Some((idx, target_ip));
                }

                let source_ip = resolve_route_source_ip(target_ip, sockets)?;
                
                ip_to_idx.get(&source_ip).copied().map(|idx| (idx, target_ip))
            }
        )
        .filter_map(|res| res)
        .collect();

    for (idx, ip) in routed_singles {
        result_map.entry(idx).or_default().add_single(ip);
    }

    result_map.into_iter()
        .map(|(idx, collection)| (interfaces[idx].clone(), collection))
        .collect()
}


fn find_local_index(interfaces: &[NetworkInterface], target: IpAddr) -> Option<usize> {
    interfaces.iter().position(|iface| {
        iface.ips.iter().any(|ip_net| {
            match (target, ip_net.ip()) {
                (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => {
                    ip_net.contains(target)
                },
                _ => false,
            }
        })
    })
}

fn resolve_route_source_ip(target: IpAddr, sockets: &mut (Option<UdpSocket>, Option<UdpSocket>)) -> Option<IpAddr> {
    let socket_opt = if target.is_ipv4() {
        &mut sockets.0
    } else {
        &mut sockets.1
    };

    if socket_opt.is_none() {
        let bind_addr = if target.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        *socket_opt = UdpSocket::bind(bind_addr).ok();
    }

    let socket = socket_opt.as_ref()?;

    socket.connect((target, 53)).ok()?;
    socket.local_addr().ok().map(|s| s.ip())
}

fn is_wired(interface: &NetworkInterface) -> bool {
    is_physical(interface) && !is_wireless(interface)
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use std::path::Path;

    pub fn is_physical(interface: &NetworkInterface) -> bool {
        Path::new(&format!("/sys/class/net/{}/device", interface.name)).exists()
    }

    pub fn is_wireless(interface: &NetworkInterface) -> bool {
        Path::new(&format!("sys/class/net/{}/wireless", interface.name)).exists()
    }

}

#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;
    use std::collections::HashSet;
    use std::process::Command;
    use std::sync::OnceLock;

    /// A struct to hold the cached hardware information
    struct HardwareInfo {
        physical_devices: HashSet<String>,
        wireless_devices: HashSet<String>,
    }

    /// Singleton that runs the shell commands only once on first access.
    fn get_hardware_info() -> &'static HardwareInfo {
        static HARDWARE_INFO: OnceLock<HardwareInfo> = OnceLock::new();

        HARDWARE_INFO.get_or_init(|| {
            let mut physical = HashSet::new();
            let mut wireless = HashSet::new();

            // Get Physical Ports (Wired & Wireless hardware)
            if let Ok(output) = Command::new("networksetup").arg("-listallhardwareports").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Some(device) = line.strip_prefix("Device: ") {
                        physical.insert(device.trim().to_string());
                    }
                }
            }

            // Identify which of those are specifically Wireless
            for device in &physical {
                let is_wifi = Command::new("networksetup")
                    .arg("-getairportnetwork")
                    .arg(device)
                    .output()
                    .map(|out| out.status.success())
                    .unwrap_or(false);
                
                if is_wifi {
                    wireless.insert(device.clone());
                }
            }

            HardwareInfo {
                physical_devices: physical,
                wireless_devices: wireless,
            }
        })
    }

    pub fn is_physical(interface: &NetworkInterface) -> bool {
        get_hardware_info().physical_devices.contains(&interface.name)
    }

    pub fn is_wireless(interface: &NetworkInterface) -> bool {
        get_hardware_info().wireless_devices.contains(&interface.name)
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
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use pnet::ipnetwork::{Ipv4Network, Ipv6Network, IpNetwork};
    use pnet::util::MacAddr;

    const IFF_UP: u32 = 1;
    const IFF_BROADCAST: u32 = 1 << 1;
    const IFF_LOOPBACK: u32 = 1 << 3;
    const IFF_POINTTOPOINT: u32 = 1 << 4;
    //const IFF_RUNNING: u32 = 1 << 6;

    fn create_mock_interface(
        name: &str,
        mac: Option<MacAddr>,
        ips: Vec<IpNetwork>,
        flags: u32,
    ) -> NetworkInterface {
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
        vec![IpNetwork::V4("192.168.1.100".parse().unwrap())]
    }

    #[test]
    fn is_viable_lan_interface_should_succeed() {
        let interface: NetworkInterface =
            create_mock_interface("eth0", default_mac(), default_ips(), IFF_UP | IFF_BROADCAST);
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Ok(()))
    }

    #[test]
    fn is_viable_lan_interface_should_succeed_with_ipv6_link_local() {
        let ipv6_ips = vec![IpNetwork::V6("fe80::1234:5678:abcd:ef01".parse().unwrap())];
        let interface: NetworkInterface =
            create_mock_interface("eth0", default_mac(), ipv6_ips, IFF_UP | IFF_BROADCAST);
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn is_viable_lan_interface_should_fail_with_invalid_ipv6() {
        let invalid_ipv6_ips = vec![IpNetwork::V6("2001:db8::1".parse().unwrap())];
        let interface: NetworkInterface = create_mock_interface(
            "eth0",
            default_mac(),
            invalid_ipv6_ips,
            IFF_UP | IFF_BROADCAST,
        );
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NoValidLanIp));
    }

    #[test]
    fn is_viable_lan_interface_should_fail_non_physical() {
        let interface: NetworkInterface =
            create_mock_interface("eth1", default_mac(), default_ips(), IFF_UP | IFF_BROADCAST);
        let is_physical = |_: &NetworkInterface| -> bool { false };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NotPhysical))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_no_mac_addr() {
        let interface: NetworkInterface =
            create_mock_interface("eth0", None, default_ips(), IFF_UP | IFF_BROADCAST);
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NoMacAddress))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_no_ips() {
        let interface: NetworkInterface =
            create_mock_interface("eth8", default_mac(), vec![], IFF_UP | IFF_BROADCAST);
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NoValidLanIp))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_when_down() {
        let interface: NetworkInterface =
            create_mock_interface("wlan0", default_mac(), default_ips(), IFF_BROADCAST);
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::IsDown))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_loop_back() {
        let interface: NetworkInterface = create_mock_interface(
            "lo",
            default_mac(),
            default_ips(),
            IFF_LOOPBACK | IFF_UP | IFF_BROADCAST,
        );
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NotPhysical))
    }

    #[test]
    fn is_viable_lan_interface_should_fail_not_broadcast() {
        let interface: NetworkInterface =
            create_mock_interface("eth0", default_mac(), default_ips(), IFF_UP);
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::NotBroadcast));
    }

    #[test]
    fn is_viable_lan_interface_should_fail_point_to_point() {
        let interface: NetworkInterface = create_mock_interface(
            "tun0",
            default_mac(),
            default_ips(),
            IFF_BROADCAST | IFF_POINTTOPOINT | IFF_UP,
        );
        let is_physical = |_: &NetworkInterface| -> bool { true };
        let result: Result<(), ViabilityError> = is_viable_lan_interface(&interface, is_physical);
        assert_eq!(result, Err(ViabilityError::IsPointToPoint))
    }

    #[test]
    fn select_best_lan_interface_selects_first_interface() {
        let interface: NetworkInterface = create_mock_interface(
            "wlan0",
            default_mac(),
            default_ips(),
            IFF_UP | IFF_BROADCAST,
        );
        let is_wired = |interface: &NetworkInterface| -> bool { interface.name == "eth0" };
        let result = select_best_lan_interface(vec![interface], is_wired);
        assert!(result.is_some(), "Should have selected an interface");
        assert_eq!(result.unwrap().name, "wlan0");
    }

    #[test]
    fn select_best_lan_interface_selects_wired_over_wireless() {
        let wired_interface: NetworkInterface =
            create_mock_interface("eth0", default_mac(), default_ips(), IFF_UP | IFF_BROADCAST);
        let wireless_interface: NetworkInterface = create_mock_interface(
            "wlan0",
            default_mac(),
            default_ips(),
            IFF_UP | IFF_BROADCAST,
        );
        let is_wired = |interface: &NetworkInterface| -> bool { interface.name == "eth0" };
        let interfaces: Vec<NetworkInterface> = vec![wireless_interface, wired_interface];
        let result = select_best_lan_interface(interfaces, is_wired);
        assert!(result.is_some(), "Should have selected an interface");
        assert_eq!(result.unwrap().name, "eth0");
    }

    #[test]
    fn select_best_lan_interface_returns_none() {
        let is_wired = |interface: &NetworkInterface| -> bool { interface.name == "eth0" };
        let interfaces: Vec<NetworkInterface> = vec![];
        let result = select_best_lan_interface(interfaces, is_wired);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_local_index_ipv4() {
        // Mock a network interface: 192.168.1.5/24
        let iface = NetworkInterface {
            name: "eth0".to_string(),
            description: "".to_string(),
            index: 1,
            mac: None,
            ips: vec![IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(192, 168, 1, 5), 24).unwrap())],
            flags: 0,
        };
        let interfaces = vec![iface];

        // Case 1: IP is inside the subnet (192.168.1.20)
        let target_inside = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
        assert_eq!(find_local_index(&interfaces, target_inside), Some(0));

        // Case 2: IP is outside the subnet (192.168.2.20)
        let target_outside = IpAddr::V4(Ipv4Addr::new(192, 168, 2, 20));
        assert_eq!(find_local_index(&interfaces, target_outside), None);
    }

    #[test]
    fn test_find_local_index_ipv6() {
        // Mock a network interface: 2001:db8::1/64
        let ipv6_addr = "2001:db8::1".parse::<Ipv6Addr>().unwrap();
        let iface = NetworkInterface {
            name: "eth0".to_string(),
            description: "".to_string(),
            index: 1,
            mac: None,
            ips: vec![IpNetwork::V6(Ipv6Network::new(ipv6_addr, 64).unwrap())],
            flags: 0,
        };
        let interfaces = vec![iface];

        // Case 1: IP is inside the subnet
        let target_inside = "2001:db8::5".parse::<IpAddr>().unwrap();
        assert_eq!(find_local_index(&interfaces, target_inside), Some(0));

        // Case 2: IP mismatch (IPv4 vs IPv6)
        let target_v4 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(find_local_index(&interfaces, target_v4), None);
    }

    #[test]
    fn test_resolve_route_source_ip_localhost() {
        // This test ensures the socket logic works without crashing.
        // Routing to 127.0.0.1 should theoretically return 127.0.0.1 or the generic bind address.
        let mut sockets = (None, None);
        let target = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        
        let result = resolve_route_source_ip(target, &mut sockets);
        assert!(result.is_some(), "Should be able to resolve route to localhost");
        assert_eq!(result.unwrap(), target);
    }

    #[test]
    fn test_resolve_route_public_internet() {
        // Try routing to Google DNS (8.8.8.8). 
        // This tests if the OS kernel can determine a route for an external IP.
        // NOTE: This test will fail if the machine has no internet connection.
        let mut sockets = (None, None);
        let target = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        
        let result = resolve_route_source_ip(target, &mut sockets);
        
        if result.is_some() {
            let src_ip = result.unwrap();
            assert!(src_ip.is_ipv4());
            assert!(!src_ip.is_loopback());
            assert!(!src_ip.is_unspecified());
        } else {
            // If we are offline, we warn rather than fail hard
            eprintln!("WARNING: Could not resolve route to 8.8.8.8 (Are you offline?)");
        }
    }

    #[test]
    fn test_map_ips_smoke_test() {
        // High level smoke test to ensure the parallel pipeline doesn't panic.
        let ips = vec![
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
        ];

        let mut collection = IpCollection::new();
        for ip in ips {
            collection.add_single(ip);
        }

        let result = map_ips_to_interfaces(collection);
        
        for (iface, routed_ips) in result {
            println!("Interface {} routes: {:?}", iface.name, routed_ips);
            assert!(!iface.ips.is_empty());
        }
    }
}