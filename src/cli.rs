//! CLI benchmarking implementation used by the prefixed CLI binary.
use anyhow::{Result, bail};
use clap::Parser;
use owo_colors::OwoColorize;
use owo_colors::{Stream, Style};
use std::{
    io::{self, IsTerminal, Write},
    time::Duration,
};
use tokio::sync::mpsc;

use crate::bench_config::{
    DEFAULT_CHUNK_SIZES, chunk_sizes_from_optional_csv, default_chunk_sizes_csv, format_chunk_size,
    max_chunk_bytes_in_list, validate_chunk_sizes,
};
use crate::speedtest::{
    BenchMode, BenchOp, Connector, PassSummary, ProbeTargets, SpeedTest, drain_stats_channel,
    live_sample_columns,
};

/// Alias for [`DEFAULT_CHUNK_SIZES`].
pub const DEFAULT_READ_SIZES: [usize; 4] = DEFAULT_CHUNK_SIZES;

/// Visual break between per-size benchmark sections in CLI output.
const BETWEEN_READ_SIZE_SECTIONS: &str =
    "--------------------------------------------------------------------";

fn print_between_read_size_sections(so: Stream) {
    println!(
        "{}",
        BETWEEN_READ_SIZE_SECTIONS.if_supports_color(so, |t| t.dimmed()),
    );
}

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

#[derive(Copy, Clone, Default, Debug, clap::ValueEnum)]
pub enum CliBenchMode {
    #[default]
    Read,
    Write,
    Both,
}

impl From<CliBenchMode> for BenchMode {
    fn from(m: CliBenchMode) -> Self {
        match m {
            CliBenchMode::Read => BenchMode::Read,
            CliBenchMode::Write => BenchMode::Write,
            CliBenchMode::Both => BenchMode::Both,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "cli-dma-speedtest-memflow-rs",
    version,
    about = "DMA benchmark (terminal). Defaults: PCILeech, device FPGA, 10 s per size, chunk sizes 4–32 KiB. Other granularities are optional: pass --sizes to select them (same idea as ticking sizes in the GUI)."
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

    #[arg(long, default_value_t = 10, help = "Seconds per chunk size (1–60).")]
    pub duration: u64,

    #[arg(long, value_enum, default_value_t = CliBenchMode::Read)]
    pub mode: CliBenchMode,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Optional override: comma-separated chunk sizes in bytes. If omitted, only the default 4–32 KiB set runs (same as the GUI defaults). When set, this list replaces the default entirely."
    )]
    pub sizes: Option<Vec<usize>>,
}

pub fn default_cli_args() -> CliArgs {
    CliArgs {
        connector: CliConnector::default(),
        device: "FPGA".to_owned(),
        duration: 10,
        mode: CliBenchMode::Read,
        sizes: None,
    }
}

pub fn print_startup_help() {
    let so = Stream::Stdout;

    println!(
        "{}: {} [OPTIONS]",
        "Usage".if_supports_color(so, |t| t.style(Style::new().cyan().bold())),
        "cli-dma-speedtest-memflow-rs.exe"
            .if_supports_color(so, |t| { t.style(Style::new().bright_white().bold()) }),
    );
    println!();
    println!(
        "{}",
        "Options:".if_supports_color(so, |t| t.style(Style::new().magenta().bold())),
    );
    println!(
        "  {} {} {}",
        "Argument".if_supports_color(so, |t| t.style(Style::new().yellow().bold())),
        format!("{:<16}", "[default]").if_supports_color(so, |t| t.dimmed()),
        "Description".if_supports_color(so, |t| t.white()),
    );
    println!(
        "{}",
        "  --------------------------------------------------------------------------"
            .if_supports_color(so, |t| t.dimmed()),
    );

    let row = |flag: &str, default: &str, desc: &str| {
        println!(
            "  {} {} {}",
            flag.if_supports_color(so, |t| t.yellow()),
            format!("{default:<16}").if_supports_color(so, |t| t.dimmed()),
            desc,
        );
    };

    row("--connector <CONNECTOR>", "[pcileech]", "pcileech | native");
    row(
        "--device <DEVICE>",
        "[FPGA]",
        "PCILeech device string (ignored for native)",
    );
    row(
        "--duration <SECONDS>",
        "[10]",
        "seconds per chunk size (1–60)",
    );
    row("--mode <MODE>", "[read]", "read | write | both");
    row(
        "--sizes <CSV_BYTES>",
        &default_chunk_sizes_csv(),
        "optional override; replaces default list when set",
    );
    row("-h, --help", "", "print clap help");
    row("-V, --version", "", "print version");
}

