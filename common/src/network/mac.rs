use std::sync::OnceLock;
use mac_oui::Oui;
use pnet::util::MacAddr;

static OUI_DB: OnceLock<Oui> = OnceLock::new();

/// Retrieves or initializes the **Organizationally unique identifier** database.
/// 
/// Used for linking a vendor to a MAC address (LAN)
fn get_oui_db() -> &'static Oui {
    OUI_DB.get_or_init(|| {
        Oui::default().expect("failed to load OUI database")
    })
}

/// Identify the vendor of a MAC address.
pub fn get_vendor(mac: MacAddr) -> Option<String> {
    let db = get_oui_db();
    let mac_str = mac.to_string();
    match db.lookup_by_mac(&mac_str) {
        Ok(Some(entry)) => Some(entry.company_name.clone()),
        _ => None,
    }
}