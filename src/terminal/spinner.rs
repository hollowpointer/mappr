use std::time::Duration;

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;

use crate::terminal::colors;

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

pub fn report_discovery_progress(count: usize) {
    SPINNER.set_message(
        format!(
            "Identified {} so far...",
            format!("{} hosts", count).green().bold()
        )
        .color(colors::TEXT_DEFAULT)
        .to_string(),
    );
}
