use std::sync::OnceLock;
use mac_oui::Oui;
use pnet::datalink::MacAddr;
use mappr_common::vendors::VendorRepository;

static OUI_DB: OnceLock<Oui> = OnceLock::new();

fn get_oui_db() -> &'static Oui {
    OUI_DB.get_or_init(|| {
        Oui::default().expect("failed to load OUI database")
    })
}

pub struct MacOuiRepo;

impl VendorRepository for MacOuiRepo {
    fn get_vendor(&self, mac: MacAddr) -> Option<String> {
        let db = get_oui_db();
        let mac_str = mac.to_string();
        match db.lookup_by_mac(&mac_str) {
            Ok(Some(entry)) => Some(entry.company_name.clone()),
            _ => None,
        }
    }
}