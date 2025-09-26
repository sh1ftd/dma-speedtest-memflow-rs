use eframe::egui;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const MAX_LOG_ENTRIES: usize = 1000000;

#[derive(Clone)]
pub struct ConsoleWindow {
    logs: Arc<Mutex<VecDeque<String>>>,
    visible: bool,
}

impl Default for ConsoleWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleWindow {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES))),
            visible: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        egui::Window::new("Console")
            .default_pos([50.0, 50.0])
            .default_size([1000.0, 500.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if let Ok(logs) = self.logs.lock() {
                            for log in logs.iter() {
                                ui.label(log);
                            }
                        }
                    });
            });
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn log(&self, message: String) {
        if let Ok(mut logs) = self.logs.lock() {
            if logs.len() >= MAX_LOG_ENTRIES {
                logs.pop_front();
            }
            logs.push_back(message);
        }
    }
}

pub fn log_to_console(console: &ConsoleWindow, message: &str) {
    console.log(message.to_string());
}
