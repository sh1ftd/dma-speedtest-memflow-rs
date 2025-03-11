use super::*;
use crate::speedtest::stats::{
    print_progress, ReadSize, ReadingStats, ReadsPerSecond, TestResult, Throughput,
};
use std::{io::Write, sync::Arc, time::Instant};
use tokio::sync::mpsc;
pub(super) struct TestRunner {
    process: Arc<parking_lot::RwLock<IntoProcessInstanceArcBox<'static>>>,
    test_addr: Address,
}

impl TestRunner {
    pub fn new(
        process: Arc<parking_lot::RwLock<IntoProcessInstanceArcBox<'static>>>,
        test_addr: Address,
    ) -> Self {
        Self { process, test_addr }
    }

    pub async fn run_tests(&mut self, duration: Duration) -> Result<()> {
        let sizes = [4096, 8192, 16384, 32768];
        let (best_throughput_size, best_reads_size) = self.determine_best_sizes(&sizes)?;

        // Only test the best performing sizes
        let sizes_to_test = if best_throughput_size != best_reads_size {
            vec![best_throughput_size, best_reads_size]
        } else {
            vec![best_throughput_size]
        };

        let mut results = Vec::new();
        for &test_size in &sizes_to_test {
            let result = self.test_size(test_size, duration).await?;
            results.push(result);
        }

        self.print_final_results(&results, best_throughput_size, best_reads_size);

        println!("\nPress Enter to exit...");
        std::io::stdin().read_line(&mut String::new())?;
        Ok(())
    }

    fn quick_test_size(&self, size: ReadSize) -> Result<(Throughput, ReadsPerSecond)> {
        let mut buffer = vec![0u8; size];
        let mut reads = 0u64;
        let test_start = Instant::now();
        let test_duration = Duration::from_secs(2);
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: u32 = 3;

        while test_start.elapsed() < test_duration {
            let result = {
                let mut process = self.process.write();
                process.read_raw_into(self.test_addr, &mut buffer)
            };

            match result {
                Ok(_) => {
                    reads += 1;
                    consecutive_errors = 0;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        return Err(anyhow::anyhow!(
                            "Failed to read memory after {} consecutive attempts: {}",
                            MAX_CONSECUTIVE_ERRORS,
                            e
                        ));
                    }
                    // Small delay before retrying
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
            }
        }

        let elapsed_secs = test_start.elapsed().as_secs_f64();
        let reads_per_sec = (reads as f64 / elapsed_secs) as u64;
        let throughput = (reads as f64 * size as f64) / elapsed_secs / (1024.0 * 1024.0);

        Ok((throughput, reads_per_sec))
    }

    fn determine_best_sizes(&self, sizes: &[ReadSize]) -> Result<(ReadSize, ReadSize)> {
        let mut best_throughput_size = sizes[0];
        let mut best_throughput = 0f64;
        let mut best_reads_size = sizes[0];
        let mut best_reads_per_sec = 0u64;

        println!("\nTesting different read sizes...");

        for &size in sizes {
            match self.quick_test_size(size) {
                Ok((throughput, reads)) => {
                    println!(
                        "   Size: {:5}KB - Throughput: {:6.1} MB/s ({:4} reads/s)",
                        size / 1024,
                        throughput,
                        reads
                    );

                    if throughput > best_throughput {
                        best_throughput = throughput;
                        best_throughput_size = size;
                    }
                    if reads > best_reads_per_sec {
                        best_reads_per_sec = reads;
                        best_reads_size = size;
                    }
                }
                Err(e) => {
                    println!("   Size: {:5}KB - Failed: {}", size / 1024, e);
                    // If we fail with a larger size, try the next smaller one
                    continue;
                }
            }
        }

        if best_throughput == 0f64 || best_reads_per_sec == 0 {
            return Err(anyhow::anyhow!("Could not find any working read sizes"));
        }

        println!("\nAnalysis:");
        println!(
            "   Best Throughput: {:.1} MB/s with {}KB reads",
            best_throughput,
            best_throughput_size / 1024
        );
        println!(
            "   Best Reads/sec: {} with {}KB reads",
            best_reads_per_sec,
            best_reads_size / 1024
        );

        Ok((best_throughput_size, best_reads_size))
    }

