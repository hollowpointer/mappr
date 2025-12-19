use std::time::Duration;
use std::time::Instant;
use anyhow;
use colored::*;

use crate::domain::models::target::Target;
use crate::domain::models::host::Host; 
use crate::adapters::outbound::terminal::spinner::get_spinner;
use crate::adapters::outbound::terminal::{
    colors,
    print,
    format,
};

use crate::application::services::discovery::DiscoveryService;
use crate::adapters::outbound::mac_oui_repo::MacOuiRepo;
use crate::adapters::outbound::network::scanner_adapter::NetworkScannerAdapter;

pub async fn discover(target: Target) -> anyhow::Result<()> {
    get_spinner().set_message("Performing discovery...".to_owned());
    print::print_status("Initializing discovery...");

    let start_time: Instant = Instant::now();

    // 1. Instantiate Dependencies
    let vendor_repo = Box::new(MacOuiRepo);
    let scanner_adapter = Box::new(NetworkScannerAdapter);
    let service = DiscoveryService::new(vendor_repo, scanner_adapter);

    // 2. Execute Service
    let mut hosts = service.perform_discovery(target).await?;

    // 3. Present Results
    Ok(discovery_ends(&mut hosts, start_time.elapsed())?)
}


fn discovery_ends(hosts: &mut Vec<Box<dyn Host>>, total_time: Duration) -> anyhow::Result<()> {
    if hosts.is_empty() {
        return Ok(no_hosts_found());
    }
    print::header("Network Discovery");
    hosts.sort_by_key(|host| host.get_primary_ip());
    for (idx, host) in hosts.iter().enumerate() {
        print_host_details(host.as_ref(), idx);
        if idx + 1 != hosts.len() {
            print::println("");
        }
    }
    print::fat_separator();
    let active_hosts: ColoredString = format!("{} active hosts", hosts.len()).bold().green();
    let total_time: ColoredString = format!("{:.2}s", total_time.as_secs_f64()).bold().yellow();
    print::centerln(
        &format!(
            "Discovery Complete: {} identified in {}",
            active_hosts, total_time
        )
        .color(colors::TEXT_DEFAULT),
    );
    print::end_of_program();
    get_spinner().finish_and_clear();
    Ok(())
}

fn no_hosts_found() {
    print::header("ZERO HOSTS DETECTED");
    print::no_results();
    print::end_of_program();
    get_spinner().finish_and_clear();
}

fn print_host_details(host: &dyn Host, idx: usize) {
    print::tree_head(idx, host.hostname());
    let mut key_value_pair = format::ip_to_key_value_pair(host.ips());

    if let Some(mac) = host.mac_addr() {
        let mac_key_value: (String, ColoredString) = (
            "MAC".to_string(),
            mac.to_string().color(colors::MAC_ADDR),
        );
        key_value_pair.push(mac_key_value);
    }

    if let Some(vendor) = host.vendor() {
        let vendor_key_value: (String, ColoredString) = (
            "Vendor".to_string(),
            vendor.to_string().color(colors::MAC_ADDR),
        );
        key_value_pair.push(vendor_key_value);
    }

    if let Some(roles) = host.roles() {
        if !roles.is_empty() {
             let joined_roles: String = roles
                .iter()
                .map(|role| format!("{:?}", role))
                .collect::<Vec<String>>()
                .join(", ");

            let roles_key_value: (String, ColoredString) =
                ("Roles".to_string(), joined_roles.normal());

            key_value_pair.push(roles_key_value);
        }
    }

    print::as_tree_one_level(key_value_pair);
}
