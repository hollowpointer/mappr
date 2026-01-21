//! # Scan Target Model
//!
//! Defines the possible inputs for a network scan.
//!
//! This module handles parsing and representing targets, which can be:
//! * A single IP address (host).
//! * An IPv4 Range (e.g., `192.168.1.1-100`).
//! * A CIDR block (e.g., `192.168.1.0/24`).
//! * The local LAN (detected automatically).

use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::{info, success, warn};

use crate::network::interface;
use crate::network::range::{self, IpCollection, Ipv4Range};

pub static IS_LAN_SCAN: AtomicBool = AtomicBool::new(false);

/// Represents a distinct target to be scanned.
#[derive(Clone, Debug)]
pub enum Target {
    /// Scan the entire local area network (requires root for advanced features).
    LAN,
    /// Scan a single specific host.
    Host { target_addr: IpAddr },
    /// Scan a range of IPv4 addresses.
    Range { ipv4_range: Ipv4Range },
    /// Scan via VPN interface (placeholder).
    VPN,
    /// Holds a list of different targets
    Multi { targets: Vec<Target> },
}

impl FromStr for Target {
    type Err = String;

    /// Parses a string into a `Target`.
    ///
    /// Supported formats:
    /// * **Keywords**: "lan", "vpn" (case-insensitive).
    /// * **Host**: Single IPv4/IPv6 address (e.g., "192.168.1.5").
    /// * **Range**: "Start-End" (e.g., "192.168.1.1-50", "192.168.1.1-192.168.1.50").
    /// * **CIDR**: "Network/Prefix" (e.g., "192.168.1.0/24").
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();

        if let Some(target) = parse_keyword(&lower) {
            return Ok(target);
        }

        if s.contains(',') {
            if let Some(target) = parse_commas(s).ok() {
                return Ok(target);
            }
        }

        if let Some(target) = parse_host(s) {
            return Ok(target);
        }

        if let Some(target) = parse_ip_range(s)? {
            return Ok(target);
        }

        if let Some(target) = parse_cidr_range(s)? {
            return Ok(target);
        }

        Err(format!("invalid target: {s}"))
    }
}

/// This prevents code duplication between single-target and multi-target parsing.
fn resolve_target(target: Target, collection: &mut IpCollection) -> anyhow::Result<()> {
    match target {
        Target::LAN => {
            if let Some(net) = interface::get_lan_network()? {
                let net_u32: u32 = u32::from(net.network());
                let broadcast_u32: u32 = u32::from(net.broadcast());

                // Calculates usable range (exclude network and broadcast)
                let start_u32 = net_u32.saturating_add(1);
                let end_u32 = broadcast_u32.saturating_sub(1);

                let start_ip = Ipv4Addr::from(start_u32);
                let end_ip = Ipv4Addr::from(end_u32);

                if start_u32 <= end_u32 {
                    IS_LAN_SCAN.store(true, Ordering::Relaxed);
                    info!("Searching for hosts from {start_ip} to {end_ip}");
                    collection.add_range(Ipv4Range::new(start_ip, end_ip));
                } else {
                    warn!("Network too small to strip broadcast, scanning full range.");
                    collection.add_range(Ipv4Range::new(net.network(), net.broadcast()));
                }
            }
        }
        Target::Host { target_addr } => {
            collection.add_single(target_addr);
        }
        Target::Range { ipv4_range } => {
            collection.add_range(ipv4_range);
        }
        Target::VPN => {
            // TODO: Implement VPN logic
            anyhow::bail!("VPN scan target not yet implemented");
        }
        Target::Multi { targets } => {
            for target in targets {
                resolve_target(target, collection)?;
            }
        }
    }
    Ok(())
}

/// Converts a single target into an IP collection.
pub fn to_collection(target: Target) -> anyhow::Result<IpCollection> {
    let mut collection = IpCollection::new();

    resolve_target(target, &mut collection)?;

    let len: usize = collection.len();
    let unit: &str = if len == 1 { "IP address has been" } else { "IP addresses have been" };
    success!("{len} {unit} parsed successfully");

    Ok(collection)
}

/// Parses a comma-separated list of targets (e.g., "192.168.1.5, 10.0.0.1-50, lan").
pub fn parse_commas(s: &str) -> anyhow::Result<Target> {
    let mut targets = Vec::new();

    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Parse the string into a Target (Host, Range, LAN, etc.)
        let target = Target::from_str(part)
            .map_err(|e| anyhow::anyhow!("Failed to parse target '{}': {}", part, e))?;

        targets.push(target);
    }

    Ok(Target::Multi { targets })
}

/// Parses special keywords like "lan" or "vpn".
fn parse_keyword(s_lower: &str) -> Option<Target> {
    match s_lower {
        "lan" => Some(Target::LAN),
        "vpn" => Some(Target::VPN),
        _ => None,
    }
}

/// Parses a single IP address.
fn parse_host(s: &str) -> Option<Target> {
    s.parse::<IpAddr>()
        .ok()
        .map(|target_addr| Target::Host { target_addr })
}

