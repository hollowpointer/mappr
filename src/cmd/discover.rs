use crate::cmd::Target;
use crate::host::{self, ExternalHost, Host, InternalHost};
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::datalink::{channel, interface};
use crate::net::tcp_connect;
use crate::net::range::Ipv4Range;
use crate::net::{ip, range};
use crate::print::{self, SPINNER};
use anyhow::{self, Context};
use is_root::is_root;
use pnet::datalink::NetworkInterface;
use std::net::IpAddr;

pub async fn discover(target: Target) -> anyhow::Result<()> {
    SPINNER.set_message("Performing discovery...");
    print::print_status("Initializing discovery...");

    if !is_root() {
        let mut hosts: Vec<Box<dyn Host>> = tcp_handshake_discovery(target).await?;
        return Ok(discovery_ends(&mut hosts)?);
    }

    print::print_status("Root privileges detected. Using advanced techniques...");
    let mut hosts: Vec<Box<dyn Host>> = match target {
        Target::LAN => discover_lan()?,
        Target::Host { dst_addr } => discover_host(dst_addr).await?,
        Target::Range { ipv4_range } => discover_ipv4_range(ipv4_range).await?,
        _ => {
            anyhow::bail!("this target is currently unimplemented!")
        }
    };

    Ok(discovery_ends(&mut hosts)?)
}

fn discover_lan() -> anyhow::Result<Vec<Box<dyn Host>>> {
    let hosts: Vec<InternalHost> = channel::discover_via_eth()?;
    Ok(host::internal_to_box(hosts))
}

async fn discover_host(dst_addr: IpAddr) -> anyhow::Result<Vec<Box<dyn Host>>> {
    if !ip::is_private(dst_addr) {
        return Ok(host::external_to_box(
            tcp_handshake_discovery_host(dst_addr).await?,
        ));
    }
    let host: Vec<InternalHost> = if let Some(host) = channel::discover_via_ip_addr(dst_addr)? {
        vec![host]
    } else {
        return Err(anyhow::anyhow!("Failed to discover the host"));
    };
    Ok(host::internal_to_box(host))
}

async fn discover_ipv4_range(ipv4_range: Ipv4Range) -> anyhow::Result<Vec<Box<dyn Host>>> {
    print::print_status(
        &format!(
            "Discovering from {} to {}",
            ipv4_range.start_addr, ipv4_range.end_addr
        )
        .to_string(),
    );
    if !ipv4_range.start_addr.is_private() {
        let hosts: Vec<ExternalHost> =
            tcp_connect::handshake_range_discovery(ipv4_range, tcp_connect::handshake_probe).await?;
        return Ok(host::external_to_box(hosts));
    }
    Ok(host::internal_to_box(channel::discover_via_range(
        ipv4_range,
    )?))
}

async fn tcp_handshake_discovery(target: Target) -> anyhow::Result<Vec<Box<dyn Host>>> {
    print::print_status("No root privileges. Falling back to non-privileged TCP scan...");
    let hosts: Vec<ExternalHost> = match target {
        Target::LAN => tcp_handshake_discovery_lan().await?,
        Target::Host { dst_addr } => tcp_handshake_discovery_host(dst_addr).await?,
        Target::Range { ipv4_range } => {
            tcp_connect::handshake_range_discovery(ipv4_range, tcp_connect::handshake_probe).await?
        }
        _ => anyhow::bail!("Handshake discovery for this target not implemented!"),
    };
    Ok(host::external_to_box(hosts))
}

async fn tcp_handshake_discovery_lan() -> anyhow::Result<Vec<ExternalHost>> {
    let interface: NetworkInterface = interface::get_lan()?;
    if let Some(ipv4_range) = range::from_ipv4_net(interface.get_ipv4_net()) {
        tcp_connect::handshake_range_discovery(ipv4_range, tcp_connect::handshake_probe)
            .await
            .context("handshake discovery failed (non-root)")
    } else {
        anyhow::bail!("No root privileges and failed to retrieve IPv4 range for TCP scan.")
    }
}

async fn tcp_handshake_discovery_host(dst_addr: IpAddr) -> anyhow::Result<Vec<ExternalHost>> {
    if let Some(host) = tcp_connect::handshake_probe(dst_addr)
        .await
        .context("handshake discovery failed (non-root)")?
    {
        Ok(vec![host])
    } else {
        Err(anyhow::anyhow!(
            "Failed to discover any hosts with a full tcp handshake"
        ))
    }
}

fn discovery_ends(hosts: &mut Vec<Box<dyn Host>>) -> anyhow::Result<()>  {
    if hosts.len() == 0 {
        return Ok(no_hosts_found());
    }
    print::header("Network Discovery");
    hosts.sort_by_key(|host| host.get_primary_ip());
    for (idx, host) in hosts.iter().enumerate() {
        host.print_details(idx);
        if idx + 1 != hosts.len() {
            print::println("");
        }
    }
    print::end_of_program();
    SPINNER.finish_and_clear();
    Ok(())
}

fn no_hosts_found() {
    print::header("ZERO HOSTS DETECTED");
    print::no_results();
    print::end_of_program();
    SPINNER.finish_and_clear();
}
