// Prevents additional console window on Windows in release mode.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "branding")]
mod branding;
mod speedtest;
mod ui;

use anyhow::Result;
use ui::app::SpeedTestApp;

#[tokio::main]
async fn main() -> Result<()> {
    let app = SpeedTestApp::new();
    app.run()?;
    Ok(())
}
