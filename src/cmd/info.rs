use std::env;
use std::net::Ipv4Addr;
use colored::Colorize;
use sys_info;
use crate::{print, SPINNER};
use crate::net::datalink::interface;

pub fn info() -> anyhow::Result<()>{
    print::println("Mappr is a quick tool for mapping and exploring networks.");
    print::println("");
    print::print_status(format!("Version    : {}", env!("CARGO_PKG_VERSION")).as_str());
    print::print_status("Author     : hollowpointer");
    print::print_status("E-Mail     : hollowpointer@pm.me");
    print::print_status("License    : MIT");
    print::print_status("Repository : https://github.com/hollowpointer/mappr");

    print::separator("local system");
    let hostname: String = sys_info::hostname()?;
    print::print_status(format!("Hostname   : {hostname}").as_str());
    let release = sys_info::os_release().unwrap_or_else(|_| { String::from("") });
    let os_name = sys_info::os_type()?;
    print::print_status(format!("OS         : {os_name} {release}").as_str());
    if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
        print::print_status(format!("User       : {user}").as_str());
    }

    print::separator("network configuration");
    print_network_interfaces();
    SPINNER.finish_and_clear();
    Ok(())
}

fn print_network_interfaces() {
    let interfaces = pnet::datalink::interfaces();
    for (idx, intf) in interfaces.iter().enumerate() {
        print::println(format!("\x1b[32m[{idx}] {}", intf.name).as_str());
        match intf.is_up() {
            true => print::println(" ├─ Status   : Connected"),
            false => print::println(" ├─ Status   : Disconnected")
        }
        print::println(format!(" ├─ IPv4     : {:?}", interface::get_ipv4(intf).unwrap_or(
            Ipv4Addr::new(0, 0, 0, 0)
        )).as_str());
        if let Some(lla) = interface::get_link_local_addr(intf) {
            print::println(format!(" ├─ LLA      : {lla}").as_str());
        }
        if let Some(mac) = intf.mac {
            print::println(format!(" └─ MAC      : {mac}").as_str());
        }
        SPINNER.println(format!("{}", "------------------------------------------------------------".bright_black()));
    }
}