pub fn print_more_options_hint() {
    let so = Stream::Stdout;
    println!(
        "{}",
        "For more options, run with --help.\n\n".if_supports_color(so, |t| t.dimmed()),
    );
}

const INTERACTIVE_DEFAULTS_PROMPT_SECS: u64 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum LaunchMenuChoice {
    RunDefaults,
    Customize,
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

fn timed_defaults_yes_no_prompt(so: Stream) -> Result<LaunchMenuChoice> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};

    const TICK: Duration = Duration::from_millis(100);

    crossterm::terminal::enable_raw_mode()
        .map_err(|e| anyhow::anyhow!("could not enable raw terminal mode: {e}"))?;
    let _raw_guard = RawModeGuard;

    let deadline =
        std::time::Instant::now() + Duration::from_secs(INTERACTIVE_DEFAULTS_PROMPT_SECS);
    let mut last_shown_secs: Option<u64> = None;

    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            print!("\r\x1b[2K");
            io::stdout().flush()?;
            println!();
            return Ok(LaunchMenuChoice::RunDefaults);
        }

        let secs = left.as_secs() + u64::from(left.subsec_nanos() > 0);
        if last_shown_secs != Some(secs) {
            last_shown_secs = Some(secs);
            print!(
                "\r\x1b[2K{} {}",
                "Run with defaults? [Y/n]".if_supports_color(so, |t| t.style(Style::new().bold())),
                format!("({secs}s)").if_supports_color(so, |t| t.dimmed()),
            );
            io::stdout().flush()?;
        }

        let poll_wait = TICK.min(left);
        if event::poll(poll_wait)?
            && let Event::Key(key) = event::read()?
        {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    print!("\r\x1b[2K");
                    io::stdout().flush()?;
                    println!();
                    return Ok(LaunchMenuChoice::RunDefaults);
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    print!("\r\x1b[2K");
                    io::stdout().flush()?;
                    println!();
                    return Ok(LaunchMenuChoice::Customize);
                }
                KeyCode::Esc => {
                    print!("\r\x1b[2K");
                    io::stdout().flush()?;
                    println!();
                    return Ok(LaunchMenuChoice::RunDefaults);
                }
                _ => {}
            }
        }
    }
}

