#![cfg(test)]
use mappr_common::config::Config;
use mappr_common::network::host::Host;
use mappr_common::network::range::{IpCollection, Ipv4Range};
use mappr_core::scanner;
use std::net::{IpAddr, Ipv4Addr};

/// This test verifies that the scanner can discover a local address (localhost).
/// It uses the 'perform_discovery' entry point which automatically selects
/// the appropriate scanning method (privileged vs unprivileged).
#[tokio::test]
async fn discovery_single_loopback() {
    let config: Config = Config {
        no_banner: true,
        no_dns: true,
        redact: false,
        quiet: 0,
        disable_input: true,
    };

    let mut targets: IpCollection = IpCollection::new();
    let localhost: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    targets.add_single(localhost);

    let result = scanner::perform_discovery(targets, &config).await;

    assert!(result.is_ok(), "Discovery failed: {:?}", result.err());
    let hosts: Vec<Host> = result.unwrap();

    assert!(!hosts.is_empty(), "No hosts found when scanning localhost");

    let found_ip = hosts[0].primary_ip;
    assert_eq!(
        found_ip, localhost,
        "Found host IP does not match expected localhost IP"
    );
}

#[tokio::test]
async fn discovery_range_loopback() {
    let cfg: Config = Config {
        no_banner: true,
        no_dns: true,
        redact: false,
        quiet: 0,
        disable_input: true,
    };

    let mut targets: IpCollection = IpCollection::new();
    let start: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
    let end: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 3);
    let range: Ipv4Range = Ipv4Range::new(start, end);
    targets.add_range(range);

    let result = scanner::perform_discovery(targets, &cfg).await;

    assert!(result.is_ok(), "Discovery failed: {:?}", result.is_err());
    let hosts: Vec<Host> = result.unwrap();

    assert!(
        hosts.len() == 3,
        "Found incorrect amount of hosts: {}",
        hosts.len()
    );
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn privileged_discovery_netns() {
    use crate::utils::NetnsContext;

    let _ctx = match NetnsContext::new("test1") {
        Some(c) => c,
        None => {
            eprintln!("Skipping netns test: Requires root privileges or 'ip' command.");
            return;
        }
    };

    let target_ip = IpAddr::V4(Ipv4Addr::new(10, 200, 0, 2));

    let config = Config {
        no_banner: true,
        no_dns: true,
        redact: false,
        quiet: 0,
        disable_input: true,
    };

    let mut collection = IpCollection::new();
    collection.add_single(target_ip);

    let result = scanner::perform_discovery(collection, &config).await;

    match result {
        Ok(hosts) => {
            assert!(!hosts.is_empty(), "Should find the target in the namespace");
            let host = hosts
                .iter()
                .find(|h| h.primary_ip == target_ip)
                .expect("Target IP not found in results");

            assert!(
                host.mac.is_some(),
                "Should resolve MAC address for local neighbor"
            );
            println!("Found host: {:?} with MAC {:?}", host.primary_ip, host.mac);
        }
        Err(e) => panic!("Discovery failed: {}", e),
    }
}