    async fn test_size(&self, size: ReadSize, duration: Duration) -> Result<TestResult> {
        println!("\nTesting {}KB reads:", size / 1024);

        let size_u64 = size as u64;
        let process = self.process.clone();
        let addr = self.test_addr;

        let (tx, mut rx) = mpsc::channel(1000);

        let reader_handle = tokio::spawn(async move {
            let mut buffer = vec![0u8; size];
            let start = Instant::now();

            // Add small buffer to ensure we capture full duration
            let test_duration = duration + Duration::from_millis(100);

            while start.elapsed() < test_duration {
                let before_read = Instant::now();
                {
                    let mut process = process.write();
                    process.read_raw_into(addr, &mut buffer)?;
                }
                let latency = before_read.elapsed();
                if tx.send(latency).await.is_err() {
                    break;
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        // Track statistics and report progress
        let mut last_report = Instant::now();
        let mut last_second = Instant::now();
        let report_interval = Duration::from_millis(100);
        let second_interval = Duration::from_secs(1);
        let start = Instant::now();
        let mut reads_this_second = 0u64;
        let mut reads_this_interval = 0u64;
        let mut max_reads = 0u64;
        let mut min_reads = 0u64;
        let mut total_reads = 0u64;
        let mut readings = Vec::new();
        let mut max_latency = Duration::ZERO;
        let mut min_latency = Duration::MAX;
        let mut total_latency = Duration::ZERO;
        let mut latency_count = 0u64;

        print!(
            "Progress: {:.1}s/{:.1}s - -- reads/s [-.-- MB/s] (lat: ---µs)    \r",
            start.elapsed().as_secs_f64(),
            duration.as_secs_f64()
        );
        std::io::stdout().flush()?;

        while let Some(latency) = rx.recv().await {
            if !latency.is_zero() {
                max_latency = max_latency.max(latency);
                min_latency = min_latency.min(latency);
                total_latency += latency;
                latency_count += 1;
            }

            reads_this_second += 1;
            reads_this_interval += 1;
            total_reads += 1;

            if last_report.elapsed() >= report_interval {
                let interval_time = last_report.elapsed().as_secs_f64();
                let current_reads = (reads_this_interval as f64 / interval_time).round() as u64;
                let current_speed = (current_reads as f64 * size_u64 as f64) / (1024.0 * 1024.0);

                let _ = print_progress(
                    start.elapsed(),
                    current_reads,
                    current_speed,
                    min_latency,
                    max_latency,
                );

                reads_this_interval = 0;
                last_report = Instant::now();
            }

            if last_second.elapsed() >= second_interval {
                let current_seconds = start.elapsed().as_secs_f64();
                max_reads = max_reads.max(reads_this_second);
                min_reads = if current_seconds <= 1.0 {
                    reads_this_second
                } else {
                    min_reads.min(reads_this_second)
                };

                let avg_latency = if latency_count > 0 {
                    Duration::from_nanos((total_latency.as_nanos() / latency_count as u128) as u64)
                } else {
                    Duration::ZERO
                };

                readings.push(ReadingStats {
                    reads: reads_this_second,
                    min_latency,
                    max_latency,
                    avg_latency,
                });

                reads_this_second = 0;
                min_latency = Duration::MAX;
                max_latency = Duration::ZERO;
                total_latency = Duration::ZERO;
                latency_count = 0;
                last_second = Instant::now();
            }
        }

        reader_handle.await??;

        let final_seconds = duration.as_secs_f64();

        let result = TestResult::new(
            size,
            total_reads,
            final_seconds,
            max_reads,
            min_reads,
            readings,
        );

        result.print_detailed_results();
        result.print_summary();

        Ok(result)
    }

    fn print_final_results(
        &self,
        results: &[TestResult],
        best_throughput_size: ReadSize,
        best_reads_size: ReadSize,
    ) {
        println!("\nFinal Results");
        println!("-------------------------------------------------------------------------------");
        println!("Best          Size       Reads/s      MB/s      Latency: min │  avg │ max  (µs)");
        println!("-------------------------------------------------------------------------------");

        for result in results {
            let row_label = match (result.read_size, best_throughput_size, best_reads_size) {
                (s, bt, br) if s == bt && s == br => "Both",
                (s, bt, _) if s == bt => "Throughput",
                (s, _, br) if s == br => "Reads",
                _ => "",
            };

            let (min_latency, avg_latency, max_latency) = result.get_latency_stats();

            println!(
                "{:<12} {:3}KB     {:8.0}    {:8.1}             {:4} │ {:4} │ {:4}",
                row_label,
                result.read_size / 1024,
                result.avg_reads,
                result.throughput,
                min_latency.as_micros(),
                avg_latency.as_micros(),
                max_latency.as_micros()
            );
        }
        println!("-------------------------------------------------------------------------------");
    }
}