/// Prompt Y/n for defaults with a short idle countdown (Yes). Non-TTY stdin/stdout returns [`default_cli_args`] immediately.
pub fn interactive_launch_cli_args() -> Result<CliArgs> {
    use dialoguer::{Input, Select, theme::ColorfulTheme};

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        eprintln!(
            "{}",
            "stdin or stdout is not a terminal; using default benchmark settings (pass CLI flags to configure)."
                .if_supports_color(Stream::Stderr, |t| t.dimmed()),
        );
        return Ok(default_cli_args());
    }

    let so = Stream::Stdout;
    let prompt_choice = timed_defaults_yes_no_prompt(so)?;

    match prompt_choice {
        LaunchMenuChoice::RunDefaults => {
            print_more_options_hint();
            Ok(default_cli_args())
        }
        LaunchMenuChoice::Customize => {
            let so = Stream::Stdout;
            println!(
                "{}",
                "Customize — defaults if you press Enter on each prompt."
                    .if_supports_color(so, |t| t.style(Style::new().cyan().bold())),
            );
            println!(
                "{}",
                "Defaults: PCILeech, device FPGA, 10 s per chunk size, sizes 4–32 KiB."
                    .if_supports_color(so, |t| t.dimmed()),
            );
            println!();

            let theme = ColorfulTheme::default();

            let mode_idx = Select::with_theme(&theme)
                .with_prompt("Benchmark mode")
                .items(["read", "write", "both"])
                .default(0)
                .interact()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let mode = match mode_idx {
                0 => CliBenchMode::Read,
                1 => CliBenchMode::Write,
                2 => CliBenchMode::Both,
                _ => CliBenchMode::Read,
            };

            let connector_idx = Select::with_theme(&theme)
                .with_prompt("Connector")
                .items(["pcileech", "native"])
                .default(0)
                .interact()
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let connector = if connector_idx == 0 {
                CliConnector::Pcileech
            } else {
                CliConnector::Native
            };

            let duration = loop {
                let s: String = Input::with_theme(&theme)
                    .with_prompt("Seconds per chunk size (1–60)")
                    .default("10".to_string())
                    .interact_text()
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                match s.trim().parse::<u64>() {
                    Ok(n) if (1..=60).contains(&n) => break n,
                    _ => eprintln!("Please enter an integer from 1 to 60."),
                }
            };

            let device = if matches!(connector, CliConnector::Pcileech) {
                Input::with_theme(&theme)
                    .with_prompt("PCILeech device")
                    .default("FPGA".to_string())
                    .interact_text()
                    .map_err(|e| anyhow::anyhow!("{e}"))?
            } else {
                default_cli_args().device
            };

            let sizes_default = default_chunk_sizes_csv();

            let sizes_str: String = Input::with_theme(&theme)
                .with_prompt("Chunk sizes in bytes, comma-separated")
                .default(sizes_default)
                .interact_text()
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            let sizes = chunk_sizes_from_optional_csv(sizes_str.trim())?;

            Ok(CliArgs {
                connector,
                device,
                duration,
                mode,
                sizes,
            })
        }
    }
}

pub async fn run_headless(args: CliArgs) -> Result<()> {
    let connector: Connector = args.connector.into();
    let bench_mode: BenchMode = args.mode.into();
    let duration_secs = args.duration;
    if !(1..=60).contains(&duration_secs) {
        bail!("duration must be between 1 and 60 seconds");
    }

    let sizes: Vec<usize> = match args.sizes {
        Some(s) => s,
        None => DEFAULT_CHUNK_SIZES.to_vec(),
    };
    validate_chunk_sizes(&sizes)?;

    let device_trim = args.device.trim();
    if matches!(connector, Connector::Pcileech) && device_trim.is_empty() {
        bail!("PCILeech requires a non-empty --device string");
    }
    let device = if matches!(connector, Connector::Pcileech) {
        device_trim.to_string()
    } else {
        args.device
    };

    let so = Stream::Stdout;
    let mode_str = match args.mode {
        CliBenchMode::Read => "read",
        CliBenchMode::Write => "write",
        CliBenchMode::Both => "both",
    };
    println!(
        "{}={} {}={} {}={} {}={}",
        "connector".if_supports_color(so, |t| t.cyan()),
        connector
            .to_string()
            .if_supports_color(so, |t| { t.style(Style::new().bright_white().bold()) }),
        "duration".if_supports_color(so, |t| t.cyan()),
        format!("{duration_secs}s")
            .if_supports_color(so, |t| { t.style(Style::new().bright_white().bold()) }),
        "mode".if_supports_color(so, |t| t.cyan()),
        mode_str.if_supports_color(so, |t| t.bright_white()),
        "sizes".if_supports_color(so, |t| t.cyan()),
        format!("{sizes:?}").if_supports_color(so, |t| t.bright_white()),
    );

    let max_chunk = max_chunk_bytes_in_list(&sizes);
    let test = SpeedTest::new(connector, device, bench_mode, max_chunk)?;
    print_probe_details(so, &test.probe_connect_detail_lines());

    let mut summaries = Vec::new();
    let mut first_block = true;

    for &size in &sizes {
        for &op in test.bench_mode().ops_for_size() {
            if !first_block {
                print_between_read_size_sections(so);
            }
            first_block = false;

            let label = format_chunk_size(size);
            println!(
                "{} {} {} ({})",
                op.label()
                    .if_supports_color(so, |t| t.style(Style::new().green().bold())),
                "size".if_supports_color(so, |t| t.white()),
                label.if_supports_color(so, |t| { t.style(Style::new().bright_yellow().bold()) }),
                format!("{size} B").if_supports_color(so, |t| t.dimmed()),
            );
            print_op_probe_detail(so, &test.probe_targets(), op, size);

            let (tx, rx) = mpsc::channel(256);
            let print = tokio::spawn(async move {
                drain_stats_channel(rx, op, size, |sample| {
                    print_colored_live_sample(sample);
                })
                .await
            });

            test.run_test_with_size(op, size, Duration::from_secs(duration_secs), tx, None)
                .await?;

            let summary = print
                .await
                .map_err(|e| anyhow::anyhow!("printer task: {e}"))?;
            summaries.push(summary);
        }
    }

    print_summary(&summaries);

    Ok(())
}

