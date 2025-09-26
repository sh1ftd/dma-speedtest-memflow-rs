use crate::ui::console::{self, ConsoleWindow};
use crate::ui::helpers::get_size_label;
use crate::ui::types::{StatsUpdateParams, TestResults};
use std::time::Instant;
use tokio::sync::mpsc;

pub fn handle_stats_update(
    stats_rx: &mut mpsc::Receiver<(f64, u64, f64, usize, f64)>,
    params: &mut StatsUpdateParams<'_>,
    results: &TestResults,
    console: &ConsoleWindow,
) -> bool {
    let mut stats_closed = false;

    while let Some(update) = try_recv(stats_rx) {
        match update {
            StatsUpdate::Data(data) => process_stats(data, params, results, console),
            StatsUpdate::Closed => {
                stats_closed = true;
                break;
            }
            StatsUpdate::Pending => break,
        }
    }

    if stats_closed {
        finalize_current_chunk(params);
    }

    stats_closed
}

struct StatsData {
    throughput: f64,
    reads_per_sec: u64,
    read_size: usize,
    latency_us: f64,
}

enum StatsUpdate {
    Data(StatsData),
    Closed,
    Pending,
}

fn try_recv(stats_rx: &mut mpsc::Receiver<(f64, u64, f64, usize, f64)>) -> Option<StatsUpdate> {
    match stats_rx.try_recv() {
        Ok((throughput, reads_per_sec, _elapsed_secs, read_size, latency_us)) => {
            Some(StatsUpdate::Data(StatsData {
                throughput,
                reads_per_sec,
                read_size,
                latency_us,
            }))
        }
        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => Some(StatsUpdate::Pending),
        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => Some(StatsUpdate::Closed),
    }
}

fn process_stats(
    data: StatsData,
    params: &mut StatsUpdateParams<'_>,
    results: &TestResults,
    console: &ConsoleWindow,
) {
    record_chunk_completion(params, data.read_size);

    *params.current_throughput = data.throughput;
    *params.current_reads = data.reads_per_sec;
    *params.current_latency = data.latency_us;

    if params.current_test_size.as_ref().copied() != Some(data.read_size) {
        *params.test_start_time = Some(Instant::now());
        *params.current_test_size = Some(data.read_size);
    }

    let max_throughput = (*params.max_throughput).max(data.throughput);
    *params.max_throughput = max_throughput;

    log_stats(console, &data);
    update_results(results, params, &data);
}

fn record_chunk_completion(params: &mut StatsUpdateParams<'_>, read_size: usize) {
    if let Some(current_size) = (*params.current_test_size)
        && current_size != read_size
        && !params
            .completed_chunks
            .iter()
            .any(|(size, _)| *size == current_size)
        && let Some(start_time) = *params.test_start_time
    {
        let completion_time = start_time.elapsed().as_secs_f64();
        params
            .completed_chunks
            .push((current_size, completion_time));
    }
}

fn log_stats(console: &ConsoleWindow, data: &StatsData) {
    console::log_to_console(
        console,
        &format!(
            "Throughput: {:.2} MB/s, Reads/s: {}, Latency: {:.1} μs, Size: {}",
            data.throughput,
            data.reads_per_sec,
            data.latency_us,
            get_size_label(data.read_size)
        ),
    );
}

fn update_results(results: &TestResults, params: &StatsUpdateParams<'_>, data: &StatsData) {
    if let Ok(mut results_guard) = results.lock() {
        let entry_index = results_guard
            .iter()
            .position(|(size, _)| *size == data.read_size);

        if entry_index.is_none() {
            results_guard.push((data.read_size, (Vec::new(), Vec::new(), Vec::new())));
        }

        if let Some(entry) = results_guard
            .iter_mut()
            .find(|(size, _)| *size == data.read_size)
        {
            let elapsed_secs = (*params.test_start_time)
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            let reads_per_sec = data.reads_per_sec as f64;

            entry.1.0.push((elapsed_secs, data.throughput));
            entry.1.1.push((elapsed_secs, reads_per_sec));
            entry.1.2.push((elapsed_secs, data.latency_us));
        }
    }
}

fn finalize_current_chunk(params: &mut StatsUpdateParams<'_>) {
    if let Some(current_size) = (*params.current_test_size)
        && !params
            .completed_chunks
            .iter()
            .any(|(s, _)| *s == current_size)
        && let Some(start_time) = *params.test_start_time
    {
        let completion_time = start_time.elapsed().as_secs_f64();
        params
            .completed_chunks
            .push((current_size, completion_time));
    }
}
