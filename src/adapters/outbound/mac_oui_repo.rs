use std::sync::OnceLock;
use mac_oui::Oui;
use pnet::datalink::MacAddr;
use crate::ports::outbound::vendor_repository::VendorRepository;

static OUI_DB: OnceLock<Oui> = OnceLock::new();

fn get_oui_db() -> &'static Oui {
    OUI_DB.get_or_init(|| {
        Oui::default().expect("failed to load OUI database")
    })
}

pub struct MacOuiRepo;

impl VendorRepository for MacOuiRepo {
    fn get_vendor(&self, mac_addr: MacAddr) -> Option<String> {
        let oui_db: &Oui = get_oui_db();
        match oui_db.lookup_by_mac(&mac_addr.to_string()) {
            Ok(Some(entry)) => Some(entry.company_name.clone()),
            Ok(None) => None,
            Err(_) => None
        }
    }
}