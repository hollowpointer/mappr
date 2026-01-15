use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use colored::*;
use indicatif::ProgressStyle;
use tracing::Span;
use tracing_indicatif::{span_ext::IndicatifSpanExt, IndicatifLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::terminal::logging;

const TIPS: &[&str] = &["You can press 'q' to finish early"];

pub fn init_logging() {
    let indicatif_layer = IndicatifLayer::new()
        .with_progress_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&["▁▁▁▁▁", "▁▂▂▂▁", "▁▄▂▄▁", "▂▄▆▄▂", "▄▆█▆▄", "▂▄▆▄▂", "▁▄▂▄▁", "▁▂▂▂▁"])
        );

    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,mappr=debug,mio=error"));

    let formatting_layer = tracing_subscriber::fmt::layer()
        .event_format(logging::MapprFormatter)
        .with_writer(indicatif_layer.get_stderr_writer()); 

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(formatting_layer)
        .with(indicatif_layer)
        .init();
}


pub fn start_discovery_spinner(span: Span, running: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut tip_index = 0;
        let mut last_phase = 0;

        while running.load(Ordering::Relaxed) {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let phase = (secs / 2) % 2;

            if phase == 0 {
                let tip = TIPS[tip_index % TIPS.len()];
                span.pb_set_message(&format!("{}", tip.italic().white()));
                
                if phase != last_phase {
                    tip_index = (tip_index + 1) % TIPS.len();
                }
            } else {
                let count: usize = mappr_core::scanner::get_host_count();
                let host_str: &str = match count {
                    1 => "host",
                    _ => "hosts"
                };
                span.pb_set_message(&format!(
                    "Identified {} {} so far...",
                    count.to_string().green().bold(),
                    host_str
                ));
            }

            last_phase = phase;
            thread::sleep(Duration::from_millis(100));
        }
    })
}
