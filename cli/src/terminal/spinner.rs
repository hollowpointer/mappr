use std::sync::OnceLock;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

const TIP_DURATION: Duration = Duration::from_secs(1);
const MESSAGE_READ_TIME: Duration = Duration::from_secs(1);
const TIPS: &[&str] = &["You can press 'q' to finish early"];

pub struct SpinnerHandle {
    pub spinner: ProgressBar,
    tx: Sender<String>,
}

impl SpinnerHandle {
    pub fn send_to_queue(&self, message: String) {
        let _ = self.tx.send(message);
    }

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
    const MIN_TIP_VISIBILITY: Duration = Duration::from_millis(750);
    let style = ProgressStyle::with_template("{spinner:.blue} {msg}")
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

    let (tx, rx) = mpsc::channel::<String>();
    let pb_clone = pb.clone();

    thread::spawn(move || {
        let mut tip_index = 0;
        let mut next_action_time = Instant::now() + TIP_DURATION;
        let mut is_showing_tip = false;
        let mut last_tip_time = Instant::now();

        loop {
            if pb_clone.is_finished() {
                break;
            }

            let now = Instant::now();
            let wait_time = if now >= next_action_time {
                Duration::ZERO
            } else {
                next_action_time - now
            };

            match rx.recv_timeout(wait_time) {
                Ok(mut msg) => {
                    if is_showing_tip {
                        let elapsed = last_tip_time.elapsed();
                        if elapsed < MIN_TIP_VISIBILITY {
                            thread::sleep(MIN_TIP_VISIBILITY - elapsed);
                        }
                        is_showing_tip = false;
                    }
                    while let Ok(newer_msg) = rx.try_recv() {
                        msg = newer_msg;
                    }
                    pb_clone.set_message(msg);
                    next_action_time = Instant::now() + MESSAGE_READ_TIME;
                }
                Err(RecvTimeoutError::Timeout) => {
                    let tip = TIPS[tip_index % TIPS.len()];
                    pb_clone.set_message(format!("{}", tip.italic().white()));

                    tip_index += 1;
                    is_showing_tip = true;
                    last_tip_time = Instant::now();

                    next_action_time = Instant::now() + TIP_DURATION;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    });

    SpinnerHandle { spinner: pb, tx }
}

pub fn report_discovery_progress(count: usize) {
    get_spinner().send_to_queue(format!(
        "Identified {} hosts so far...",
        count.to_string().green().bold()
    ));
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
