use std::collections::HashSet;
use std::net::IpAddr;
use pnet::util::MacAddr;

#[derive(Debug, Clone)]
pub struct EngineHost {
    pub mac: MacAddr,
    pub ips: HashSet<IpAddr>,
    pub hostname: Option<String>,
}

impl EngineHost {
    pub fn new(mac: MacAddr, ip: IpAddr) -> Self {
        let mut ips = HashSet::new();
        ips.insert(ip);
        Self {
            mac,
            ips,
            hostname: None,
        }
    }

    pub fn add_ip(&mut self, ip: IpAddr) {
        self.ips.insert(ip);
    }

    pub fn set_hostname(&mut self, name: String) {
        self.hostname = Some(name);
    }
}
