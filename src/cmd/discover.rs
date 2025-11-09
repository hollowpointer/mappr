use pnet::datalink::NetworkInterface;
use pnet::ipnetwork::Ipv4Network;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use anyhow::{self, Context};
use is_root::is_root;
use crate::host::Host;
use crate::cmd::Target;
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::datalink::{channel, interface};
use crate::net::{ip, range};
use crate::net::packets::tcp::{self, handshake_range_discovery};
use crate::net::range::Ipv4Range;
use crate::{host, print, SPINNER};

pub async fn discover(target: Target) -> anyhow::Result<()> {
    let hosts: Vec<Host> = match target {
        Target::LAN => { 
            SPINNER.set_message("Performing LAN discovery...");
            discover_lan().await? 
        },
        Target::Host { dst_addr } => {
            SPINNER.set_message("Performing host discovery...");
            if let Some(host) = discover_host(dst_addr).await? {
                vec![host]
            } else { vec![] }
        }
        _ => { anyhow::bail!("this target is currently unimplemented!") }
    };
    print::header("Network Discovery");
    host::print(hosts, target)?;
    print::end_of_program();
    SPINNER.finish_and_clear();
    Ok(())
}

async fn discover_lan() -> anyhow::Result<Vec<Host>> {
    print::print_status("Initializing discovery...");
    let interface: NetworkInterface = interface::select(Target::LAN);
    if is_root() {
        let ipv4_net: Option<Ipv4Network> = interface.get_ipv4_net();
        let src_addr_v4: Option<Ipv4Addr> = ipv4_net.map(|net| net.ip());
        let ipv4_range: Option<Ipv4Range> = range::from_ipv4_net(ipv4_net);
        let link_local_addr: Option<Ipv6Addr> = interface.get_link_local_addr();
        print::print_status("Root privileges detected. Starting L2/L3 scan...");
        return channel::discover_via_eth(
            interface,
            src_addr_v4,
            ipv4_range,
            link_local_addr,
        )
    } else {
        // Non-root path
        print::print_status("No root privileges. Falling back to non-privileged TCP scan...");
        if let Some(ipv4_range) = range::from_ipv4_net(interface.get_ipv4_net()) {
            handshake_range_discovery(ipv4_range)
                .await
                .context("handshake discovery failed (non-root)")
        } else {
            anyhow::bail!("No root privileges and failed to retrieve IPv4 range for TCP scan.")
        }
    }
}

async fn discover_host(dst_addr: IpAddr) -> anyhow::Result<Option<Host>> {
    if !is_root() || !ip::is_private(dst_addr) {
        print::print_status("No root privileges. Falling back to non-privileged TCP scan...");
        return tcp::handshake_probe(dst_addr).await.context("handshake discovery failed (non-root)");
    }
    print::print_status("Root privileges detected. Starting L2/L3 scan...");
    let intf: NetworkInterface = interface::select(Target::LAN);
    let host: Option<Host> = channel::discover_via_ip_addr(
        intf,
        dst_addr,
    )?;
    Ok(host)
}