/// Parses a range string like "1.1.1.1-2.2.2.2" or "1.1.1.1-50".
fn parse_ip_range(s: &str) -> Result<Option<Target>, String> {
    let Some((start_str, end_str)) = s.split_once('-') else {
        return Ok(None);
    };

    let start_addr = start_str
        .parse::<Ipv4Addr>()
        .map_err(|e| format!("Invalid start IP in range '{start_str}': {e}"))?;

    let end_addr = parse_range_end_addr(end_str, &start_addr, s)?;

    let ipv4_range = Ipv4Range::new(start_addr, end_addr);
    Ok(Some(Target::Range { ipv4_range }))
}

/// Helper to parse the end address of a range.
///
/// Handles abbreviated forms like "192.168.1.1-50" (implies 192.168.1.50)
/// and full forms like "192.168.1.1-192.168.1.255".
fn parse_range_end_addr(
    end_str: &str,
    start_addr: &Ipv4Addr,
    original_s: &str,
) -> Result<Ipv4Addr, String> {
    if let Ok(full_addr) = end_str.parse::<Ipv4Addr>() {
        return Ok(full_addr);
    }

    let mut end_octets = start_addr.octets();
    let partial_octets: Vec<u8> = end_str
        .split('.')
        .map(|octet_str| octet_str.parse::<u8>())
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| format!("Invalid end range '{end_str}': {e}"))?;

    if partial_octets.is_empty() {
        return Err(format!("End range cannot be empty: {original_s}"));
    }
    if partial_octets.len() > 4 {
        return Err(format!("End range has too many octets: {end_str}"));
    }

    let partial_len = partial_octets.len();
    let start_index = 4 - partial_len;
    end_octets[start_index..].copy_from_slice(&partial_octets);

    Ok(Ipv4Addr::from(end_octets))
}

/// Parses CIDR notation like "192.168.1.0/24".
fn parse_cidr_range(s: &str) -> Result<Option<Target>, String> {
    let Some((ip_str, prefix_str)) = s.split_once('/') else {
        return Ok(None);
    };

    let ipv4_addr = ip_str
        .parse::<Ipv4Addr>()
        .map_err(|e| format!("Invalid IP in CIDR '{ip_str}': {e}"))?;

    let prefix = prefix_str
        .parse::<u8>()
        .map_err(|e| format!("Invalid prefix in CIDR '{prefix_str}': {e}"))?;

    let ipv4_range = range::cidr_range(ipv4_addr, prefix).map_err(|e| e.to_string())?;

    Ok(Some(Target::Range { ipv4_range }))
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
    use std::net::Ipv4Addr;

    #[test]
    fn test_parse_range_end_addr_helper() {
        let start = Ipv4Addr::new(192, 168, 1, 10);
        let s = "192.168.1.10-255";

        // Test full IP end
        assert_eq!(
            parse_range_end_addr("192.168.1.50", &start, s),
            Ok(Ipv4Addr::new(192, 168, 1, 50))
        );

        // Test partial 1-octet end
        assert_eq!(
            parse_range_end_addr("50", &start, s),
            Ok(Ipv4Addr::new(192, 168, 1, 50))
        );

        // Test partial 2-octet end
        assert_eq!(
            parse_range_end_addr("2.66", &start, s),
            Ok(Ipv4Addr::new(192, 168, 2, 66))
        );

        // Test partial 3-octet end
        assert_eq!(
            parse_range_end_addr("10.2.1", &start, s),
            Ok(Ipv4Addr::new(192, 10, 2, 1))
        );

        // Test partial 4-octet end (same as full)
        assert_eq!(
            parse_range_end_addr("10.20.30.40", &start, s),
            Ok(Ipv4Addr::new(10, 20, 30, 40))
        );

        // --- Error Cases ---

        // Invalid octet
        let err_s = "192.168.1.10-2.256";
        assert!(parse_range_end_addr("2.256", &start, err_s).is_err());

        // Too many octets
        let err_s = "192.168.1.10-1.2.3.4.5";
        assert!(parse_range_end_addr("1.2.3.4.5", &start, err_s).is_err());

        // Empty octets
        let err_s = "192.168.1.10-";
        assert!(parse_range_end_addr("", &start, err_s).is_err());
    }

    #[test]
    fn test_from_str_full_parsing() {
        // Test keywords (case-insensitive)
        assert!(matches!(Target::from_str("lan"), Ok(Target::LAN)));
        assert!(matches!(Target::from_str("VPN"), Ok(Target::VPN)));

        // Test host
        assert!(matches!(
            Target::from_str("1.1.1.1"),
            Ok(Target::Host { .. })
        ));
        assert!(matches!(Target::from_str("::1"), Ok(Target::Host { .. })));

        // Test full range
        assert!(matches!(
            Target::from_str("10.0.0.1-10.0.0.255"),
            Ok(Target::Range { .. })
        ));

        // Test partial range
        assert!(matches!(
            Target::from_str("192.168.1.1-255"),
            Ok(Target::Range { .. })
        ));
        assert!(matches!(
            Target::from_str("192.168.1.1-2.255"),
            Ok(Target::Range { .. })
        ));

        // Test CIDR
        assert!(matches!(
            Target::from_str("10.0.0.0/24"),
            Ok(Target::Range { .. })
        ));

        // Test invalid
        assert!(Target::from_str("not-an-ip").is_err());
        assert!(Target::from_str("10.0.0.1/33").is_err());
        assert!(Target::from_str("10.0.0.256-1.1.1.1").is_err());
    }
}
