//! CLI benchmarking implementation used by the prefixed CLI binary.
use anyhow::{Result, bail};
use clap::Parser;
use std::{
    io::{self, Write},
    time::Duration,
};
use tokio::sync::mpsc;

use crate::speedtest::{Connector, SpeedTest};

pub const DEFAULT_READ_SIZES: [usize; 4] = [4096, 8192, 16384, 32768];

#[derive(Copy, Clone, Default, Debug, clap::ValueEnum)]
pub enum CliConnector {
    #[default]
    Pcileech,
    Native,
}

impl From<CliConnector> for Connector {
    fn from(h: CliConnector) -> Self {
        match h {
            CliConnector::Pcileech => Connector::Pcileech,
            CliConnector::Native => Connector::Native,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "cli-dma-speedtest-memflow-rs",
    version,
    about = "DMA read benchmark (terminal). Defaults: PCILeech, device FPGA, 10 s per size, read sizes 4–32 KiB. Other granularities are optional: pass --sizes to select them (same idea as ticking sizes in the GUI)."
)]
pub struct CliArgs {
    #[arg(long, value_enum, default_value_t = CliConnector::Pcileech)]
    pub connector: CliConnector,

    #[arg(
        long,
        default_value = "FPGA",
        help = "PCILeech device string (ignored for native connector)."
    )]
    pub device: String,

    #[arg(long, default_value_t = 10, help = "Seconds per read size (1–60).")]
    pub duration: u64,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Optional override: comma-separated read sizes in bytes. If omitted, only the default 4–32 KiB set runs (same as the GUI defaults). When set, this list replaces the default entirely."
    )]
    pub sizes: Option<Vec<usize>>,
}

pub fn print_startup_help() {
    // Keep this terse and column-aligned so defaults are obvious at a glance.
    println!(
        "Usage: cli-dma-speedtest-memflow-rs.exe [OPTIONS]\n\n\
Options:\n\
  {arg:<22} {def:<16} Description\n\
  --------------------------------------------------------------------------\n\
  {carg:<22} {cdef:<16} pcileech | native\n\
  {darg:<22} {ddef:<16} PCILeech device string (ignored for native)\n\
  {targ:<22} {tdef:<16} seconds per read size (1–60)\n\
  {sarg:<22} {sdef:<16} optional override; replaces default list when set\n\
  {harg:<22} {hdef:<16} print clap help\n\
  {varg:<22} {vdef:<16} print version\n",
        arg = "Argument",
        def = "[default]",
        carg = "--connector <CONNECTOR>",
        cdef = "[pcileech]",
        darg = "--device <DEVICE>",
        ddef = "[FPGA]",
        targ = "--duration <SECONDS>",
        tdef = "[10]",
        sarg = "--sizes <CSV_BYTES>",
        sdef = "[4096,8192,16384,32768]",
        harg = "-h, --help",
        hdef = "",
        varg = "-V, --version",
        vdef = "",
    );
}

pub async fn run_headless(args: CliArgs) -> Result<()> {
    let connector: Connector = args.connector.into();
    let duration_secs = args.duration;
    if !(1..=60).contains(&duration_secs) {
        bail!("duration must be between 1 and 60 seconds");
    }

    let sizes: Vec<usize> = match args.sizes {
        Some(s) if s.is_empty() => bail!("sizes must contain at least one value when set"),
        Some(s) => s,
        None => DEFAULT_READ_SIZES.to_vec(),
    };

    let device_trim = args.device.trim();
    if matches!(connector, Connector::Pcileech) && device_trim.is_empty() {
        bail!("PCILeech requires a non-empty --device string");
    }
    let device = if matches!(connector, Connector::Pcileech) {
        device_trim.to_string()
    } else {
        args.device
    };

    println!(
        "connector={} duration={duration_secs}s sizes={sizes:?}",
        connector,
    );

    let test = SpeedTest::new(connector, device)?;

    let mut summaries = Vec::with_capacity(sizes.len());

    for &size in &sizes {
        let label = size_label(size);
        println!("read size {label} ({size} B)");
        let (tx, mut rx) = mpsc::channel(256);
        let print = tokio::spawn(async move {
            let mut sum_tp = 0.0f64;
            let mut sum_rps = 0.0f64;
            let mut sum_lat = 0.0f64;
            let mut samples = 0u64;

            while let Some((tp, rps, elapsed, sz, lat)) = rx.recv().await {
                println!(
                    "  t={elapsed:6.1}s  {tp:8.2} MiB/s  {rps:8} r/s  {lat:8.1} μs  ({})",
                    size_label(sz)
                );
                sum_tp += tp;
                sum_rps += rps as f64;
                sum_lat += lat;
                samples += 1;
            }

            SizeSummary {
                size_bytes: size,
                avg_mib_s: avg(sum_tp, samples),
                avg_reads_s: avg(sum_rps, samples),
                avg_latency_us: avg(sum_lat, samples),
                samples,
            }
        });

        test.run_test_with_size(size, Duration::from_secs(duration_secs), tx)
            .await?;

        let summary = print
            .await
            .map_err(|e| anyhow::anyhow!("printer task: {e}"))?;
        summaries.push(summary);
    }

    print_summary(&summaries);

    Ok(())
}

