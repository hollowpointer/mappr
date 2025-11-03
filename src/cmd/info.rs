use std::env;
use anyhow::{self};
use colored::*;
use is_root::is_root;
use sys_info;

use crate::GLOBAL_KEY_WIDTH;
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

        if let Some(loop_back) = interface::get_loop_back_addr(intf) {
            lines.push(("IPv6".color(colors::TEXT_DEFAULT), loop_back.to_string().color(colors::IPV6_ADDR)));
        }

        if let Some(gua) = interface::get_global_unicast_addr(intf) {
            lines.push(("GUA".color(colors::TEXT_DEFAULT), gua.to_string().color(colors::IPV6_ADDR)));
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
            let dots = ".".repeat(GLOBAL_KEY_WIDTH.get() - key.len() - 1);
            let colon = format!("{}{}", dots.color(colors::SEPARATOR), ":".color(colors::SEPARATOR));
            let output = format!(" {branch} {}{} {}", key, colon, value);
            print::println(&output)
        }
        if idx + 1 != interfaces.len() { print::println(""); }
    }
}