use std::env;
use anyhow::{self};
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
        let mut lines: Vec<(&str, ColoredString)> = Vec::new();
        print::println(format!("{} {}", format!("[{idx}]").green(), intf.name.green()).as_str());
        match interface::get_ipv4(intf) {
            Ok(ipv4) => lines.push(("IPv4", ipv4.unwrap().to_string().truecolor(83, 179, 203))),
            _ => { }
        }
        if let Some(lla) = interface::get_link_local_addr(intf) {
            lines.push(("LLA", lla.to_string().magenta())); 
        }
        if let Some(mac) = intf.mac {
            lines.push(("MAC", mac.to_string().truecolor(255, 176, 0))); 
        }
        for(i, (key, value)) in lines.iter().enumerate() {
            let last = i + 1 == lines.len();
            let branch = if last { "└─".bright_black() } else { "├─".bright_black() };
            let whitespace = " ".repeat(LENGTH_OF_LONGEST_WORD - key.len() - 1);
            let colon = format!("{}{}", whitespace, ":".bright_black());
            let output = format!(" {branch} {}{} {}", key, colon, value);
            print::println(&output)
        }
        SPINNER.println(format!("{}", "------------------------------------------------------------".bright_black()));
    }
}

fn print_info_line(key: &str, value: &str) {
    let whitespace = " ".repeat(LENGTH_OF_LONGEST_WORD - key.len());
    let colon = format!("{}{}", whitespace, ":".bright_black());
    print::print_status(format!("{} {} {}", key.yellow(), colon, value).as_str());
}