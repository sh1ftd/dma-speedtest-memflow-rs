use crate::{
    speedtest::{BenchMode, BenchOp, BenchPassStartFn, BenchStats, Connector, SpeedTest},
    ui::console::{ConsoleWindow, log_to_console},
    ui::helpers::get_size_label,
};
use std::{sync::Arc, sync::mpsc::Sender, time::Duration};
use tokio::sync::mpsc;

pub fn start_connect(
    connector: Connector,
    pcileech_device: String,
    bench_mode: BenchMode,
    max_chunk_bytes: usize,
    console: &ConsoleWindow,
) -> std::sync::mpsc::Receiver<Result<SpeedTest, String>> {
    let (tx, rx) = std::sync::mpsc::channel();

    log_to_console(console, "Connecting to device...");

    std::thread::spawn(move || {
        let result = SpeedTest::new(connector, pcileech_device, bench_mode, max_chunk_bytes)
            .map_err(|e| format!("Failed to initialize test: {e}"));
        let _ = tx.send(result);
    });

    rx
}

/// Start the test runner with an already-connected `SpeedTest`.
pub fn start_test_from_connected(
    test: SpeedTest,
    duration: u64,
    test_sizes: &[(usize, bool)],
    console: &ConsoleWindow,
    modal_tx: Sender<String>,
    stats_tx: mpsc::Sender<BenchStats>,
) -> std::sync::mpsc::Receiver<()> {
    let selected_sizes = collect_enabled_sizes(test_sizes);

    log_to_console(console, "Starting speed test...");

    spawn_test_runner(
        selected_sizes,
        duration,
        console.clone(),
        test,
        modal_tx,
        stats_tx,
    )
}

fn collect_enabled_sizes(test_sizes: &[(usize, bool)]) -> Vec<usize> {
    test_sizes
        .iter()
        .filter_map(|(size, enabled)| (*enabled).then_some(*size))
        .collect()
}

fn spawn_test_runner(
    test_sizes: Vec<usize>,
    duration: u64,
    console: ConsoleWindow,
    test: SpeedTest,
    modal_tx: Sender<String>,
    stats_tx: mpsc::Sender<BenchStats>,
) -> std::sync::mpsc::Receiver<()> {
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let warn_console = console.clone();
    let on_warn: crate::speedtest::BenchWarnFn = Arc::new(move |msg| {
        log_to_console(&warn_console, msg);
    });

    let on_pass_start: BenchPassStartFn = {
        let console = console.clone();
        let test = test.clone();
        Arc::new(move |op, size| log_test_start(&console, &test, op, size))
    };

    std::thread::spawn(move || {
        let _notify_done = DoneNotify(done_tx);
        let runtime = tokio::runtime::Runtime::new().expect("failed to create runtime");
        runtime.block_on(async move {
            if test_sizes.is_empty() {
                log_to_console(&console, "No test sizes selected!");
                return;
            }

            let test_duration = Duration::from_secs(duration);
            for size in test_sizes {
                if test.is_cancelled() {
                    break;
                }
                if let Err(e) = test
                    .run_passes_for_size(
                        size,
                        test_duration,
                        stats_tx.clone(),
                        Some(on_warn.clone()),
                        Some(on_pass_start.clone()),
                    )
                    .await
                {
                    handle_test_error(&console, e, &modal_tx);
                    return;
                }
            }
        });
    });

    done_rx
}

/// Signals the UI when the benchmark thread has fully exited (join equivalent).
struct DoneNotify(std::sync::mpsc::Sender<()>);

impl Drop for DoneNotify {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn log_test_start(console: &ConsoleWindow, test: &SpeedTest, op: BenchOp, size: usize) {
    let targets = test.probe_targets();
    let detail = match op {
        BenchOp::Read => targets.format_read_pass(size),
        BenchOp::Write => targets
            .format_write_pass(size)
            .unwrap_or_else(|| format!("write chunk {}", get_size_label(size))),
    };
    log_to_console(console, &detail);
}

fn handle_test_error(console: &ConsoleWindow, error: anyhow::Error, modal_tx: &Sender<String>) {
    let error_msg = format!("Test error: {error}");
    log_to_console(console, &error_msg);
    let _ = modal_tx.send(error_msg);
}
