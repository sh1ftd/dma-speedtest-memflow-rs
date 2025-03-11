use std::io::Write;
use std::time::Duration;

pub type ReadSize = usize;
pub type ReadsPerSecond = u64;
pub type Throughput = f64;

#[derive(Debug)]
pub struct ReadingStats {
    pub reads: ReadsPerSecond,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub avg_latency: Duration,
}

#[derive(Debug)]
pub struct TestResult {
    pub read_size: ReadSize,
    pub avg_reads: ReadsPerSecond,
    pub throughput: Throughput,
    pub read_statistics: Vec<ReadingStats>,
    pub total_reads: u64,
    pub test_duration: f64,
    pub max_reads: u64,
    pub min_reads: u64,
}

impl TestResult {
    pub fn new(
        read_size: ReadSize,
        total_reads: u64,
        test_duration: f64,
        max_reads: u64,
        min_reads: u64,
        read_statistics: Vec<ReadingStats>,
    ) -> Self {
        let avg_reads = total_reads as f64 / test_duration;
        let throughput = (avg_reads * read_size as f64) / (1024.0 * 1024.0);

        Self {
            read_size,
            avg_reads: avg_reads.round() as u64,
            throughput,
            read_statistics,
            total_reads,
            test_duration,
            max_reads,
            min_reads,
        }
    }

    pub fn get_latency_range(&self) -> (Duration, Duration) {
        self.read_statistics
            .iter()
            .fold((Duration::from_secs(1), Duration::ZERO), |acc, r| {
                (acc.0.min(r.min_latency), acc.1.max(r.max_latency))
            })
    }

    pub fn get_latency_stats(&self) -> (Duration, Duration, Duration) {
        let (min, max) = self.get_latency_range();

        let avg = if !self.read_statistics.is_empty() {
            let total_nanos: u128 = self
                .read_statistics
                .iter()
                .map(|r| r.avg_latency.as_nanos())
                .sum();
            Duration::from_nanos((total_nanos / self.read_statistics.len() as u128) as u64)
        } else {
            Duration::ZERO
        };

        (min, avg, max)
    }

    pub fn print_summary(&self) {
        println!("\nTest Summary:");
        println!("   Maximum: {} reads/s", self.max_reads);
        println!("   Minimum: {} reads/s", self.min_reads);
        println!("   Average: {:.0} reads/s", self.avg_reads);
        println!("   Total Reads: {}", self.total_reads);
        println!("   Total Time: {:.3}s", self.test_duration);

        println!("\nThroughput:");
        println!("   {:.2} MB/s", self.throughput);
    }

    pub fn print_detailed_results(&self) {
        const DETAILED_RESULTS_FRAME: &str =
            "+---------+--------+--------------------------------+";

        println!("\nResults for {}KB reads:", self.read_size / 1024);
        println!("{}", DETAILED_RESULTS_FRAME);
        println!("| Second  | Reads  | Latency: min │ avg │ max  (µs) |");
        println!("{}", DETAILED_RESULTS_FRAME);

        for (i, stats) in self.read_statistics.iter().enumerate() {
            self.print_reading_row(i + 1, stats);
        }

        println!("{}", DETAILED_RESULTS_FRAME);
        println!();
    }

    fn print_reading_row(&self, second: usize, stats: &ReadingStats) {
        println!(
            "| {:>7} | {:>6} |         {:>4} │{:>4} │ {:<4}      |",
            second,
            stats.reads,
            stats.min_latency.as_micros(),
            stats.avg_latency.as_micros(),
            stats.max_latency.as_micros()
        );
    }
}

pub fn print_progress(
    elapsed: Duration,
    reads: u64,
    speed: f64,
    min_lat: Duration,
    max_lat: Duration,
) -> std::io::Result<()> {
    print!(
        "Progress: {:.1}s - {} reads/s [{:.1} MB/s] [lat: {} │ {}µs]    \r",
        elapsed.as_secs_f64(),
        reads,
        speed,
        min_lat.as_micros(),
        max_lat.as_micros()
    );
    std::io::stdout().flush()
}
