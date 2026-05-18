//! Shared bench stats types and pass aggregation (CLI + GUI).

use super::bench::{BenchOp, BenchStats};
use crate::bench_config::format_chunk_size;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
pub struct BenchSample {
    pub op: BenchOp,
    pub throughput_mib_s: f64,
    pub ops_per_sec: u64,
    pub elapsed_secs: f64,
    pub chunk_bytes: usize,
    pub latency_us: f64,
}

impl BenchSample {
    pub fn from_tuple(
        (op, throughput_mib_s, ops_per_sec, elapsed_secs, chunk_bytes, latency_us): BenchStats,
    ) -> Self {
        Self {
            op,
            throughput_mib_s,
            ops_per_sec,
            elapsed_secs,
            chunk_bytes,
            latency_us,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PassSummary {
    pub op: BenchOp,
    pub chunk_bytes: usize,
    pub avg_mib_s: f64,
    pub avg_ops_s: f64,
    pub avg_latency_us: f64,
    pub samples: u64,
}

pub struct PassAggregator {
    op: BenchOp,
    chunk_bytes: usize,
    sum_tp: f64,
    sum_ops: f64,
    sum_lat: f64,
    samples: u64,
}

impl PassAggregator {
    pub fn new(op: BenchOp, chunk_bytes: usize) -> Self {
        Self {
            op,
            chunk_bytes,
            sum_tp: 0.0,
            sum_ops: 0.0,
            sum_lat: 0.0,
            samples: 0,
        }
    }

    pub fn push(&mut self, sample: &BenchSample) {
        self.sum_tp += sample.throughput_mib_s;
        self.sum_ops += sample.ops_per_sec as f64;
        self.sum_lat += sample.latency_us;
        self.samples += 1;
    }

    pub fn finish(self) -> PassSummary {
        let n = self.samples;
        PassSummary {
            op: self.op,
            chunk_bytes: self.chunk_bytes,
            avg_mib_s: avg(self.sum_tp, n),
            avg_ops_s: avg(self.sum_ops, n),
            avg_latency_us: avg(self.sum_lat, n),
            samples: n,
        }
    }
}

fn avg(sum: f64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { sum / n as f64 }
}

/// Columns for one live stats row (CLI applies color per column).
pub fn live_sample_columns(sample: &BenchSample) -> [String; 5] {
    [
        format!("{:6.1}s", sample.elapsed_secs),
        format!("{:8.2} MiB/s", sample.throughput_mib_s),
        format!("{:8} ops/s", sample.ops_per_sec),
        format!("{:8.1} μs", sample.latency_us),
        format_chunk_size(sample.chunk_bytes),
    ]
}

/// One live stats line (no ANSI).
pub fn format_live_sample_line(sample: &BenchSample) -> String {
    live_sample_columns(sample).join("  ")
}

/// Drain a stats channel until closed; invoke `on_sample` for each live update.
pub async fn drain_stats_channel(
    mut rx: mpsc::Receiver<BenchStats>,
    op: BenchOp,
    chunk_bytes: usize,
    mut on_sample: impl FnMut(&BenchSample),
) -> PassSummary {
    let mut agg = PassAggregator::new(op, chunk_bytes);
    while let Some(tuple) = rx.recv().await {
        let sample = BenchSample::from_tuple(tuple);
        on_sample(&sample);
        agg.push(&sample);
    }
    agg.finish()
}

/// Console log line for GUI.
pub fn format_console_log_line(sample: &BenchSample) -> String {
    format!(
        "{}: {:.2} MiB/s, {}: {}, Latency: {:.1} μs, Size: {}",
        sample.op.label(),
        sample.throughput_mib_s,
        sample.op.ops_per_sec_label(),
        sample.ops_per_sec,
        sample.latency_us,
        format_chunk_size(sample.chunk_bytes),
    )
}
