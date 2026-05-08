use anyhow::Result;
use clap::Parser;
use dma_speedtest_memflow_rs::cli::{
    CliArgs, ensure_stdio_for_headless, print_startup_help, prompt_exit, run_headless,
};

#[tokio::main]
async fn main() -> Result<()> {
    ensure_stdio_for_headless();

    // Print help on startup so available flags are visible by default.
    // Avoid double-print when the user explicitly requests help/version.
    let argv: Vec<String> = std::env::args().collect();
    let wants_help_or_version = argv
        .iter()
        .any(|a| matches!(a.as_str(), "-h" | "--help" | "-V" | "--version"));
    if !wants_help_or_version {
        print_startup_help();
    }

    let args = CliArgs::parse();
    let result = run_headless(args).await;
    if let Err(ref e) = result {
        eprintln!("Error: {e}");
    }

    // Always wait for Enter, even on failures (e.g. PCILeech init errors).
    prompt_exit()?;
    result
}
