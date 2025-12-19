use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    process::Command,
};

use anyhow;

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, get_sockets_info};
use sysinfo::{Pid, System};
use pnet::datalink::NetworkInterface;

use crate::domain::models::localhost::{IpServiceGroup, Service, FirewallStatus};
use crate::ports::outbound::system_repository::SystemRepository;

pub struct SystemRepo;

impl SystemRepository for SystemRepo {
    fn get_local_services(&self) -> anyhow::Result<Vec<IpServiceGroup>> {
        let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
        let sockets_info = get_sockets_info(af_flags, proto_flags)?;
        let sys = System::new_all();

        let mut tcp_service_map: HashMap<(String, IpAddr), Service> = HashMap::new();
        let mut udp_service_map: HashMap<(String, IpAddr), Service> = HashMap::new();

        for si in sockets_info {
            if let Some(&pid) = si.associated_pids.get(0) {
                if let Some(process) = sys.process(Pid::from_u32(pid)) {
                    let process_name = process.name().to_string_lossy().to_string();
                    
                    let local_addr: IpAddr = si.local_addr();
                    let local_port: u16 = si.local_port();
                    let local_ports: HashSet<u16> = HashSet::new();
                    let new_service = Service::new(process_name.clone(), local_addr, local_ports);
                    
                    match si.protocol_socket_info {
                        ProtocolSocketInfo::Tcp(_) => {
                            let tcp_service_entry = tcp_service_map
                                .entry((process_name, local_addr))
                                .or_insert_with(|| new_service);
                            tcp_service_entry.local_ports.insert(local_port);
                        }
                        ProtocolSocketInfo::Udp(_) => {
                            let udp_service_entry = udp_service_map
                                .entry((process_name.clone(), local_addr))
                                .or_insert_with(|| new_service);
                            udp_service_entry.local_ports.insert(local_port);
                        }
                    }
                }
            }
        }

        let mut tcp_ip_map: HashMap<IpAddr, Vec<Service>> = HashMap::new();
        for service in tcp_service_map.into_values() {
            tcp_ip_map
                .entry(service.local_addr)
                .or_default()
                .push(service);
        }
        let mut udp_ip_map: HashMap<IpAddr, Vec<Service>> = HashMap::new();
        for service in udp_service_map.into_values() {
            udp_ip_map
                .entry(service.local_addr)
                .or_default()
                .push(service);
        }

        let mut all_ips: HashSet<IpAddr> = tcp_ip_map.keys().cloned().collect();
        all_ips.extend(udp_ip_map.keys().cloned());

        let mut service_groups: Vec<IpServiceGroup> = Vec::new();

        for ip in all_ips {
            let tcp_procs = tcp_ip_map.remove(&ip).unwrap_or_default();
            let udp_procs = udp_ip_map.remove(&ip).unwrap_or_default();
            service_groups.push(IpServiceGroup::new(ip, tcp_procs, udp_procs));
        }

        service_groups.sort_by_key(|sm| sm.ip_addr);

        Ok(service_groups)
    }

    fn get_firewall_status(&self) -> anyhow::Result<FirewallStatus> {
        #[cfg(target_os = "linux")]
        {
            let ufw_active = Command::new("ufw").arg("status").output().is_ok();
            let firewalld_active = Command::new("firewall-cmd").arg("--state").output().is_ok();

            if ufw_active || firewalld_active {
                Ok(FirewallStatus::Active)
            } else {
                Ok(FirewallStatus::NotDetected) 
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(FirewallStatus::NotDetected)
        }
    }

    fn get_network_interfaces(&self) -> anyhow::Result<Vec<NetworkInterface>> {
        crate::engine::scanner::get_prioritized_interfaces()
    }
}
