use std::env;
use std::net::IpAddr;
use anyhow;
use colored::*;
use is_root::is_root;
use sys_info;

use crate::GLOBAL_KEY_WIDTH;
use crate::net::ip::{self, Ipv6AddressType};
use crate::{print, utils::colors, SPINNER};
use crate::net::datalink::interface;

mod services;
mod firewall;

pub fn info() -> anyhow::Result<()>{
    print::println(format!("{}",
        "Mappr is a quick tool for mapping and exploring networks.".color(colors::TEXT_DEFAULT)).as_str());
    print::println("");
    GLOBAL_KEY_WIDTH.set(10);
    if !is_root() {
        print_about_the_tool();
        print_local_system()?;
        print_network_interfaces();
        print::end_of_program();
        SPINNER.finish_and_clear();
        return Ok(())
    }

    let (service_groups, longest_name) = services::build_socket_maps()?;
    GLOBAL_KEY_WIDTH.set(longest_name + 6);

    print_about_the_tool();
    print_local_system()?;
    services::print_local_services(service_groups)?;
    firewall::print_firewall_status()?;
    print_network_interfaces();

    print::end_of_program();
    SPINNER.finish_and_clear();
    Ok(())
}

fn print_about_the_tool() {
    print::aligned_line("Version", env!("CARGO_PKG_VERSION"));
    print::aligned_line("Author", "hollowpointer");
    print::aligned_line("E-Mail", "hollowpointer@pm.me");
    print::aligned_line("License", "MIT");
    print::aligned_line("Repository", "https://github.com/hollowpointer/mappr");
}

fn print_local_system() -> anyhow::Result<()> {
    print::header("local system");
    let hostname: String = sys_info::hostname()?;
    print::aligned_line("Hostname", hostname);
    let release = sys_info::os_release().unwrap_or_else(|_| { String::from("") });
    let os_name = sys_info::os_type()?;
    print::aligned_line("OS", format!("{} {}", os_name, release).as_str());
    if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
        print::aligned_line("User", user);
    }
    Ok(())
}

fn print_network_interfaces() {
    print::header("network interfaces");
    let interfaces = interface::get_unique_interfaces(5)
        .expect("Failed to get interfaces");
    for (idx, intf) in interfaces.iter().enumerate() {
        print::println(format!("{} {}", format!("[{}]", idx.to_string().color(colors::ACCENT))
            .color(colors::SEPARATOR), intf.name.color(colors::PRIMARY)).as_str());

        let mut lines: Vec<(ColoredString, ColoredString)> = Vec::new();

        for ipv4_addr in &intf.ipv4_addr {
            let address: ColoredString = ipv4_addr.ip_addr.to_string().color(colors::IPV4_ADDR);
            let prefix: ColoredString = ipv4_addr.prefix.to_string().color(colors::IPV4_PREFIX);
            let result: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR); 
            lines.push(("IPv4".color(colors::TEXT_DEFAULT), result));
        }

        for ipv6_addr in &intf.ipv6_addr {
            let address: ColoredString = ipv6_addr.ip_addr.to_string().color(colors::IPV6_ADDR);
            let prefix: ColoredString = ipv6_addr.prefix.to_string().color(colors::IPV6_PREFIX);
            let result: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR);
            let ipv6_type = match ipv6_addr.ip_addr {
                IpAddr::V4(_) => panic!("This should never panic."),
                IpAddr::V6(ipv6_addr) => ip::get_ipv6_type(&ipv6_addr),
            };
            let key = match ipv6_type {
                Ipv6AddressType::GlobalUnicast  => "GUA",
                Ipv6AddressType::LinkLocal      => "LLA",
                Ipv6AddressType::UniqueLocal    => "ULA",
                _                               => "IPv6"
            };
            lines.push((key.color(colors::TEXT_DEFAULT), result));
        }

        if let Some(mac_addr) = intf.mac_addr {
            lines.push(("MAC".color(colors::TEXT_DEFAULT), mac_addr.to_string().color(colors::MAC_ADDR)));
        }
        
        for(i, (key, value)) in lines.iter().enumerate() {
            let last = i + 1 == lines.len();
            let branch = if last { "└─".color(colors::SEPARATOR) } else { "├─".color(colors::SEPARATOR) };
            let dots = ".".repeat(GLOBAL_KEY_WIDTH.get() - key.len() - 1);
            let colon = format!("{}{}", dots.color(colors::SEPARATOR), ":".color(colors::SEPARATOR));
            let output = format!(" {branch} {}{} {}", key, colon, value);
            print::println(&output)
        }
        if idx + 1 != interfaces.len() { print::println(""); }
    }
}