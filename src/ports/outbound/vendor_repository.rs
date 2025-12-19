use pnet::datalink::MacAddr;

pub trait VendorRepository {
    fn get_vendor(&self, mac_addr: MacAddr) -> Option<String>;
}