fn print_colored_live_sample(sample: &crate::speedtest::BenchSample) {
    let so = Stream::Stdout;
    let [t, mib, ops, lat, sz] = live_sample_columns(sample);
    println!(
        "  {}  {}  {}  {}  {}",
        t.if_supports_color(so, |c| c.style(Style::new().bright_blue().bold())),
        mib.if_supports_color(so, |c| c.style(Style::new().bright_green().bold())),
        ops.if_supports_color(so, |c| c.cyan()),
        lat.if_supports_color(so, |c| c.magenta()),
        sz.if_supports_color(so, |c| c.style(Style::new().bright_yellow().bold())),
    );
}

fn print_summary(summaries: &[PassSummary]) {
    if summaries.is_empty() {
        return;
    }

    let so = Stream::Stdout;
    let groups = summary_groups(summaries);
    println!(
        "\n{}",
        "Summary (averages over emitted samples):"
            .if_supports_color(so, |t| t.style(Style::new().bright_blue().bold())),
    );

    for (idx, group) in groups.iter().enumerate() {
        if let Some(title) = group.title {
            println!();
            println!(
                "{}",
                title.if_supports_color(so, |t| t.style(Style::new().green().bold()))
            );
        } else if idx > 0 {
            println!();
        }

        print_summary_table(so, group.op, &group.rows);
    }
}

struct SummaryGroup<'a> {
    title: Option<&'static str>,
    op: Option<BenchOp>,
    rows: Vec<&'a PassSummary>,
}

fn summary_groups(summaries: &[PassSummary]) -> Vec<SummaryGroup<'_>> {
    let has_read = summaries.iter().any(|s| matches!(s.op, BenchOp::Read));
    let has_write = summaries.iter().any(|s| matches!(s.op, BenchOp::Write));

    if has_read && has_write {
        return vec![
            SummaryGroup {
                title: Some("Read summary:"),
                op: Some(BenchOp::Read),
                rows: summaries
                    .iter()
                    .filter(|s| matches!(s.op, BenchOp::Read))
                    .collect(),
            },
            SummaryGroup {
                title: Some("Write summary:"),
                op: Some(BenchOp::Write),
                rows: summaries
                    .iter()
                    .filter(|s| matches!(s.op, BenchOp::Write))
                    .collect(),
            },
        ];
    }

    vec![SummaryGroup {
        title: None,
        op: None,
        rows: summaries.iter().collect(),
    }]
}

