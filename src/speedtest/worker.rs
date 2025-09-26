use super::connector::Connector;
use anyhow::Result;
use memflow::prelude::v1::*;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, task};

use super::initialization;

#[derive(Clone)]
pub struct SpeedTest {
    process: Arc<parking_lot::RwLock<IntoProcessInstanceArcBox<'static>>>,
    test_addr: Address,
}

impl SpeedTest {
    pub fn new(connector: Connector, pcileech_device: String) -> Result<Self> {
        let (process, test_addr) =
            initialization::initialize_speedtest(connector, pcileech_device)?;
        let speedtest = Self {
            process: Arc::new(parking_lot::RwLock::new(process)),
            test_addr,
        };
        Ok(speedtest)
    }

    pub async fn run_test_with_size(
        &self,
        size: usize,
        duration: Duration,
        stats_tx: mpsc::Sender<(f64, u64, f64, usize, f64)>, // (throughput, reads_per_sec, elapsed_secs, read_size, latency_us)
    ) -> Result<()> {
        let mut buffer = vec![0u8; size];
        let start_time = std::time::Instant::now();
        let mut reads_this_interval = 0u64;
        let mut last_update = std::time::Instant::now();
        let mut total_latency = Duration::ZERO;
        let mut latency_count = 0u64;
        let update_interval = Duration::from_millis(100);

        while start_time.elapsed() < duration {
            let read_start = std::time::Instant::now();
            {
                let mut process = self.process.write();
                process.read_raw_into(self.test_addr, &mut buffer)?;
            }
            let latency = read_start.elapsed();
            total_latency += latency;
            latency_count += 1;
            reads_this_interval += 1;

            // Update stats every 100ms
            let now = std::time::Instant::now();

            if now.duration_since(last_update) >= update_interval {
                let interval_duration = now - last_update;
                let interval_secs = interval_duration.as_secs_f64();

                if interval_secs.is_normal() && interval_secs > 0.0 {
                    let reads_per_sec_f64 = reads_this_interval as f64 / interval_secs;
                    let throughput_mib_s = (reads_per_sec_f64 * size as f64) / (1024.0 * 1024.0);
                    let avg_latency_us = if latency_count > 0 {
                        (total_latency.as_nanos() as f64 / latency_count as f64) / 1000.0
                    } else {
                        0.0
                    };
                    let elapsed_secs = start_time.elapsed().as_secs_f64();

                    if stats_tx
                        .send((
                            throughput_mib_s,
                            reads_per_sec_f64.round() as u64,
                            elapsed_secs,
                            size,
                            avg_latency_us,
                        ))
                        .await
                        .is_err()
                    {
                        break;
                    }

                    reads_this_interval = 0;
                    total_latency = Duration::ZERO;
                    latency_count = 0;
                }

                last_update = now;

                // Yield occasionally so we do not starve the scheduler.
                task::yield_now().await;
            }

            if reads_this_interval.is_multiple_of(1024) {
                task::yield_now().await;
            }
        }

        if reads_this_interval > 0 && !stats_tx.is_closed() {
            let now = std::time::Instant::now();
            let interval_duration = now - last_update;
            let interval_secs = interval_duration.as_secs_f64();

            if interval_secs.is_normal() && interval_secs > 0.0 {
                let reads_per_sec_f64 = reads_this_interval as f64 / interval_secs;
                let throughput_mib_s = (reads_per_sec_f64 * size as f64) / (1024.0 * 1024.0);
                let avg_latency_us = if latency_count > 0 {
                    (total_latency.as_nanos() as f64 / latency_count as f64) / 1000.0
                } else {
                    0.0
                };

                let total_elapsed = now.duration_since(start_time).as_secs_f64();

                let _ = stats_tx
                    .send((
                        throughput_mib_s,
                        reads_per_sec_f64.round() as u64,
                        total_elapsed,
                        size,
                        avg_latency_us,
                    ))
                    .await;
            }
        }

        Ok(())
    }
}
