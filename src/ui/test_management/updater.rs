use crate::speedtest::{BenchOp, BenchSample, BenchStats, PassAggregator, format_console_log_line};
use crate::ui::console::{self, ConsoleWindow};
use crate::ui::constants::CONSOLE_STATS_LOG_INTERVAL_SECS;
use crate::ui::types::{StatsUpdateParams, TestResults};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub fn handle_stats_update(
    stats_rx: &mut mpsc::Receiver<BenchStats>,
    params: &mut StatsUpdateParams<'_>,
    results: &TestResults,
    console: &ConsoleWindow,
) -> bool {
    let mut stats_closed = false;

    while let Some(update) = try_recv(stats_rx) {
        match update {
            StatsUpdate::Data(sample) => apply_sample(sample, params, results, console),
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

enum StatsUpdate {
    Data(BenchSample),
    Closed,
    Pending,
}

fn try_recv(stats_rx: &mut mpsc::Receiver<BenchStats>) -> Option<StatsUpdate> {
    match stats_rx.try_recv() {
        Ok(stats) => Some(StatsUpdate::Data(BenchSample::from_stats(stats))),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => Some(StatsUpdate::Pending),
        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => Some(StatsUpdate::Closed),
    }
}

fn apply_sample(
    sample: BenchSample,
    params: &mut StatsUpdateParams<'_>,
    results: &TestResults,
    console: &ConsoleWindow,
) {
    let prev_op = *params.current_bench_op;
    let prev_size = *params.current_test_size;

    record_chunk_completion(params, prev_op, prev_size, sample.op, sample.chunk_bytes);

    *params.current_throughput = sample.throughput_mib_s;
    *params.current_ops_per_sec = sample.ops_per_sec;
    *params.current_latency = sample.latency_us;
    *params.current_bench_op = Some(sample.op);

    if prev_op != Some(sample.op) || prev_size != Some(sample.chunk_bytes) {
        *params.test_start_time = Some(Instant::now());
        *params.current_test_size = Some(sample.chunk_bytes);
    }

    *params.max_throughput = (*params.max_throughput).max(sample.throughput_mib_s);

    if should_log_live_sample(params, prev_op, prev_size, sample.op, sample.chunk_bytes) {
        console::log_to_console(console, &format_console_log_line(&sample));
        *params.last_console_stats_log = Some(Instant::now());
    }

    record_pass_summary_sample(params.pass_aggregators, &sample);
    append_plot_point(results, params, &sample);
}

fn should_log_live_sample(
    params: &StatsUpdateParams<'_>,
    prev_op: Option<BenchOp>,
    prev_size: Option<usize>,
    op: BenchOp,
    size: usize,
) -> bool {
    let chunk_changed = prev_op != Some(op) || prev_size != Some(size);
    if chunk_changed {
        return true;
    }

    let interval = Duration::from_secs_f64(CONSOLE_STATS_LOG_INTERVAL_SECS);
    params
        .last_console_stats_log
        .map(|t| t.elapsed() >= interval)
        .unwrap_or(true)
}

fn record_chunk_completion(
    params: &mut StatsUpdateParams<'_>,
    prev_op: Option<BenchOp>,
    prev_size: Option<usize>,
    new_op: BenchOp,
    new_size: usize,
) {
    let Some((current_op, current_size)) = prev_op.zip(prev_size) else {
        return;
    };
    if current_op == new_op && current_size == new_size {
        return;
    }
    if params
        .completed_chunks
        .iter()
        .any(|(o, s, _)| *o == current_op && *s == current_size)
    {
        return;
    }
    let Some(start_time) = *params.test_start_time else {
        return;
    };
    params
        .completed_chunks
        .push((current_op, current_size, start_time.elapsed().as_secs_f64()));
}

fn append_plot_point(results: &TestResults, params: &StatsUpdateParams<'_>, sample: &BenchSample) {
    if let Ok(mut results_guard) = results.lock() {
        if !results_guard
            .iter()
            .any(|(op, size, _)| *op == sample.op && *size == sample.chunk_bytes)
        {
            results_guard.push((
                sample.op,
                sample.chunk_bytes,
                (Vec::new(), Vec::new(), Vec::new()),
            ));
        }

        if let Some(entry) = results_guard
            .iter_mut()
            .find(|(op, size, _)| *op == sample.op && *size == sample.chunk_bytes)
        {
            let elapsed_secs = (*params.test_start_time)
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);

            entry.2.0.push((elapsed_secs, sample.throughput_mib_s));
            entry.2.1.push((elapsed_secs, sample.ops_per_sec as f64));
            entry.2.2.push((elapsed_secs, sample.latency_us));
        }
    }
}

fn record_pass_summary_sample(aggregators: &mut Vec<PassAggregator>, sample: &BenchSample) {
    if let Some(aggregator) = aggregators
        .iter_mut()
        .find(|agg| agg.is_for(sample.op, sample.chunk_bytes))
    {
        aggregator.push(sample);
        return;
    }

    let mut aggregator = PassAggregator::new(sample.op, sample.chunk_bytes);
    aggregator.push(sample);
    aggregators.push(aggregator);
}

fn finalize_current_chunk(params: &mut StatsUpdateParams<'_>) {
    if let Some(current_op) = *params.current_bench_op
        && let Some(current_size) = *params.current_test_size
        && !params
            .completed_chunks
            .iter()
            .any(|(o, s, _)| *o == current_op && *s == current_size)
        && let Some(start_time) = *params.test_start_time
    {
        params.completed_chunks.push((
            current_op,
            current_size,
            start_time.elapsed().as_secs_f64(),
        ));
    }
}
