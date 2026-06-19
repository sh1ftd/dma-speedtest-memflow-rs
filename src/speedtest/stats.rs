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
    pub interval_secs: f64,
    pub ops: u64,
    pub chunk_bytes: usize,
    pub latency_us: f64,
}

impl BenchSample {
    pub fn from_stats(stats: BenchStats) -> Self {
        Self {
            op: stats.op,
            throughput_mib_s: stats.throughput_mib_s,
            ops_per_sec: stats.ops_per_sec,
            elapsed_secs: stats.elapsed_secs,
            interval_secs: stats.interval_secs,
            ops: stats.ops,
            chunk_bytes: stats.chunk_bytes,
            latency_us: stats.latency_us,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PassSummary {
    pub op: BenchOp,
    pub chunk_bytes: usize,
    pub min_mib_s: f64,
    pub avg_mib_s: f64,
    pub max_mib_s: f64,
    pub min_ops_s: f64,
    pub avg_ops_s: f64,
    pub max_ops_s: f64,
    pub min_latency_us: f64,
    pub avg_latency_us: f64,
    pub max_latency_us: f64,
    pub samples: u64,
    pub total_ops: u64,
    pub measured_secs: f64,
}

#[derive(Debug, Clone)]
pub struct PassAggregator {
    op: BenchOp,
    chunk_bytes: usize,
    weighted_tp: f64,
    weighted_latency: f64,
    latency_weight_ops: u64,
    min_tp: f64,
    max_tp: f64,
    min_ops_rate: f64,
    max_ops_rate: f64,
    min_latency: f64,
    max_latency: f64,
    samples: u64,
    total_ops: u64,
    measured_secs: f64,
}

impl PassAggregator {
    pub fn new(op: BenchOp, chunk_bytes: usize) -> Self {
        Self {
            op,
            chunk_bytes,
            weighted_tp: 0.0,
            weighted_latency: 0.0,
            latency_weight_ops: 0,
            min_tp: f64::INFINITY,
            max_tp: 0.0,
            min_ops_rate: f64::INFINITY,
            max_ops_rate: 0.0,
            min_latency: f64::INFINITY,
            max_latency: 0.0,
            samples: 0,
            total_ops: 0,
            measured_secs: 0.0,
        }
    }

    pub fn push(&mut self, sample: &BenchSample) {
        let interval_secs = sample.interval_secs.max(0.0);
        let ops_rate = if interval_secs > 0.0 {
            sample.ops as f64 / interval_secs
        } else {
            0.0
        };
        self.weighted_tp += sample.throughput_mib_s * interval_secs;
        self.weighted_latency += sample.latency_us * sample.ops as f64;
        self.latency_weight_ops = self.latency_weight_ops.saturating_add(sample.ops);
        self.min_tp = self.min_tp.min(sample.throughput_mib_s);
        self.max_tp = self.max_tp.max(sample.throughput_mib_s);
        self.min_ops_rate = self.min_ops_rate.min(ops_rate);
        self.max_ops_rate = self.max_ops_rate.max(ops_rate);
        self.min_latency = self.min_latency.min(sample.latency_us);
        self.max_latency = self.max_latency.max(sample.latency_us);
        self.samples += 1;
        self.total_ops = self.total_ops.saturating_add(sample.ops);
        self.measured_secs += interval_secs;
    }

    pub fn is_for(&self, op: BenchOp, chunk_bytes: usize) -> bool {
        self.op == op && self.chunk_bytes == chunk_bytes
    }

    pub fn summary(&self) -> PassSummary {
        let n = self.samples;
        PassSummary {
            op: self.op,
            chunk_bytes: self.chunk_bytes,
            min_mib_s: finite_or_zero(self.min_tp, n),
            avg_mib_s: weighted_avg(self.weighted_tp, self.measured_secs),
            max_mib_s: finite_or_zero(self.max_tp, n),
            min_ops_s: finite_or_zero(self.min_ops_rate, n),
            avg_ops_s: weighted_avg(self.total_ops as f64, self.measured_secs),
            max_ops_s: finite_or_zero(self.max_ops_rate, n),
            min_latency_us: finite_or_zero(self.min_latency, n),
            avg_latency_us: if self.latency_weight_ops == 0 {
                0.0
            } else {
                self.weighted_latency / self.latency_weight_ops as f64
            },
            max_latency_us: finite_or_zero(self.max_latency, n),
            samples: n,
            total_ops: self.total_ops,
            measured_secs: self.measured_secs,
        }
    }

    pub fn finish(self) -> PassSummary {
        self.summary()
    }
}

fn weighted_avg(weighted_sum: f64, total_weight: f64) -> f64 {
    if total_weight <= 0.0 {
        0.0
    } else {
        weighted_sum / total_weight
    }
}

fn finite_or_zero(value: f64, n: u64) -> f64 {
    if n == 0 || !value.is_finite() {
        0.0
    } else {
        value
    }
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
    while let Some(stats) = rx.recv().await {
        let sample = BenchSample::from_stats(stats);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        throughput: f64,
        ops_per_sec: u64,
        latency: f64,
        interval: f64,
        ops: u64,
    ) -> BenchSample {
        BenchSample {
            op: BenchOp::Read,
            throughput_mib_s: throughput,
            ops_per_sec,
            elapsed_secs: interval,
            interval_secs: interval,
            ops,
            chunk_bytes: 4096,
            latency_us: latency,
        }
    }

    #[test]
    fn pass_summary_uses_weighted_rates_and_ops_weighted_latency() {
        let mut agg = PassAggregator::new(BenchOp::Read, 4096);
        agg.push(&sample(100.0, 1, 10.0, 1.0, 1000));
        agg.push(&sample(300.0, 1, 30.0, 3.0, 9000));

        let summary = agg.finish();

        assert_eq!(summary.samples, 2);
        assert_eq!(summary.total_ops, 10_000);
        assert_eq!(summary.measured_secs, 4.0);
        assert_eq!(summary.min_mib_s, 100.0);
        assert_eq!(summary.max_mib_s, 300.0);
        assert_eq!(summary.avg_mib_s, 250.0);
        assert_eq!(summary.min_ops_s, 1000.0);
        assert_eq!(summary.avg_ops_s, 2500.0);
        assert_eq!(summary.max_ops_s, 3000.0);
        assert_eq!(summary.avg_latency_us, 28.0);
    }
}
