use anyhow;
use colored::*;
use is_root::is_root;
use pnet::datalink::NetworkInterface;
use std::env;
use sys_info;

use crate::GLOBAL_KEY_WIDTH;
use crate::net::datalink::interface::{self, NetworkInterfaceExtension};
use crate::{
    print::{self, SPINNER},
    utils::colors,
};

mod firewall;
mod services;

pub fn info() -> anyhow::Result<()> {
    print::println(
        format!(
            "{}",
            "Mappr is a quick tool for mapping and exploring networks.".color(colors::TEXT_DEFAULT)
        )
        .as_str(),
    );
    print::println("");
    GLOBAL_KEY_WIDTH.set(10);
    if !is_root() {
        print_about_the_tool();
        print_local_system()?;
        print_network_interfaces()?;
        print::end_of_program();
        SPINNER.finish_and_clear();
        return Ok(());
    }

    let (service_groups, longest_name) = services::build_socket_maps()?;
    GLOBAL_KEY_WIDTH.set(longest_name + 6);

    print_about_the_tool();
    print_local_system()?;
    firewall::print_firewall_status()?;
    services::print_local_services(service_groups)?;
    print_network_interfaces()?;

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
    let release = sys_info::os_release().unwrap_or_else(|_| String::from(""));
    let os_name = sys_info::os_type()?;
    print::aligned_line("OS", format!("{} {}", os_name, release).as_str());
    if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
        print::aligned_line("User", user);
    }
    Ok(())
}

fn print_network_interfaces() -> anyhow::Result<()> {
    print::header("network interfaces");
    let interfaces: Vec<NetworkInterface> = interface::get_prioritized_interfaces(5)?;
    for (idx, intf) in interfaces.iter().enumerate() {
        intf.print_details(idx);
        if idx + 1 != interfaces.len() {
            print::println("");
        }
    }
    Ok(())
}