#[derive(Debug, Clone)]
struct SizeSummary {
    size_bytes: usize,
    avg_mib_s: f64,
    avg_reads_s: f64,
    avg_latency_us: f64,
    samples: u64,
}

fn avg(sum: f64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { sum / n as f64 }
}

fn print_summary(summaries: &[SizeSummary]) {
    if summaries.is_empty() {
        return;
    }

    println!("\nSummary (averages over emitted samples):");
    println!("Size        Avg MiB/s     Avg r/s      Avg μs   Samples");
    println!("----------  ----------  ----------  ----------  -------");

    for s in summaries {
        let size_label = size_label(s.size_bytes);
        println!(
            "{:<10}  {:>10.2}  {:>10.0}  {:>10.1}  {:>7}",
            size_label, s.avg_mib_s, s.avg_reads_s, s.avg_latency_us, s.samples
        );
    }
}

fn size_label(size: usize) -> String {
    if size >= 1024 {
        format!("{} KiB", size / 1024)
    } else {
        format!("{size} B")
    }
}

pub fn prompt_exit() -> Result<()> {
    io::stdout().write_all(b"\nBenchmark finished. Press Enter to exit.\n")?;
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(())
}

#[cfg(windows)]
pub fn ensure_stdio_for_headless() {
    use winapi::um::consoleapi::AllocConsole;
    use winapi::um::wincon::{ATTACH_PARENT_PROCESS, AttachConsole, GetConsoleWindow};
    // SAFETY: standard Win32 console attach/alloc; null check avoids calling on an existing console.
    unsafe {
        if GetConsoleWindow().is_null() {
            let attached = AttachConsole(ATTACH_PARENT_PROCESS) != 0;
            if !attached {
                let _ = AllocConsole();
            }
        }
    }
}

#[cfg(not(windows))]
pub fn ensure_stdio_for_headless() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speedtest::Connector;

    #[test]
    fn default_read_sizes_are_expected() {
        assert_eq!(DEFAULT_READ_SIZES, [4096, 8192, 16384, 32768]);
    }

    #[test]
    fn cli_connector_maps_to_speedtest_connector() {
        let pcileech: Connector = CliConnector::Pcileech.into();
        let native: Connector = CliConnector::Native.into();
        assert!(matches!(pcileech, Connector::Pcileech));
        assert!(matches!(native, Connector::Native));
    }

    #[test]
    fn size_label_formats_bytes_under_1k() {
        assert_eq!(size_label(0), "0 B");
        assert_eq!(size_label(1), "1 B");
        assert_eq!(size_label(512), "512 B");
    }

    #[test]
    fn size_label_formats_kib_for_1024_and_above() {
        assert_eq!(size_label(1024), "1 KiB");
        assert_eq!(size_label(2048), "2 KiB");
        assert_eq!(size_label(4096), "4 KiB");
    }

    #[test]
    fn avg_handles_empty() {
        assert_eq!(avg(123.0, 0), 0.0);
        assert_eq!(avg(10.0, 2), 5.0);
    }
}
