use std::{fmt::Display, time::Duration};

use crate::{GLOBAL_KEY_WIDTH, utils::colors};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use rand;
use unicode_width::UnicodeWidthStr;

const TOTAL_WIDTH: usize = 64;

pub static SPINNER: Lazy<ProgressBar> = Lazy::new(|| {
    let pb: ProgressBar = ProgressBar::new_spinner();
    let style: ProgressStyle = ProgressStyle::with_template("{spinner:.blue} {msg}")
        .unwrap()
        .tick_strings(&[
            "▁▁▁▁▁",
            "▁▂▂▂▁",
            "▁▄▂▄▁",
            "▂▄▆▄▂",
            "▄▆█▆▄",
            "▂▄▆▄▂",
            "▁▄▂▄▁",
            "▁▂▂▂▁",
        ]);
    pb.set_style(style);
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
});

const BANNER_0: &str = r#"
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

pub trait WithDefaultColor {
    fn with_default(self, default_color: Color) -> ColoredString;
}

impl<'a> WithDefaultColor for &'a str {
    fn with_default(self, default_color: Color) -> ColoredString {
        self.color(default_color)
    }
}

impl<'a> WithDefaultColor for String {
    fn with_default(self, default_color: Color) -> ColoredString {
        self.color(default_color)
    }
}

impl WithDefaultColor for ColoredString {
    fn with_default(self, _default_color: Color) -> ColoredString {
        self
    }
}

pub fn print_banner() {
    println!();
    initialize();
    let n: u8 = rand::random_range(0..=4);
    banner(n);
}

fn initialize() {
    let text_content: String = format!("⟦ INITIALIZING MAPPR v{} ⟧ ", env!("CARGO_PKG_VERSION"));
    let text_width: usize = UnicodeWidthStr::width(text_content.as_str());
    let text: ColoredString = text_content.bright_green().bold();
    let sep: ColoredString = "═".repeat((TOTAL_WIDTH - text_width) / 2).bright_black();
    println!("{}{}{}", sep, text, sep);
}

fn banner(id: u8) {
    match id {
        0 => println!("{}", BANNER_0.red()),
        1 => println!("{}", BANNER_1.truecolor(255, 165, 0)),
        2 => println!("{}", BANNER_2.green()),
        3 => println!("{}", BANNER_3.blue()),
        4 => println!("{}", BANNER_4.truecolor(80, 80, 100)),
        _ => {}
    }
}

pub fn header(msg: &str) {
    let formatted: String = format!("⟦ {} ⟧", msg);
    let msg_len: usize = formatted.chars().count();

    let dash_count: usize = TOTAL_WIDTH.saturating_sub(msg_len);
    let left: usize = dash_count / 2;
    let right: usize = dash_count - left;

    let line: ColoredString = format!(
        "{}{}{}",
        "─".repeat(left),
        formatted.to_uppercase().bright_green(),
        "─".repeat(right)
    )
    .bright_black();

    SPINNER.println(&format!("{line}"));
}

pub fn aligned_line<V>(key: &str, value: V)
where
    V: Display + WithDefaultColor,
{
    let whitespace: String = ".".repeat(GLOBAL_KEY_WIDTH.get() + 1 - key.len());
    let colon: String = format!(
        "{}{}",
        whitespace.color(colors::SEPARATOR),
        ":".color(colors::SEPARATOR)
    );
    let value: ColoredString = value.with_default(colors::TEXT_DEFAULT);
    print_status(format!("{}{} {}", key.color(colors::PRIMARY), colon, value));
}

pub fn print_status<T: AsRef<str>>(msg: T) {
    let prefix: ColoredString = ">".color(colors::SEPARATOR);
    let message: String = format!("{} {}", prefix, msg.as_ref().color(colors::TEXT_DEFAULT));
    SPINNER.println(message);
}

pub fn tree_head(idx: usize, name: &str) {
    let idx_str: String = format!("[{}]", idx.to_string().color(colors::ACCENT));
    let output: String = format!(
        "{} {}",
        idx_str.color(colors::SEPARATOR),
        name.color(colors::PRIMARY)
    );
    println(&output);
}

pub fn as_tree_one_level(key_value_pair: Vec<(String, ColoredString)>) {
    for (i, (key, value)) in key_value_pair.iter().enumerate() {
        let last: bool = i + 1 == key_value_pair.len();
        let branch: ColoredString = if !last {
            "├─".bright_black()
        } else {
            "└─".bright_black()
        };
        let key: ColoredString = key.color(colors::TEXT_DEFAULT);
        let output: String = format!(
            " {} {}{}{} {}",
            branch,
            key,
            ".".repeat(7 - key.len()).color(colors::SEPARATOR), // 7 what? bananas?
            ":".color(colors::SEPARATOR),
            value
        );
        println(&output);
    }
}

pub fn println(msg: &str) {
    SPINNER.println(format!("{}", msg));
}

const NO_RESULTS_0: &str = r#"
                       _  _    ___  _  _                 
                      | || |  / _ \| || |                
                      | || |_| | | | || |_               
                      |__   _| |_| |__   _|              
         _   _  ___ _____|_|__\___/__ |_|  _ _   _ ____  
        | \ | |/ _ \_   _| |  ___/ _ \| | | | \ | |  _ \ 
        |  \| | | | || |   | |_ | | | | | | |  \| | | | |
        | |\  | |_| || |   |  _|| |_| | |_| | |\  | |_| |
        |_| \_|\___/ |_|   |_|   \___/ \___/|_| \_|____/ 
"#;

pub fn no_results() {
    println(&format!("{}", NO_RESULTS_0.red().bold()));
}

pub fn end_of_program() {
    println(format!("{}", "═".repeat(TOTAL_WIDTH).color(colors::SEPARATOR)).as_str());
}
