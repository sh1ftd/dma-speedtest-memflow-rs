use anyhow::Result;
use clap::Parser;
use dma_speedtest_memflow_rs::{Cli, SpeedTest};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let test = SpeedTest::new(args.connector, args.pcileech_device)?;
    let duration = Duration::from_secs(args.duration);

    test.run_speed_test(duration).await?;

    Ok(())
}
