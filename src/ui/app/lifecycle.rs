use super::super::test_management::{start_connect, start_test_from_connected};

use super::state::SpeedTestApp;
use crate::speedtest::{BenchmarkReport, ReportFormat, default_report_path, write_report_to_path};
use crate::ui::console::log_to_console;
use crate::ui::types::ReportExportStatus;

impl SpeedTestApp {
    pub fn start_test_impl(&mut self) {
        if !self.can_start_test() {
            return;
        }

        self.is_connecting = true;
        self.error_message = None;
        self.show_error_modal = false;
        self.error_modal_message.clear();
        self.connection_cancelled = false;
        self.results.lock().unwrap().clear();
        self.probe_targets = None;
        self.current_throughput = 0.0;
        self.current_ops_per_sec = 0;
        self.current_bench_op = None;
        self.current_latency = 0.0;
        self.max_throughput = 0.0;
        self.test_start_time = None;
        self.overall_test_start_time = None;
        self.test_end_time = None;
        self.current_test_size = None;
        self.completed_chunks.clear();
        self.pass_aggregators.clear();
        self.last_console_stats_log = None;
        self.report_export_status = None;

        let bench_mode = self.bench_mode;
        let max_chunk = crate::bench_config::max_enabled_chunk_bytes(&self.test_sizes);
        let rx = start_connect(
            self.connector,
            self.pcileech_device.clone(),
            bench_mode,
            max_chunk,
            &self.console,
        );
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
                if self.connection_cancelled {
                    self.connection_cancelled = false;
                    self.show_config = true;
                    return;
                }
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

        if self.connection_cancelled {
            self.connection_cancelled = false;
            self.show_config = true;
            return;
        }

        match result {
            Ok(test) => {
                let targets = test.probe_targets();
                self.probe_targets = Some(targets);
                for line in test.probe_connect_detail_lines() {
                    log_to_console(&self.console, &line);
                }

                self.is_running = true;
                self.test_start_time = Some(std::time::Instant::now());
                self.overall_test_start_time = Some(std::time::Instant::now());

                let (modal_tx, modal_rx) = std::sync::mpsc::channel::<String>();
                let (stats_tx, stats_rx) = tokio::sync::mpsc::channel(100);
                self.stats_rx = Some(stats_rx);
                self.modal_rx = Some(modal_rx);

                let test_clone = test.clone();
                self.bench_done_rx = Some(start_test_from_connected(
                    test_clone,
                    self.duration,
                    &self.test_sizes,
                    &self.console,
                    modal_tx,
                    stats_tx,
                ));

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
        if self.is_connecting {
            self.cancel_connection_impl();
            return;
        }

        if let Some(test) = &self.test {
            test.request_cancel();
        }

        self.is_running = false;
        self.test = None;
        self.stats_rx = None;

        if let Some(overall_start_time) = self.overall_test_start_time {
            self.test_end_time = Some(overall_start_time.elapsed().as_secs_f64());
        }

        if let Some(start_time) = self.test_start_time
            && let Some(current_op) = self.current_bench_op
            && let Some(current_size) = self.current_test_size
            && !self
                .completed_chunks
                .iter()
                .any(|(o, s, _)| *o == current_op && *s == current_size)
        {
            self.completed_chunks.push((
                current_op,
                current_size,
                start_time.elapsed().as_secs_f64(),
            ));
        }
        self.test_start_time = None;
        self.overall_test_start_time = None;
        self.current_test_size = None;
        self.current_bench_op = None;
        self.modal_rx = None;
        log_to_console(&self.console, "Test stopped");
    }

    fn cancel_connection_impl(&mut self) {
        self.connection_cancelled = true;
        self.is_connecting = false;
        self.show_config = true;
        self.show_error_modal = false;
        self.error_modal_message.clear();
        log_to_console(
            &self.console,
            "Connection cancelled; waiting for connector cleanup before another start.",
        );
    }

    pub fn poll_bench_thread_finished(&mut self) {
        let rx = match self.bench_done_rx.as_ref() {
            Some(rx) => rx,
            None => return,
        };

        match rx.try_recv() {
            Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.bench_done_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    pub fn export_report_impl(&mut self, format: ReportFormat) {
        let Some(probes) = self.probe_targets else {
            let message = "No probe metadata available for report export.";
            self.report_export_status = Some(ReportExportStatus::error(message));
            log_to_console(&self.console, message);
            return;
        };

        let summaries = self
            .pass_aggregators
            .iter()
            .map(crate::speedtest::PassAggregator::summary)
            .collect::<Vec<_>>();

        if summaries.is_empty() {
            let message = "No benchmark samples available for report export.";
            self.report_export_status = Some(ReportExportStatus::error(message));
            log_to_console(&self.console, message);
            return;
        }

        let sizes = self
            .test_sizes
            .iter()
            .filter_map(|(size, enabled)| (*enabled).then_some(*size))
            .collect::<Vec<_>>();
        let report = BenchmarkReport::new(
            self.connector,
            self.bench_mode,
            self.duration,
            &sizes,
            probes,
            summaries,
        );
        let path = default_report_path(format);

        match write_report_to_path(&report, format, &path) {
            Ok(()) => {
                let message = format!("Report saved: {}", path.display());
                self.report_export_status = Some(ReportExportStatus::success(message.clone()));
                log_to_console(&self.console, &message);
            }
            Err(e) => {
                let message = format!("Report export failed: {e}");
                self.report_export_status = Some(ReportExportStatus::error(message.clone()));
                log_to_console(&self.console, &message);
            }
        }
    }
}
