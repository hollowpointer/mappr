use pnet::datalink::NetworkInterface;
use crate::cmd::Target;
use std::path::Path;

pub fn select(target: Target, interfaces: &[NetworkInterface]) -> Option<NetworkInterface> {
    match target {
        Target::LAN => select_lan(interfaces),
    }
}

// Selects the first LAN interface it finds
fn select_lan(interfaces: &[NetworkInterface]) -> Option<NetworkInterface> {
    let mut candidates: Vec<_> = interfaces
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

    // Prefer wired over wireless
    candidates.sort_by_key(|k| is_wireless(*k));
    candidates.first().cloned().cloned()
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