use pnet::datalink::NetworkInterface;
use crate::cmd::Target;

pub fn select(target: Target, interfaces: &[NetworkInterface]) -> Option<NetworkInterface> {
    match target {
        Target::LAN => select_lan(interfaces),
    }
}

// Selects the first LAN interface it finds
fn select_lan(interfaces: &[NetworkInterface]) -> Option<NetworkInterface> {
    interfaces
        .iter()
        .find(|i|
            i.is_up()
                && i.mac.is_some()
                && i.is_broadcast()
                && !i.is_loopback()
                && !i.is_point_to_point()
                && i.ips.iter().any(|ip| ip.is_ipv4())
        )
        .cloned()
}