use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{sync::mpsc, thread};

pub struct InputHandle {
    rx: mpsc::Receiver<Event>,
    tx: Option<mpsc::Sender<Event>>,
}

impl InputHandle {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { rx, tx: Some(tx) }
    }

    pub fn start(&mut self) {
        if let Some(tx) = self.tx.take() {
            thread::spawn(move || {
                enable_raw_mode().expect("failed to enable raw mode");
                loop {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        let is_q = key_event.code == KeyCode::Char('q');
                        let is_ctrl_c = key_event.code == KeyCode::Char('c')
                            && key_event.modifiers.contains(KeyModifiers::CONTROL);

                        if (is_q || is_ctrl_c) && key_event.kind == KeyEventKind::Press {
                            let _ = tx.send(Event::Key(key_event));
                            break;
                        }
                    }
                }
                let _ = disable_raw_mode();
            });
        }
    }

    pub fn should_interrupt(&self) -> bool {
        match self.rx.try_recv() {
            Ok(Event::Key(event)) => {
                event.code == KeyCode::Char('q') || event.code == KeyCode::Char('c')
            }
            _ => false,
        }
    }
}

impl Drop for InputHandle {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}
