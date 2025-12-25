use pnet::datalink::MacAddr;

/// Defines the contract for resolving device manufacturers from MAC addresses.
pub trait VendorRepository {
    /// Retrieves the vendor name for a given MAC address.
    ///
    /// # Arguments
    /// * `mac_addr` - The MAC address to lookup.
    ///
    /// # Returns
    /// * `Some(String)` - The name of the vendor if found.
    /// * `None` - If the OUI is unknown.
    fn get_vendor(&self, mac_addr: MacAddr) -> Option<String>;
}