fn print_summary_table(so: Stream, group_op: Option<BenchOp>, summaries: &[&PassSummary]) {
    if let Some(op) = group_op {
        let ops_heading = format!("Avg {}", op.ops_per_sec_label());
        println!(
            "{}  {}  {}  {}  {}",
            format!("{:<10}", "Size")
                .if_supports_color(so, |t| t.style(Style::new().bright_yellow().bold())),
            format!("{:>10}", "Avg MiB/s")
                .if_supports_color(so, |t| t.style(Style::new().bright_green().bold())),
            format!("{ops_heading:>12}")
                .if_supports_color(so, |t| t.style(Style::new().cyan().bold())),
            format!("{:>10}", "Avg μs")
                .if_supports_color(so, |t| t.style(Style::new().magenta().bold())),
            format!("{:>7}", "Samples").if_supports_color(so, |t| t.style(Style::new().bold())),
        );
        println!(
            "{}  {}  {}  {}  {}",
            format!("{:<10}", "----------").if_supports_color(so, |t| t.dimmed()),
            format!("{:>10}", "----------").if_supports_color(so, |t| t.dimmed()),
            format!("{:>12}", "------------").if_supports_color(so, |t| t.dimmed()),
            format!("{:>10}", "----------").if_supports_color(so, |t| t.dimmed()),
            format!("{:>7}", "-------").if_supports_color(so, |t| t.dimmed()),
        );

        for &s in summaries {
            let sz = format!("{:<10}", format_chunk_size(s.chunk_bytes));
            let mib = format!("{:>10.2}", s.avg_mib_s);
            let ops = format!("{:>12.0}", s.avg_ops_s);
            let lat = format!("{:>10.1}", s.avg_latency_us);
            let n = format!("{:>7}", s.samples);
            println!(
                "{}  {}  {}  {}  {}",
                sz.if_supports_color(so, |t| t.style(Style::new().bright_yellow().bold())),
                mib.if_supports_color(so, |t| t.style(Style::new().bright_green().bold())),
                ops.if_supports_color(so, |t| t.cyan()),
                lat.if_supports_color(so, |t| t.magenta()),
                n.if_supports_color(so, |t| t.bright_white()),
            );
        }
        return;
    }

    println!(
        "{}  {}  {}  {}  {}  {}",
        format!("{:<6}", "Op").if_supports_color(so, |t| t.style(Style::new().green().bold())),
        format!("{:<10}", "Size")
            .if_supports_color(so, |t| t.style(Style::new().bright_yellow().bold())),
        format!("{:>10}", "Avg MiB/s")
            .if_supports_color(so, |t| t.style(Style::new().bright_green().bold())),
        format!("{:>10}", "Avg ops/s")
            .if_supports_color(so, |t| t.style(Style::new().cyan().bold())),
        format!("{:>10}", "Avg μs")
            .if_supports_color(so, |t| t.style(Style::new().magenta().bold())),
        format!("{:>7}", "Samples").if_supports_color(so, |t| t.style(Style::new().bold())),
    );
    println!(
        "{}  {}  {}  {}  {}  {}",
        format!("{:<6}", "------").if_supports_color(so, |t| t.dimmed()),
        format!("{:<10}", "----------").if_supports_color(so, |t| t.dimmed()),
        format!("{:>10}", "----------").if_supports_color(so, |t| t.dimmed()),
        format!("{:>10}", "----------").if_supports_color(so, |t| t.dimmed()),
        format!("{:>10}", "----------").if_supports_color(so, |t| t.dimmed()),
        format!("{:>7}", "-------").if_supports_color(so, |t| t.dimmed()),
    );

    for &s in summaries {
        let op = format!("{:<6}", s.op.label());
        let sz = format!("{:<10}", format_chunk_size(s.chunk_bytes));
        let mib = format!("{:>10.2}", s.avg_mib_s);
        let ops = format!("{:>10.0}", s.avg_ops_s);
        let lat = format!("{:>10.1}", s.avg_latency_us);
        let n = format!("{:>7}", s.samples);
        println!(
            "{}  {}  {}  {}  {}  {}",
            op.if_supports_color(so, |t| t.style(Style::new().green().bold())),
            sz.if_supports_color(so, |t| t.style(Style::new().bright_yellow().bold())),
            mib.if_supports_color(so, |t| t.style(Style::new().bright_green().bold())),
            ops.if_supports_color(so, |t| t.cyan()),
            lat.if_supports_color(so, |t| t.magenta()),
            n.if_supports_color(so, |t| t.bright_white()),
        );
    }
}

fn print_probe_details(so: Stream, detail_lines: &[String]) {
    println!();
    println!(
        "{}",
        "Probe targets (fixed; not configurable):"
            .if_supports_color(so, |t| t.style(Style::new().bright_blue().bold())),
    );
    for line in detail_lines {
        println!("  {}", line.if_supports_color(so, |t| t.bright_white()),);
    }
    println!();
}

fn print_op_probe_detail(so: Stream, targets: &ProbeTargets, op: BenchOp, chunk_bytes: usize) {
    let detail = match op {
        BenchOp::Read => Some(targets.format_read_pass(chunk_bytes)),
        BenchOp::Write => targets.format_write_pass(chunk_bytes),
    };
    if let Some(line) = detail {
        println!("  {}", line.if_supports_color(so, |t| t.dimmed()),);
    }
}

