use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::mpsc;

use crate::speedtest::{Connector, SpeedTest};
use crate::ui::console::ConsoleWindow;

use super::super::constants::DEFAULT_PLOT_HEIGHT;
use super::super::constants::DEFAULT_PLOT_WIDTH;
use super::super::types::PlotResizeDirection;
use super::super::types::TestResults;

pub struct SpeedTestApp {
    pub connector: Connector,
    pub pcileech_device: String,
    pub duration: u64,
    pub test: Option<SpeedTest>,
    pub results: TestResults,
    pub is_running: bool,
    pub was_running: bool,
    pub error_message: Option<String>,
    pub current_throughput: f64,
    pub current_reads: u64,
    pub current_latency: f64,
    pub show_config: bool,
    #[allow(clippy::type_complexity)]
    pub stats_rx: Option<mpsc::Receiver<(f64, u64, f64, usize, f64)>>, // (throughput, reads_per_sec, elapsed_secs, read_size, latency_us)
    pub test_start_time: Option<Instant>,
    pub overall_test_start_time: Option<Instant>,
    pub test_end_time: Option<f64>,
    pub current_test_size: Option<usize>,
    pub completed_chunks: Vec<(usize, f64)>,
    pub max_throughput: f64,
    pub console: ConsoleWindow,
    pub ui_scale: f32,
    pub ui_scale_text: String,
    pub test_sizes: Vec<(usize, bool)>,
    pub show_error_modal: bool,
    pub error_modal_message: String,
    pub modal_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub custom_plot_width: f32,
    pub custom_plot_height: f32,
    pub plot_resize_start_time: Option<std::time::Instant>,
    pub plot_resize_direction: PlotResizeDirection,
    pub plot_resize_last_repeat: Option<std::time::Instant>,
}

impl Default for SpeedTestApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeedTestApp {
    pub fn new() -> Self {
        Self {
            connector: Connector::default(),
            pcileech_device: String::new(),
            duration: 10,
            test: None,
            results: Arc::new(Mutex::new(Vec::new())), // (read_size, (throughput_points, reads_points, latency_points))
            is_running: false,
            was_running: false,
            error_message: None,
            current_throughput: 0.0,
            current_reads: 0,
            current_latency: 0.0,
            show_config: true,
            stats_rx: None,
            test_start_time: None,
            overall_test_start_time: None,
            test_end_time: None,
            current_test_size: None,
            completed_chunks: Vec::new(),
            max_throughput: 0.0,
            console: ConsoleWindow::new(),
            ui_scale: 1.0,
            ui_scale_text: "1.0".to_string(),
            show_error_modal: false,
            error_modal_message: String::new(),
            modal_rx: None,
            custom_plot_width: DEFAULT_PLOT_WIDTH,
            custom_plot_height: DEFAULT_PLOT_HEIGHT,
            plot_resize_start_time: None,
            plot_resize_direction: PlotResizeDirection::None,
            plot_resize_last_repeat: None,
            test_sizes: vec![
                (512, false),    // 512 bytes
                (1024, false),   // 1KB
                (2048, false),   // 2KB
                (4096, true),    // 4KB - selected by default
                (8192, true),    // 8KB - selected by default
                (16384, true),   // 16KB - selected by default
                (32768, true),   // 32KB - selected by default
                (65536, false),  // 64KB
                (131072, false), // 128KB
            ],
        }
    }
}
