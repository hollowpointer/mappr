use std::env;
use std::net::Ipv4Addr;
use anyhow;
use colored::*;
use sys_info;
use crate::{print, SPINNER};
use crate::net::datalink::interface;

const LENGTH_OF_LONGEST_WORD: usize = 10;

pub fn info() -> anyhow::Result<()>{
    print::println("Mappr is a quick tool for mapping and exploring networks.");
    print::println("");
    print_about_the_tool();
    print_local_system()?;
    print_network_interfaces();

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
    print::separator("network configuration");
    let interfaces = pnet::datalink::interfaces();
    for (idx, intf) in interfaces.iter().enumerate() {
        print::println(format!("{} {}", format!("[{idx}]").green(), intf.name.green()).as_str());
        let ipv4 = interface::get_ipv4(intf).unwrap_or(Ipv4Addr::new(0,0,0,0));
        print_intf_line("IPv4", &ipv4.to_string());
        if let Some(lla) = interface::get_link_local_addr(intf) { print_intf_line("LLA", &lla.to_string()); }
        if let Some(mac) = intf.mac { print_intf_line("MAC", &mac.to_string()); }
        SPINNER.println(format!("{}", "------------------------------------------------------------".bright_black()));
    }
}

fn print_info_line(key: &str, value: &str) {
    let whitespace = " ".repeat(LENGTH_OF_LONGEST_WORD - key.len());
    let colon = format!("{}{}", whitespace, ":".bright_black());
    print::print_status(format!("{} {} {}", key.yellow(), colon, value).as_str());
}

fn print_intf_line(key: &str, value: &str) {
    let whitespace = " ".repeat(LENGTH_OF_LONGEST_WORD - key.len() - 1);
    let colon = format!("{}{}", whitespace, ":".bright_black());
    print::println(format!("{} {}{} {}", " ├─".bright_black(), key.yellow(), colon, value).as_str())
}