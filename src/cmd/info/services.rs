use std::{collections::{HashMap, HashSet}, net::IpAddr};

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, get_sockets_info};
use sysinfo::{Pid, System};
use colored::*;

use crate::{cmd::info::GLOBAL_KEY_WIDTH, utils::{colors, print}};


pub struct IpServiceGroup {
    ip_addr: IpAddr,
    tcp_services: Vec<Service>,
    udp_services: Vec<Service>
}

impl IpServiceGroup {
    fn new(ip_addr: IpAddr, tcp_services: Vec<Service>, udp_services: Vec<Service>) -> Self {
        Self {
            ip_addr,
            tcp_services,
            udp_services
        }
    }
}

struct Service {
    name: String,
    local_addr: IpAddr,
    local_ports: HashSet<u16>
}

impl Service {
    fn new(name: String, local_addr: IpAddr, local_ports: HashSet<u16>) -> Self {
        Self {
            name,
            local_addr,
            local_ports,
        }
    }
}

pub fn print_local_services(service_groups: Vec<IpServiceGroup>) -> anyhow::Result<()> {
    print::header("local services");
    
    for (idx, group) in service_groups.iter().enumerate() {
        let ip_addr = group.ip_addr;
        let tcp_services = &group.tcp_services;
        let udp_services = &group.udp_services;

        let has_tcp = !tcp_services.is_empty();
        let has_udp = !udp_services.is_empty();

        if !has_tcp && !has_udp {
            continue;
        }

        // Print IP Address Header
        let ip_addr_colored = if ip_addr.is_ipv4() {
            ip_addr.to_string().color(colors::IPV4_ADDR)
        } else {
            ip_addr.to_string().color(colors::IPV6_ADDR)
        };
        print::println(format!("{}", format!("[{}]", ip_addr_colored).color(colors::SEPARATOR)).as_str());

        // Print TCP Services
        if has_tcp {
            let tcp_branch = if has_udp { "├─" } else { "└─" };
            let vertical_branch = if has_udp { "│" } else { " " };
            print::println(format!(" {} {}", tcp_branch.color(colors::SEPARATOR), "TCP".color(colors::PRIMARY)).as_str());

            for (i, service) in tcp_services.iter().enumerate() {
                print_service_line(i, service, vertical_branch, tcp_services.len());
            }
        }

        // Print UDP Services
        if has_udp {
            let udp_branch = "└─"; // UDP is always the last branch if it exists
            let vertical_branch = " "; // No vertical (│) line needed below UDP
            print::println(format!(" {} {}", udp_branch.color(colors::SEPARATOR), "UDP".color(colors::PRIMARY)).as_str());

            for (i, service) in udp_services.iter().enumerate() {
                print_service_line(i, service, vertical_branch, udp_services.len())
            }
        }

        if idx + 1 != service_groups.len() { print::println(""); }
    }
    Ok(())
}

fn print_service_line(idx: usize, service: &Service, vertical_branch: &str, services_len: usize) {
    let last: bool = idx + 1 == services_len;
    let branch: ColoredString = if last { "└─".color(colors::SEPARATOR) } else { "├─".color(colors::SEPARATOR) };
    let dashes: usize = GLOBAL_KEY_WIDTH.get() - service.name.len() - 5;

    let num_ports = service.local_ports.len();

    let mut port_strings: Vec<String> = service.local_ports.iter()
        .take(5)
        .map(|p| p.to_string())
        .collect();

    if num_ports > 5 { port_strings.push("...".to_string()); }
    let ports: String = port_strings.join(", ");

    let output: String = format!(" {}   {branch} {}{}{}{}",
        vertical_branch.color(colors::SEPARATOR),
        service.name.color(colors::SECONDARY),
        ".".repeat(dashes).color(colors::SEPARATOR),
        ": ".color(colors::SEPARATOR),
        ports.color(colors::TEXT_DEFAULT)
    );
    print::println(&output);
}

pub fn build_socket_maps() -> anyhow::Result<(Vec<IpServiceGroup>, usize)> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let sockets_info = get_sockets_info(af_flags, proto_flags)?;
    let sys = System::new_all();
    let mut longest_name: usize = 0;

    let mut tcp_service_map: HashMap<(String, IpAddr), Service> = HashMap::new(); 
    let mut udp_service_map: HashMap<(String, IpAddr), Service> = HashMap::new();

    for si in sockets_info {
        if let Some(&pid) = si.associated_pids.get(0) {
            if let Some(process) = sys.process(Pid::from_u32(pid)) {
                let process_name = process.name().to_string_lossy().to_string();
                if  process_name.len() > longest_name { longest_name = process_name.len() }
                let local_addr: IpAddr = si.local_addr();
                let local_port: u16 = si.local_port();
                let local_ports: HashSet<u16> = HashSet::new();
                let new_service = Service::new(process_name.clone(), local_addr, local_ports);
                match si.protocol_socket_info {
                    ProtocolSocketInfo::Tcp(_) => {
                        let tcp_service_entry = tcp_service_map
                            .entry((process_name, local_addr))
                            .or_insert_with(|| { new_service });
                        tcp_service_entry.local_ports.insert(local_port);
                    },
                    ProtocolSocketInfo::Udp(_) => {
                        let udp_service_entry = udp_service_map
                            .entry((process_name.clone(), local_addr))
                            .or_insert_with(|| { new_service });
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

    Ok((service_groups, longest_name))
}