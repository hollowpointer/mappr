use std::sync::OnceLock;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

const TIPS: &[&str] = &["You can press 'q' to finish early"];

pub struct SpinnerHandle {
    pub spinner: ProgressBar,
}

impl SpinnerHandle {

    pub fn println(&self, msg: &String) {
        self.spinner.println(msg);
    }

    pub fn finish_and_clear(&self) {
        self.spinner.finish_and_clear();
    }

    pub fn set_message(&self, msg: String) {
        self.spinner.set_message(msg);
    }
}
pub(crate) static SPINNER: OnceLock<SpinnerHandle> = OnceLock::new();

pub fn get_spinner() -> &'static SpinnerHandle {
    SPINNER.get_or_init(|| init_spinner())
}

fn init_spinner() -> SpinnerHandle {
    let pb = ProgressBar::new_spinner();
    
    let style = ProgressStyle::with_template("{spinner:.blue} {msg}")
        .unwrap()
        .tick_strings(&["▁▁▁▁▁", "▁▂▂▂▁", "▁▄▂▄▁", "▂▄▆▄▂", "▄▆█▆▄", "▂▄▆▄▂", "▁▄▂▄▁", "▁▂▂▂▁"]);
    pb.set_style(style);
    pb.enable_steady_tick(Duration::from_millis(100));

    let (_, rx) = mpsc::channel::<String>();
    let pb_clone = pb.clone();

    thread::spawn(move || {
        let mut tip_index = 0;
        let mut last_phase = 0;

        loop {
            if pb_clone.is_finished() { break; }
            while let Ok(_) = rx.try_recv() {}
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let phase = (secs / 2) % 2;

            if phase == 0 {
                let tip = TIPS[tip_index % TIPS.len()];
                pb_clone.set_message(format!("{}", tip.italic().white()));
                
                if phase != last_phase {
                    tip_index = (tip_index + 1) % TIPS.len();
                }
            } else {
                let count = mappr_core::scanner::get_host_count();
                pb_clone.set_message(format!(
                    "Identified {} hosts so far...",
                    count.to_string().green().bold()
                ));
            }
            last_phase = phase;
            thread::sleep(Duration::from_millis(100));
        }
    });

    SpinnerHandle { spinner: pb }
}

pub struct SpinnerWriter;

impl std::io::Write for SpinnerWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8_lossy(buf);
        let msg = msg.trim_end();
        get_spinner().println(&msg.to_string());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
