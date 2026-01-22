use std::{cell::Cell, fmt::Display};

use crate::terminal::{banner, colors};
use colored::*;
use unicode_width::UnicodeWidthStr;
use tracing::info;

const TOTAL_WIDTH: usize = 64;

thread_local! {
    pub static GLOBAL_KEY_WIDTH: Cell<usize> = Cell::new(0);
}

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

fn safe_print(msg: &str) {
    info!(target: "mappr::print", raw_msg = msg);
}

pub fn banner() {
    let text_content: String = format!("⟦ MAPPR v{} ⟧ ", env!("CARGO_PKG_VERSION"));
    let text_width: usize = UnicodeWidthStr::width(text_content.as_str());
    let text: ColoredString = text_content.bright_green().bold();
    let sep: ColoredString = "═".repeat((TOTAL_WIDTH - text_width) / 2).bright_black();
    let output: String = format!("{}{}{}", sep, text, sep);
    
    safe_print(&output);
    banner::print();
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

    safe_print(&format!("{}", line));
}

pub fn fat_separator() {
    let sep: ColoredString = "═".repeat(TOTAL_WIDTH).bright_black();
    safe_print(&format!("{}", sep));
}

pub fn aligned_line<V>(key: &str, value: V)
where
    V: Display + WithDefaultColor,
{
    let whitespace: String = ".".repeat((GLOBAL_KEY_WIDTH.get() + 1).saturating_sub(key.len()));
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
    safe_print(&message);
}

pub fn tree_head(idx: usize, name: &str) {
    let idx_str: String = format!("[{}]", idx.to_string().color(colors::ACCENT));
    let output: String = format!(
        "{} {}",
        idx_str.color(colors::SEPARATOR),
        name.color(colors::PRIMARY)
    );
    safe_print(&output);
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
            ".".repeat(7 - key.len()).color(colors::SEPARATOR), 
            ":".color(colors::SEPARATOR),
            value
        );
        safe_print(&output);
    }
}

pub fn centerln(msg: &str) {
    let space = " ".repeat((TOTAL_WIDTH - console::measure_text_width(msg)) / 2);
    safe_print(&format!("{}{}{}", space, msg, space));
}

pub fn println(msg: &str) {
    safe_print(msg);
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
    safe_print(&format!("{}", NO_RESULTS_0.red().bold()));
}

pub fn end_of_program() {
    safe_print(&format!("{}", "═".repeat(TOTAL_WIDTH).color(colors::SEPARATOR)));
}