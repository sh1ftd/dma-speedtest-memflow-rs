use anyhow::Result;
use clap::Parser;
use dma_speedtest_memflow_rs::cli::{
    CliArgs, ensure_stdio_for_headless, interactive_launch_cli_args, print_startup_help,
    prompt_exit, run_headless,
};
use owo_colors::OwoColorize;
use owo_colors::{Stream, Style};

#[tokio::main]
async fn main() -> Result<()> {
    ensure_stdio_for_headless();

    let user_arg_count = std::env::args_os().skip(1).count();

    // Print help on startup when non-interactive so available flags are visible by default.
    // Avoid double-print when the user explicitly requests help/version.
    let argv: Vec<String> = std::env::args().collect();
    let wants_help_or_version = argv
        .iter()
        .any(|a| matches!(a.as_str(), "-h" | "--help" | "-V" | "--version"));

    let args = if user_arg_count == 0 {
        interactive_launch_cli_args()?
    } else {
        if !wants_help_or_version {
            print_startup_help();
        }
        CliArgs::parse()
    };
    let result = run_headless(args).await;
    if let Err(ref e) = result {
        eprintln!(
            "{} {e}",
            "Error:".if_supports_color(Stream::Stderr, |t| t.style(Style::new().red().bold())),
        );
    }

    // Always wait for Enter, even on failures (e.g. PCILeech init errors).
    prompt_exit()?;
    result
}
