use std::time::Duration;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rand;

const BANNER_0: &str =  r#"
       ███▄ ▄███▓ ▄▄▄       ██▓███   ██▓███   ██▀███
      ▓██▒▀█▀ ██▒▒████▄    ▓██░  ██▒▓██░  ██▒▓██ ▒ ██▒
      ▓██    ▓██░▒██  ▀█▄  ▓██░ ██▓▒▓██░ ██▓▒▓██ ░▄█ ▒
      ▒██    ▒██ ░██▄▄▄▄██ ▒██▄█▓▒ ▒▒██▄█▓▒ ▒▒██▀▀█▄
      ▒██▒   ░██▒ ▓█   ▓██▒▒██▒ ░  ░▒██▒ ░  ░░██▓ ▒██▒
      ░ ▒░   ░  ░ ▒▒   ▓▒█░▒▓▒░ ░  ░▒▓▒░ ░  ░░ ▒▓ ░▒▓░
      ░  ░      ░  ▒   ▒▒ ░░▒ ░     ░▒ ░       ░▒ ░ ▒░
      ░      ░     ░   ▒   ░░       ░░         ░░   ░
             ░         ░  ░                     ░
"#;

const BANNER_1: &str = r#"
    _   .-')      ('-.      _ (`-.    _ (`-.  _  .-')
   ( '.( OO )_   ( OO ).-. ( (OO  )  ( (OO  )( \( -O )
    ,--.   ,--.) / . --. /_.`     \ _.`     \ ,------.
    |   `.'   |  | \-.  \(__...--''(__...--'' |   /`. '
    |         |.-'-'  |  ||  /  | | |  /  | | |  /  | |
    |  |'.'|  | \| |_.'  ||  |_.' | |  |_.' | |  |_.' |
    |  |   |  |  |  .-.  ||  .___.' |  .___.' |  .  '.'
    |  |   |  |  |  | |  ||  |      |  |      |  |\  \
    `--'   `--'  `--' `--'`--'      `--'      `--' '--'
"#;

const BANNER_2: &str = r#"
        ___       ___       ___       ___       ___
       /\__\     /\  \     /\  \     /\  \     /\  \
      /::L_L_   /::\  \   /::\  \   /::\  \   /::\  \
     /:/L:\__\ /::\:\__\ /::\:\__\ /::\:\__\ /::\:\__\
     \/_/:/  / \/\::/  / \/\::/  / \/\::/  / \;:::/  /
       /:/  /    /:/  /     \/__/     \/__/   |:\/__/
       \/__/     \/__/                         \|__|
"#;

const BANNER_3: &str = r#"
   ___ __ __   ________   ______   ______   ______
  /__//_//_/\ /_______/\ /_____/\ /_____/\ /_____/\
  \::\| \| \ \\::: _  \ \\:::_ \ \\:::_ \ \\:::_ \ \
   \:.      \ \\::(_)  \ \\:(_) \ \\:(_) \ \\:(_) ) )_
    \:.\-/\  \ \\:: __  \ \\: ___\/ \: ___\/ \: __ `\ \
     \. \  \  \ \\:.\ \  \ \\ \ \    \ \ \    \ \ `\ \ \
      \__\/ \__\/ \__\/\__\/ \_\/     \_\/     \_\/ \_\/
"#;

const BANNER_4: &str = r#"
    =/\                 /\=
    / \'._   (\_/)   _.'/ \       (_                   _)
   / .''._'--(o.o)--'_.''. \       /\                 /\
  /.' _/ |`'=/ " \='`| \_ `.\     / \'._   (\_/)   _.'/ \
 /` .' `\;-,'\___/',-;/` '. '\   /_.''._'--('.')--'_.''._\
/.-' jgs   `\(-V-)/`       `-.\  | \_ / `;=/ " \=;` \ _/ |
             "   "               \/  `\__|`\___/`|__/`  \/
                                  `       \(/|\)/       `
                                           " ` "
"#;

pub fn print_header() {
    println!();
    initialize();
    let n: u8 = rand::random_range(0..=4);
    banner(n);
}

fn initialize() {
    let sep = "<══════════════════".bright_black();
    let text = "⟦ INITIALIZING MAPPR ⟧".bright_green().bold();
    let end = "══════════════════>".bright_black();
    println!("{}{}{}", sep, text, end);
}

fn banner(id: u8) {
    match id {
        0 => println!("{}", BANNER_0.red()),
        1 => println!("{}", BANNER_1.truecolor(255, 165, 0)),
        2 => println!("{}", BANNER_2.green()),
        3 => println!("{}", BANNER_3.blue()),
        4 => println!("{}", BANNER_4.truecolor(80, 80, 100)),
        _ => { },
    }
}

pub fn separator(msg: &str) {
    let total_width: usize = 60; // explicitly typed
    let formatted = format!("⟦ {} ⟧", msg);
    let msg_len = formatted.chars().count();

    let dash_count = total_width.saturating_sub(msg_len);
    let left = dash_count / 2;
    let right = dash_count - left;

    let line = format!(
        "{}{}{}",
        "─".repeat(left),
        formatted.to_uppercase().bright_green(),
        "─".repeat(right)
    )
        .bright_black();

    println!("{}", line);
}

pub fn print_status(msg: &str) {
    let prefix = ">".bright_black();
    println!("{}", format!("{} {}", prefix, msg));
}

pub fn create_progressbar(len: u64, prefix: String) -> ProgressBar {
    let progress_bar = ProgressBar::new(len);
    progress_bar.set_prefix(prefix);
    progress_bar.enable_steady_tick(Duration::from_millis(100));
    progress_bar.set_style(
        ProgressStyle::with_template("[{prefix}] {elapsed_precise} {bar:36.cyan/blue} {pos:>4}/{len:4} {msg}")
            .unwrap()
            .progress_chars("■■□"),
    );
    progress_bar
}