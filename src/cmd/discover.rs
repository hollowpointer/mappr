use pnet::datalink;
use crate::cmd::Target;
use crate::net;

pub fn discover(target: Target) {
    match target {
        Target::LAN => discover_lan()
    }
}

pub fn discover_lan() {
    let interface = net::interface::select(Target::LAN, &datalink::interfaces());
    println!("{:?}", interface)
}