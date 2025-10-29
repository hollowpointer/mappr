use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::net::IpAddr;
use anyhow::{self};
use colored::*;
use is_root::is_root;
use netstat2::*;
use sys_info;
use sysinfo::{Pid, System};

use crate::{print, utils::colors, SPINNER};
use crate::net::datalink::interface;

thread_local! {
    static GLOBAL_KEY_WIDTH: Cell<usize> = Cell::new(0);
}

struct SocketMap {
    ip_addr: IpAddr,
    tcp_processes: Vec<Process>,
    udp_processes: Vec<Process>
}

impl SocketMap {
    fn new(ip_addr: IpAddr, tcp_processes: Vec<Process>, udp_processes: Vec<Process>) -> Self {
        Self {
            ip_addr,
            tcp_processes,
            udp_processes
        }
    }
}

struct Process {
    name: String,
    local_addr: IpAddr,
    local_ports: HashSet<u16>
}

impl Process {
    fn new(name: String, local_addr: IpAddr, local_ports: HashSet<u16>) -> Self {
        Self {
            name,
            local_addr,
            local_ports,
        }
    }
}

pub fn info() -> anyhow::Result<()>{
    print::println(format!("{}",
        "Mappr is a quick tool for mapping and exploring networks.".color(colors::TEXT_DEFAULT)).as_str());
    print::println("");
    GLOBAL_KEY_WIDTH.set(10);
    if !is_root() {
        print_about_the_tool();
        print_local_system()?;
        print_network_interfaces();
        return Ok(())
    }
    let (socket_maps, longest_name) = handle_local_services()?;
    GLOBAL_KEY_WIDTH.set(longest_name + 6);
    print_about_the_tool();
    print_local_system()?;
    print_local_services(socket_maps)?;
    print_firewall_status();
    print_network_interfaces();

    print::end_of_program();
    SPINNER.finish_and_clear();
    Ok(())
}

fn print_about_the_tool() {
    print_info_line("Version", env!("CARGO_PKG_VERSION"));
    print_info_line("Author", "hollowpointer");
    print_info_line("E-Mail", "hollowpointer@pm.me");
    print_info_line("License", "MIT");
    print_info_line("Repository", "https://github.com/hollowpointer/mappr");
}

fn print_local_system() -> anyhow::Result<()> {
    print::separator("local system");
    let hostname: String = sys_info::hostname()?;
    print_info_line("Hostname", &hostname);
    let release = sys_info::os_release().unwrap_or_else(|_| { String::from("") });
    let os_name = sys_info::os_type()?;
    print_info_line("OS", format!("{} {}", os_name, release).as_str());
    if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
        print_info_line("User", &user);
    }
    Ok(())
}

fn print_network_interfaces() {
    print::separator("network interfaces");
    let interfaces = interface::get_unique_interfaces(3)
        .expect("Failed to get interfaces");

    for (idx, intf) in interfaces.iter().enumerate() {
        let mut lines: Vec<(ColoredString, ColoredString)> = Vec::new();
        print::println(format!("{} {}", format!("[{}]", idx.to_string().color(colors::ACCENT))
            .color(colors::SEPARATOR), intf.name.color(colors::PRIMARY)).as_str());

        if let Ok(Some(ipv4_addr)) = interface::get_ipv4(intf) {
            if let Ok(Some(prefix)) = interface::get_prefix(intf) {
                let value: ColoredString = ColoredString::from(
                 format!(
                    "{}{}{}",
                    ipv4_addr.to_string().color(colors::IPV4_ADDR),
                    "/".color(colors::SEPARATOR),
                    prefix.to_string().color(colors::IPV4_PREFIX)
                ));
                lines.push(("IPv4".color(colors::TEXT_DEFAULT), value));
            }
        }

        if let Some(lla) = interface::get_link_local_addr(intf) {
            lines.push(("LLA".color(colors::TEXT_DEFAULT), lla.to_string().color(colors::IPV6_ADDR)));
        }

        if let Some(mac) = intf.mac {
            lines.push(("MAC".color(colors::TEXT_DEFAULT), mac.to_string().color(colors::MAC_ADDR)));
        }
        
        for(i, (key, value)) in lines.iter().enumerate() {
            let last = i + 1 == lines.len();
            let branch = if last { "└─".color(colors::SEPARATOR) } else { "├─".color(colors::SEPARATOR) };
            let whitespace = ".".repeat(GLOBAL_KEY_WIDTH.get() - key.len() - 1);
            let colon = format!("{}{}", whitespace.color(colors::SEPARATOR), ":".color(colors::SEPARATOR));
            let output = format!(" {branch} {}{} {}", key, colon, value);
            print::println(&output)
        }
        if idx + 1 != interfaces.len() { print::println(""); }
    }
}

