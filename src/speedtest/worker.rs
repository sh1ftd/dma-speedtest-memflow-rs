use super::bench::{BenchMode, BenchOp, BenchStats};
use super::connector::Connector;
use super::initialization::SpeedTestInit;
use super::mem_io::{self, IoAttempt, MAX_IO_RETRIES};
use super::probe_targets::ProbeTargets;
use super::write_target;
use anyhow::Result;
use memflow::prelude::v1::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// Optional hook for retry / skip warnings (GUI console, etc.).
pub type BenchWarnFn = Arc<dyn Fn(&str) + Send + Sync>;

/// Optional hook before each op/size pass (GUI console, CLI headers, etc.).
pub type BenchPassStartFn = Arc<dyn Fn(BenchOp, usize) + Send + Sync>;
use tokio::{sync::mpsc, task};

use super::initialization;

#[derive(Clone)]
pub struct SpeedTest {
    process: Arc<parking_lot::RwLock<IntoProcessInstanceArcBox<'static>>>,
    read_addr: Address,
    write_addr: Option<Address>,
    write_region_bytes: Option<umem>,
    write_verified_bytes: Option<usize>,
    write_restore_bytes: Option<Arc<[u8]>>,
    mode: BenchMode,
    cancel: Arc<AtomicBool>,
}

