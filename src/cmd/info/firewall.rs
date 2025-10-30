use anyhow;

#[cfg(target_os = "linux")]
pub fn print_firewall_status() -> anyhow::Result<()> {
    use std::process::Command;
    use colored::Colorize;
    use crate::utils::print;

    print::header("firewall status");

    let ufw_output = Command::new("ufw")
    .arg("status")
    .output();
    let ufw_value = match ufw_output {
        Ok(_) => "active".green().bold(),
        Err(_) => "inactive".red().bold()
    };

    print::aligned_line("UFW", ufw_value);

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