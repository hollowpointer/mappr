//! # Network Discovery Service
//!
//! Implements the core "Network Scan" use case.
//!
//! This service is responsible for finding devices on the network and aggregating
//! information about them from various sources (Network Scanner, Vendor Repository).

use mappr_common::network::host::Host;
use mappr_common::network::target::Target;
use mappr_common::vendors::VendorRepository;
use mappr_common::scanning::NetworkScanner;

/// Application Service for Network Discovery.
///
/// Orchestrates the discovery process by:
/// 1. delegating the raw network scan to the [`NetworkScanner`] trait.
/// 2. enriching the results with additional data (e.g., Vendor lookups).
pub struct DiscoveryService {
    vendor_repo: Box<dyn VendorRepository>,
    scanner: Box<dyn NetworkScanner>,
}

impl DiscoveryService {
    pub fn new(
        vendor_repo: Box<dyn VendorRepository>,
        scanner: Box<dyn NetworkScanner>,
    ) -> Self {
        Self {
            vendor_repo,
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
        let mut hosts = self.scanner.scan(target).await?;

        // 2. Enrich with Vendor Data (Domain logic)
        self.enrich_vendors(&mut hosts);

        Ok(hosts)
    }

    fn enrich_vendors(&self, hosts: &mut Vec<Host>) {
        for host in hosts.iter_mut() {
            if let Some(mac) = host.mac {
                if let Some(vendor) = self.vendor_repo.get_vendor(mac) {
                    host.vendor = Some(vendor);
                }
            }
        }
    }
}
