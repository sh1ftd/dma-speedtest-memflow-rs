use super::super::test_management::start_test;

use super::state::SpeedTestApp;
use crate::ui::console::log_to_console;

impl SpeedTestApp {
    pub fn start_test_impl(&mut self) {
        self.is_running = true;
        self.error_message = None;
        self.show_error_modal = false;
        self.error_modal_message.clear();
        self.results.lock().unwrap().clear();
        self.current_throughput = 0.0;
        self.current_reads = 0;
        self.current_latency = 0.0;
        self.max_throughput = 0.0;
        self.test_start_time = Some(std::time::Instant::now());
        self.overall_test_start_time = Some(std::time::Instant::now());
        self.test_end_time = None;
        self.current_test_size = None;
        self.completed_chunks.clear();

        let (modal_tx, modal_rx) = std::sync::mpsc::channel::<String>();
        let (stats_tx, stats_rx) = tokio::sync::mpsc::channel(100);
        self.stats_rx = Some(stats_rx);
        self.modal_rx = Some(modal_rx);

        match start_test(
            self.connector,
            self.pcileech_device.clone(),
            self.duration,
            &self.test_sizes,
            &self.console,
            modal_tx,
            stats_tx,
        ) {
            Ok(test) => {
                self.test = Some(test);
            }
            Err(error_msg) => {
                log_to_console(&self.console, &error_msg);
                self.show_error_modal = true;
                self.error_modal_message = error_msg;
                self.is_running = false;
                self.show_config = true;
                self.modal_rx = None;
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
