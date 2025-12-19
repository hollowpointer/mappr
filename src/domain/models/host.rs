//! # Host Domain Model
//!
//! This module defines the [`Host`] entity, which represents a single network device detected during a scan.
//!
//! ## Key Concepts
//! * **Unified Model**: A `Host` represents both devices on the local LAN (Layer 2) and remote devices (Layer 3).
//! * **Identity**: A host is primarily identified by its IP address for the duration of a scan.
//! * **Enrichment**: The model is mutable and strictly additive; scans populate optional fields (hostname, vendor) as data becomes available.
use pnet::datalink::MacAddr;
use std::{
    collections::{BTreeSet, HashSet},
    net::IpAddr,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NetworkRole {
    _Gateway,
    _DHCP,
    _DNS,
}

/// Represents a discovered network host.
///
/// A host is defined by what we know about it.
#[derive(Debug, Clone)]
pub struct Host {
    /// The primary way to identify the host (on this run).
    /// Note: A host might have multiple IPs, but we usually discover it via one.
    pub ip: IpAddr,
    
    /// The resolved hostname (if any).
    pub hostname: Option<String>,
    
    /// All known IP addresses for this host.
    pub ips: BTreeSet<IpAddr>,
    
    /// Open ports found on the host.
    /// TODO: Refactor to a rich `Port` struct in a future iteration.
    pub ports: BTreeSet<u16>,
    
    /// The MAC address (only available if the host is on the same LAN).
    pub mac: Option<MacAddr>,
    
    /// The device vendor/manufacturer (derived from MAC).
    pub vendor: Option<String>,
    
    /// Inferred network roles (e.g., is it a Gateway?).
    pub network_roles: HashSet<NetworkRole>,
}

impl Host {
    /// Creates a new Host with minimal information (just an IP).
    pub fn new(ip: IpAddr) -> Self {
        Self {
            ip,
            hostname: None,
            ips: BTreeSet::from([ip]),
            ports: BTreeSet::new(),
            mac: None,
            vendor: None,
            network_roles: HashSet::new(),
        }
    }

    /// Sets the MAC address.
    pub fn with_mac(mut self, mac: MacAddr) -> Self {
        self.mac = Some(mac);
        self
    }
}
