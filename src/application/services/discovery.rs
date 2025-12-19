use crate::domain::models::host::Host;
use crate::domain::models::target::Target;
use crate::ports::outbound::vendor_repository::VendorRepository;
use crate::ports::outbound::network_scanner::NetworkScanner;

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

    pub async fn perform_discovery(&self, target: Target) -> anyhow::Result<Vec<Box<dyn Host>>> {
        // 1. Delegate "How to scan" to the Adapter
        let mut hosts = self.scanner.scan(target).await?;

        // 2. Enrich with Vendor Data (Domain logic)
        self.enrich_vendors(&mut hosts);

        Ok(hosts)
    }

    fn enrich_vendors(&self, hosts: &mut Vec<Box<dyn Host>>) {
        for host in hosts.iter_mut() {
            if let Some(mac) = host.mac_addr() {
                if let Some(vendor) = self.vendor_repo.get_vendor(mac) {
                    host.set_vendor(vendor);
                }
            }
        }
    }
}
