use anyhow;

#[cfg(target_os = "linux")]
pub fn print_firewall_status() -> anyhow::Result<()> {
    use crate::utils::{colors, print};
    use colored::Colorize;
    use std::process::Command;

    print::header("firewall status");

    let mut has_ufw: bool = false;
    let mut has_firwalld: bool = false;

    let ufw_output = Command::new("ufw").arg("status").output();
    let ufw_activity = match ufw_output {
        Ok(_) => {
            has_ufw = true;
            "active".green().bold()
        }
        Err(_) => "inactive".red().bold(),
    };

    let firewalld_output = Command::new("firewall-cmd").arg("--state").output();
    let firewalld_activity = match firewalld_output {
        Ok(_) => {
            has_firwalld = true;
            "active".green().bold()
        }
        Err(_) => "inactive".red().bold(),
    };

    print::aligned_line("ufw", ufw_activity);
    print::aligned_line("firewalld", firewalld_activity);

    if !has_ufw && !has_firwalld {
        let output = format!(
            "{}",
            "No active firewall detected. Services may be exposed to public."
                .color(colors::TEXT_DEFAULT)
        );
        print::println("");
        print::println(&output);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn print_firewall_status() -> anyhow::Result<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn print_firewall_status() -> anyhow::Result<()> {
    Ok(())
}