fn print_local_services(socket_maps: Vec<SocketMap>) -> anyhow::Result<()> {
    print::separator("local services");
    
    for (idx, socket_map) in socket_maps.iter().enumerate() {
        let ip_addr = socket_map.ip_addr;
        let tcp_processes = &socket_map.tcp_processes;
        let udp_processes = &socket_map.udp_processes;

        let has_tcp = !tcp_processes.is_empty();
        let has_udp = !udp_processes.is_empty();

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

        // Print TCP Processes
        if has_tcp {
            let tcp_branch = if has_udp { "├─" } else { "└─" };
            let vertical_branch = if has_udp { "│" } else { " " };
            print::println(format!(" {} {}", tcp_branch.color(colors::SEPARATOR), "TCP".color(colors::PRIMARY)).as_str());

            for (i, process) in tcp_processes.iter().enumerate() {
                print_process(i, process, vertical_branch, tcp_processes.len());
            }
        }

        // Print UDP Processes
        if has_udp {
            let udp_branch = "└─"; // UDP is always the last branch if it exists
            let vertical_branch = " "; // No vertical (│) line needed below UDP
            print::println(format!(" {} {}", udp_branch.color(colors::SEPARATOR), "UDP".color(colors::PRIMARY)).as_str());

            for (i, process) in udp_processes.iter().enumerate() {
                print_process(i, process, vertical_branch, udp_processes.len())
            }
        }

        if idx + 1 != socket_maps.len() { print::println(""); }
    }
    Ok(())
}

fn print_process(idx: usize, process: &Process, vertical_branch: &str, processes_len: usize) {
    let last: bool = idx + 1 == processes_len;
    let branch: ColoredString = if last { "└─".color(colors::SEPARATOR) } else { "├─".color(colors::SEPARATOR) };
    let dashes: usize = GLOBAL_KEY_WIDTH.get() - process.name.len() - 5;

    let num_ports = process.local_ports.len();

    let mut port_strings: Vec<String> = process.local_ports.iter()
        .take(5)
        .map(|p| p.to_string())
        .collect();

    if num_ports > 5 { port_strings.push("...".to_string()); }
    let ports: String = port_strings.join(", ");

    let output: String = format!(" {}   {branch} {}{}{}{}",
        vertical_branch.color(colors::SEPARATOR),
        process.name.color(colors::SECONDARY),
        ".".repeat(dashes).color(colors::SEPARATOR),
        ": ".color(colors::SEPARATOR),
        ports.color(colors::TEXT_DEFAULT)
    );
    print::println(&output);
}


fn print_firewall_status() {

}

fn handle_local_services() -> anyhow::Result<(Vec<SocketMap>, usize)> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let sockets_info = get_sockets_info(af_flags, proto_flags)?;
    let sys = System::new_all();
    let mut longest_name: usize = 0;

    let mut tcp_process_map: HashMap<(String, IpAddr), Process> = HashMap::new();
    let mut udp_process_map: HashMap<(String, IpAddr), Process> = HashMap::new();

    for si in sockets_info {
        if let Some(&pid) = si.associated_pids.get(0) {
            if let Some(process) = sys.process(Pid::from_u32(pid)) {
                let process_name = process.name().to_string_lossy().to_string();
                if  process_name.len() > longest_name { longest_name = process_name.len() }
                let local_addr: IpAddr = si.local_addr();
                let local_port: u16 = si.local_port();
                let local_ports: HashSet<u16> = HashSet::new();
                let new_process = Process::new(process_name.clone(), local_addr, local_ports);
                match si.protocol_socket_info {
                    ProtocolSocketInfo::Tcp(_) => {
                        let tcp_process_entry = tcp_process_map
                            .entry((process_name, local_addr))
                            .or_insert_with(|| { new_process });
                        tcp_process_entry.local_ports.insert(local_port);
                    },
                    ProtocolSocketInfo::Udp(_) => {
                        let udp_process_map = udp_process_map
                            .entry((process_name.clone(), local_addr))
                            .or_insert_with(|| { new_process });
                        udp_process_map.local_ports.insert(local_port);
                    }
                }
            }
        }
    }

    let mut tcp_socket_map: HashMap<IpAddr, Vec<Process>> = HashMap::new();
    for process in tcp_process_map.into_values() {
        tcp_socket_map
            .entry(process.local_addr)
            .or_default()
            .push(process);
    }
    let mut udp_socket_map: HashMap<IpAddr, Vec<Process>> = HashMap::new();
    for process in udp_process_map.into_values() {
        udp_socket_map
            .entry(process.local_addr)
            .or_default()
            .push(process);
    }

    let mut all_ips: HashSet<IpAddr> = tcp_socket_map.keys().cloned().collect();
    all_ips.extend(udp_socket_map.keys().cloned());

    let mut socket_maps: Vec<SocketMap> = Vec::new();

    for ip in all_ips {
        let tcp_procs = tcp_socket_map.remove(&ip).unwrap_or_default();
        let udp_procs = udp_socket_map.remove(&ip).unwrap_or_default();
        socket_maps.push(SocketMap::new(ip, tcp_procs, udp_procs));
    }

    socket_maps.sort_by_key(|sm| sm.ip_addr);

    Ok((socket_maps, longest_name))
}

fn print_info_line(key: &str, value: &str) {
    let whitespace = ".".repeat(GLOBAL_KEY_WIDTH.get() + 1 - key.len());
    let colon = format!("{}{}", whitespace.color(colors::SEPARATOR), ":".color(colors::SEPARATOR));
    print::print_status(format!("{}{} {}", key.color(colors::PRIMARY), colon, value.color(colors::TEXT_DEFAULT)).as_str());
}