pub fn prompt_exit() -> Result<()> {
    println!(
        "\n{} {}",
        "Benchmark finished."
            .if_supports_color(Stream::Stdout, |t| t.style(Style::new().green().bold())),
        "Press Enter to exit.".if_supports_color(Stream::Stdout, |t| t.dimmed()),
    );
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
    use crate::speedtest::{BenchMode, Connector};

    #[test]
    fn cli_connector_maps_to_speedtest_connector() {
        let pcileech: Connector = CliConnector::Pcileech.into();
        let native: Connector = CliConnector::Native.into();
        assert!(matches!(pcileech, Connector::Pcileech));
        assert!(matches!(native, Connector::Native));
    }

    #[test]
    fn default_cli_args_matches_clap_equivalent() {
        let d = default_cli_args();
        assert!(matches!(d.connector, CliConnector::Pcileech));
        assert_eq!(d.device, "FPGA");
        assert_eq!(d.duration, 10);
        assert!(matches!(d.mode, CliBenchMode::Read));
        assert!(d.sizes.is_none());
    }

    #[test]
    fn cli_bench_mode_maps_to_speedtest_bench_mode() {
        let read: BenchMode = CliBenchMode::Read.into();
        let write: BenchMode = CliBenchMode::Write.into();
        let both: BenchMode = CliBenchMode::Both.into();
        assert!(matches!(read, BenchMode::Read));
        assert!(matches!(write, BenchMode::Write));
        assert!(matches!(both, BenchMode::Both));
    }

    #[test]
    fn clap_parses_mode_flag() {
        use clap::Parser;

        let read = CliArgs::parse_from(["cli-dma-speedtest", "--mode", "read"]);
        assert!(matches!(read.mode, CliBenchMode::Read));

        let write = CliArgs::parse_from(["cli-dma-speedtest", "--mode", "write"]);
        assert!(matches!(write.mode, CliBenchMode::Write));

        let both = CliArgs::parse_from(["cli-dma-speedtest", "--mode", "both"]);
        assert!(matches!(both.mode, CliBenchMode::Both));
    }

    #[tokio::test]
    async fn run_headless_rejects_zero_size_before_connecting() {
        let args = CliArgs {
            sizes: Some(vec![0]),
            ..default_cli_args()
        };

        let err = run_headless(args).await.unwrap_err();
        assert!(err.to_string().contains("chunk sizes must be positive"));
    }

    fn pass_summary(op: BenchOp, chunk_bytes: usize) -> PassSummary {
        PassSummary {
            op,
            chunk_bytes,
            avg_mib_s: 1.0,
            avg_ops_s: 2.0,
            avg_latency_us: 3.0,
            samples: 4,
        }
    }

    #[test]
    fn summary_groups_split_read_and_write_rows_when_mixed() {
        let summaries = vec![
            pass_summary(BenchOp::Read, 4096),
            pass_summary(BenchOp::Write, 4096),
            pass_summary(BenchOp::Read, 8192),
            pass_summary(BenchOp::Write, 8192),
        ];

        let groups = summary_groups(&summaries);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].title, Some("Read summary:"));
        assert_eq!(groups[1].title, Some("Write summary:"));
        assert_eq!(groups[0].op, Some(BenchOp::Read));
        assert_eq!(groups[1].op, Some(BenchOp::Write));
        assert_eq!(groups[0].rows.len(), 2);
        assert_eq!(groups[1].rows.len(), 2);
        assert!(groups[0].rows.iter().all(|s| s.op == BenchOp::Read));
        assert!(groups[1].rows.iter().all(|s| s.op == BenchOp::Write));
    }

    #[test]
    fn summary_groups_keep_single_op_summary_unsplit() {
        let summaries = vec![
            pass_summary(BenchOp::Read, 4096),
            pass_summary(BenchOp::Read, 8192),
        ];

        let groups = summary_groups(&summaries);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].title, None);
        assert_eq!(groups[0].op, None);
        assert_eq!(groups[0].rows.len(), 2);
    }
}
