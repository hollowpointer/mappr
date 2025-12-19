use crate::domain::models::system::{IpServiceGroup, FirewallStatus};
use pnet::datalink::NetworkInterface;

pub trait SystemRepository {
   fn get_local_services(&self) -> anyhow::Result<Vec<IpServiceGroup>>;
   fn get_firewall_status(&self) -> anyhow::Result<FirewallStatus>;
   fn get_network_interfaces(&self) -> anyhow::Result<Vec<NetworkInterface>>;
}
