//! # Network Discovery Service
//!
//! Implements the core "Network Scan" use case.
//!
//! This service is responsible for finding devices on the network and aggregating
//! information about them from various sources (Network Scanner, Vendor Repository).

use mappr_common::network::host::Host;
use mappr_common::network::target::Target;
use mappr_common::scanning::NetworkScanner;

/// Application Service for Network Discovery.
///
/// Orchestrates the discovery process by:
/// 1. delegating the raw network scan to the [`NetworkScanner`] trait.
/// 2. enriching the results with additional data (e.g., Vendor lookups).
pub struct DiscoveryService {
    scanner: Box<dyn NetworkScanner>,
}

impl DiscoveryService {
    pub fn new(
        scanner: Box<dyn NetworkScanner>,
    ) -> Self {
        Self {
            scanner,
        }
    }

    /// Executes a network scan against the specified `target`.
    ///
    /// The process involves:
    /// 1. **Scanning**: Using the underlying network adapter to find hosts.
    /// 2. **Enrichment**: Resolving MAC addresses to Vendor names.
    pub async fn perform_discovery(&self, target: Target) -> anyhow::Result<Vec<Host>> {
        // 1. Delegate "How to scan" to the implementations
        let hosts = self.scanner.scan(target).await?;

        Ok(hosts)
    }
}
