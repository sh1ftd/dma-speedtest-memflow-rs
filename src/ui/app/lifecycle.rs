use super::super::test_management::{start_connect, start_test_from_connected};

use super::state::SpeedTestApp;
use crate::ui::console::log_to_console;

impl SpeedTestApp {
    pub fn start_test_impl(&mut self) {
        self.is_connecting = true;
        self.error_message = None;
        self.show_error_modal = false;
        self.error_modal_message.clear();
        self.results.lock().unwrap().clear();
        self.current_throughput = 0.0;
        self.current_reads = 0;
        self.current_latency = 0.0;
        self.max_throughput = 0.0;
        self.test_start_time = None;
        self.overall_test_start_time = None;
        self.test_end_time = None;
        self.current_test_size = None;
        self.completed_chunks.clear();

        let rx = start_connect(self.connector, self.pcileech_device.clone(), &self.console);
        self.connect_rx = Some(rx);
    }

    pub fn poll_connection(&mut self) {
        let rx = match self.connect_rx.as_ref() {
            Some(rx) => rx,
            None => return,
        };

        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.connect_rx = None;
                self.is_connecting = false;
                let msg = "Connection thread terminated unexpectedly".to_string();
                log_to_console(&self.console, &msg);
                self.show_error_modal = true;
                self.error_modal_message = msg;
                self.show_config = true;
                return;
            }
        };

        self.connect_rx = None;
        self.is_connecting = false;

        match result {
            Ok(test) => {
                self.is_running = true;
                self.test_start_time = Some(std::time::Instant::now());
                self.overall_test_start_time = Some(std::time::Instant::now());

                let (modal_tx, modal_rx) = std::sync::mpsc::channel::<String>();
                let (stats_tx, stats_rx) = tokio::sync::mpsc::channel(100);
                self.stats_rx = Some(stats_rx);
                self.modal_rx = Some(modal_rx);

                let test_clone = test.clone();
                start_test_from_connected(
                    test_clone,
                    self.duration,
                    &self.test_sizes,
                    &self.console,
                    modal_tx,
                    stats_tx,
                );

                self.test = Some(test);
            }
            Err(error_msg) => {
                log_to_console(&self.console, &error_msg);
                self.show_error_modal = true;
                self.error_modal_message = error_msg;
                self.show_config = true;
            }
        }
    }

    pub fn stop_test_impl(&mut self) {
        self.is_running = false;
        self.test = None;
        self.stats_rx = None;

        if let Some(overall_start_time) = self.overall_test_start_time {
            let final_time = overall_start_time.elapsed().as_secs_f64();
            self.test_end_time = Some(final_time);

            if let Some(current_size) = self.current_test_size
                && !self
                    .completed_chunks
                    .iter()
                    .any(|(s, _)| *s == current_size)
            {
                self.completed_chunks.push((current_size, final_time));
            }
        }
        self.test_start_time = None;
        self.overall_test_start_time = None;
        self.current_test_size = None;
        self.modal_rx = None;
        log_to_console(&self.console, "Test stopped");
    }
}
