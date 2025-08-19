use pnet::datalink;
use pnet::ipnetwork::IpNetwork;
use crate::cmd::Target;


pub fn select(target: Target) -> datalink::NetworkInterface {
    match target {
        Target::LAN => select_lan(),
    }
}

// Selects the first LAN interface it finds
fn select_lan() -> datalink::NetworkInterface {
    datalink::interfaces()
        .into_iter()
        .find(|i|
            i.is_up() &&
                i.mac.is_some() &&
                i.is_broadcast() &&
                !i.is_loopback() &&
                !i.is_point_to_point() &&
                i.ips.iter().any(|ip| matches!(ip, IpNetwork::V4(_)))
        )
        .expect("No interface for LAN found")
}