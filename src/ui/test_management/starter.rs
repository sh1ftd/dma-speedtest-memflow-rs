use crate::{
    speedtest::Connector,
    speedtest::SpeedTest,
    ui::console::{ConsoleWindow, log_to_console},
    ui::helpers::get_size_label,
};
use std::{sync::mpsc::Sender, time::Duration};
use tokio::sync::mpsc;

pub fn start_test(
    connector: Connector,
    pcileech_device: String,
    duration: u64,
    test_sizes: &[(usize, bool)],
    console: &ConsoleWindow,
    modal_tx: Sender<String>,
    stats_tx: mpsc::Sender<(f64, u64, f64, usize, f64)>,
) -> Result<SpeedTest, String> {
    let selected_sizes = collect_enabled_sizes(test_sizes);

    log_to_console(console, "Starting speed test...");

    let test = SpeedTest::new(connector, pcileech_device)
        .map_err(|e| format!("Failed to initialize test: {e}"))?;

    let test_clone = test.clone();
    let console_clone = console.clone();

    spawn_test_runner(
        selected_sizes,
        duration,
        console_clone,
        test_clone,
        modal_tx,
        stats_tx,
    );

    Ok(test)
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
    stats_tx: mpsc::Sender<(f64, u64, f64, usize, f64)>,
) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create runtime");
        runtime.block_on(async move {
            if test_sizes.is_empty() {
                log_to_console(&console, "No test sizes selected!");
                return;
            }

            let test_duration = Duration::from_secs(duration);
            for size in test_sizes {
                log_test_start(&console, size);

                if let Err(e) = test
                    .run_test_with_size(size, test_duration, stats_tx.clone())
                    .await
                {
                    handle_test_error(&console, e, &modal_tx);
                    return;
                }
            }
        });
    });
}

fn log_test_start(console: &ConsoleWindow, size: usize) {
    log_to_console(
        console,
        &format!("Testing with read size: {}", get_size_label(size)),
    );
}

fn handle_test_error(console: &ConsoleWindow, error: anyhow::Error, modal_tx: &Sender<String>) {
    let error_msg = format!("Test error: {error}");
    log_to_console(console, &error_msg);
    let _ = modal_tx.send(error_msg);
}
