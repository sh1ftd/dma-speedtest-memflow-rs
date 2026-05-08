#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use anyhow::Result;
use dma_speedtest_memflow_rs::ui::app::SpeedTestApp;

fn main() -> Result<()> {
    SpeedTestApp::run()
}