impl SpeedTest {
    pub fn new(
        connector: Connector,
        pcileech_device: String,
        mode: BenchMode,
        max_chunk_bytes: usize,
    ) -> Result<Self> {
        let SpeedTestInit {
            process,
            read_addr,
            write_addr,
            write_region_bytes,
            write_verified_bytes,
            write_restore_bytes,
        } = initialization::initialize_speedtest(
            connector,
            pcileech_device,
            mode,
            max_chunk_bytes,
        )?;
        Ok(Self {
            process: Arc::new(parking_lot::RwLock::new(process)),
            read_addr,
            write_addr,
            write_region_bytes,
            write_verified_bytes,
            write_restore_bytes: write_restore_bytes.map(Arc::from),
            mode,
            cancel: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn bench_mode(&self) -> BenchMode {
        self.mode
    }

    /// Run each operation enabled by [`Self::bench_mode`] for one chunk size.
    pub async fn run_passes_for_size(
        &self,
        size: usize,
        duration: Duration,
        stats_tx: mpsc::Sender<BenchStats>,
        on_warn: Option<BenchWarnFn>,
        on_pass_start: Option<BenchPassStartFn>,
    ) -> Result<()> {
        for &op in self.mode.ops_for_size() {
            if self.is_cancelled() {
                break;
            }
            if let Some(ref hook) = on_pass_start {
                hook(op, size);
            }
            self.run_test_with_size(op, size, duration, stats_tx.clone(), on_warn.clone())
                .await?;
        }
        Ok(())
    }

    pub fn read_addr(&self) -> Address {
        self.read_addr
    }

    pub fn write_target(&self) -> Option<(Address, umem)> {
        match (self.write_addr, self.write_region_bytes) {
            (Some(addr), Some(bytes)) => Some((addr, bytes)),
            _ => None,
        }
    }

    pub fn probe_targets(&self) -> ProbeTargets {
        ProbeTargets::new(self.read_addr, self.write_addr, self.write_region_bytes)
    }

    pub fn probe_connect_detail_lines(&self) -> Vec<String> {
        self.probe_targets()
            .connect_detail_lines_with_verified(self.write_verified_bytes)
    }

    pub fn restore_write_target(&self) -> Result<()> {
        let (Some(addr), Some(original)) = (self.write_addr, self.write_restore_bytes.as_deref())
        else {
            return Ok(());
        };

        let mut process = self.process.write();
        write_target::restore_write_target(&mut process, addr, original)
    }

    pub async fn run_test_with_size(
        &self,
        op: BenchOp,
        size: usize,
        duration: Duration,
        stats_tx: mpsc::Sender<BenchStats>,
        on_warn: Option<BenchWarnFn>,
    ) -> Result<()> {
        let addr = self.operation_address(op, size)?;
        let mut buffer = prepare_buffer(op, size);

        let start_time = std::time::Instant::now();
        let mut ops_this_interval = 0u64;
        let mut last_update = std::time::Instant::now();
        let mut total_latency = Duration::ZERO;
        let mut latency_count = 0u64;
        let mut skipped_ops = 0u64;
        let mut last_retry_warning = std::time::Instant::now()
            .checked_sub(Duration::from_secs(2))
            .unwrap_or_else(std::time::Instant::now);
        let update_interval = Duration::from_millis(100);

        while start_time.elapsed() < duration && !self.is_cancelled() {
            let op_start = std::time::Instant::now();
            let attempt = {
                let mut process = self.process.write();
                match op {
                    BenchOp::Read => {
                        mem_io::read_raw_into_with_retry(&mut process, addr, &mut buffer)
                    }
                    BenchOp::Write => mem_io::write_raw_with_retry(&mut process, addr, &buffer),
                }
            };

            if attempt == IoAttempt::FailedAfterRetries {
                skipped_ops += 1;
                if last_retry_warning.elapsed() >= Duration::from_secs(1) {
                    let msg = mem_io::retry_exhausted_message(op, MAX_IO_RETRIES);
                    emit_warn(&on_warn, &format!("warning: {msg}"));
                    last_retry_warning = std::time::Instant::now();
                }
                task::yield_now().await;
                continue;
            }

            let latency = op_start.elapsed();
            total_latency += latency;
            latency_count += 1;
            ops_this_interval += 1;

            let now = std::time::Instant::now();

            if now.duration_since(last_update) >= update_interval {
                let interval_duration = now - last_update;
                let interval_secs = interval_duration.as_secs_f64();

                if interval_secs.is_normal() && interval_secs > 0.0 {
                    let update = IntervalStatsUpdate {
                        op,
                        size,
                        ops_this_interval,
                        interval_secs,
                        total_latency,
                        latency_count,
                        start_time,
                    };
                    if send_interval_stats(&update, &stats_tx).await {
                        break;
                    }

                    ops_this_interval = 0;
                    total_latency = Duration::ZERO;
                    latency_count = 0;
                }

                last_update = now;
                task::yield_now().await;
            }

            if ops_this_interval.is_multiple_of(1024) {
                task::yield_now().await;
            }
        }

        if skipped_ops > 0 {
            emit_warn(
                &on_warn,
                &format!(
                    "note: {skipped_ops} DMA {} ops skipped after {MAX_IO_RETRIES} retries each (partial I/O)",
                    op.label()
                ),
            );
        }

        if ops_this_interval > 0 && !stats_tx.is_closed() {
            let now = std::time::Instant::now();
            let interval_duration = now - last_update;
            let interval_secs = interval_duration.as_secs_f64();

            if interval_secs.is_normal() && interval_secs > 0.0 {
                let update = IntervalStatsUpdate {
                    op,
                    size,
                    ops_this_interval,
                    interval_secs,
                    total_latency,
                    latency_count,
                    start_time,
                };
                let _ = send_interval_stats(&update, &stats_tx).await;
            }
        }

        Ok(())
    }
}

impl SpeedTest {
    fn operation_address(&self, op: BenchOp, size: usize) -> Result<Address> {
        if !self.mode.ops_for_size().contains(&op) {
            anyhow::bail!(
                "benchmark op {:?} is not enabled for session mode {:?}",
                op,
                self.mode
            );
        }

        let addr = match op {
            BenchOp::Read => self.read_addr,
            BenchOp::Write => self.write_addr.ok_or_else(|| {
                anyhow::anyhow!(
                    "write benchmark requested but no writable probe target was resolved"
                )
            })?,
        };

        if matches!(op, BenchOp::Write)
            && let Some(region_bytes) = self.write_region_bytes
            && size > region_bytes as usize
        {
            anyhow::bail!(
                "write chunk size {size} B exceeds writable probe region ({} B); reduce enabled sizes or reconnect",
                region_bytes
            );
        }

        Ok(addr)
    }
}

fn prepare_buffer(op: BenchOp, size: usize) -> Vec<u8> {
    let mut buffer = vec![0u8; size];
    if matches!(op, BenchOp::Write) {
        for (i, b) in buffer.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
    }
    buffer
}

struct IntervalStatsUpdate {
    op: BenchOp,
    size: usize,
    ops_this_interval: u64,
    interval_secs: f64,
    total_latency: Duration,
    latency_count: u64,
    start_time: std::time::Instant,
}

fn emit_warn(on_warn: &Option<BenchWarnFn>, message: &str) {
    println!("  {message}");
    if let Some(warn) = on_warn {
        warn(message);
    }
}

/// Returns `true` if the stats channel closed.
async fn send_interval_stats(
    update: &IntervalStatsUpdate,
    stats_tx: &mpsc::Sender<BenchStats>,
) -> bool {
    let ops_per_sec_f64 = update.ops_this_interval as f64 / update.interval_secs;
    let throughput_mib_s = (ops_per_sec_f64 * update.size as f64) / (1024.0 * 1024.0);
    let avg_latency_us = if update.latency_count > 0 {
        (update.total_latency.as_nanos() as f64 / update.latency_count as f64) / 1000.0
    } else {
        0.0
    };
    let elapsed_secs = update.start_time.elapsed().as_secs_f64();

    stats_tx
        .send(BenchStats {
            op: update.op,
            chunk_bytes: update.size,
            elapsed_secs,
            interval_secs: update.interval_secs,
            ops: update.ops_this_interval,
            throughput_mib_s,
            ops_per_sec: ops_per_sec_f64.round() as u64,
            latency_us: avg_latency_us,
        })
        .await
        .is_err()
}
