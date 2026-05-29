use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::mpsc;

use crate::speedtest::{BenchMode, BenchStats, Connector, ProbeTargets, SpeedTest};
use crate::ui::console::ConsoleWindow;

use super::super::constants::DEFAULT_PLOT_HEIGHT;
use super::super::constants::DEFAULT_PLOT_WIDTH;
use super::super::types::PlotResizeDirection;
use super::super::types::TestResults;

#[cfg(feature = "branding")]
use crate::branding::BrandingManager;

pub struct SpeedTestApp {
    pub connector: Connector,
    pub pcileech_device: String,
    pub duration: u64,
    pub bench_mode: BenchMode,
    pub test: Option<SpeedTest>,
    pub probe_targets: Option<ProbeTargets>,
    pub results: TestResults,
    pub is_running: bool,
    pub is_connecting: bool,
    pub connect_rx: Option<std::sync::mpsc::Receiver<Result<SpeedTest, String>>>,
    pub connection_cancelled: bool,
    pub was_running: bool,
    pub error_message: Option<String>,
    pub current_throughput: f64,
    pub current_ops_per_sec: u64,
    pub current_bench_op: Option<crate::speedtest::BenchOp>,
    pub current_latency: f64,
    pub show_config: bool,
    pub was_show_config: bool,
    #[allow(clippy::type_complexity)]
    pub stats_rx: Option<mpsc::Receiver<BenchStats>>,
    pub test_start_time: Option<Instant>,
    pub overall_test_start_time: Option<Instant>,
    pub test_end_time: Option<f64>,
    pub current_test_size: Option<usize>,
    pub completed_chunks: Vec<(crate::speedtest::BenchOp, usize, f64)>,
    pub max_throughput: f64,
    pub console: ConsoleWindow,
    pub ui_scale: f32,
    pub ui_scale_text: String,
    pub test_sizes: Vec<(usize, bool)>,
    pub show_error_modal: bool,
    pub was_error_modal: bool,
    pub error_modal_message: String,
    pub modal_rx: Option<std::sync::mpsc::Receiver<String>>,
    /// Set while the benchmark thread is alive; cleared when it exits.
    pub bench_done_rx: Option<std::sync::mpsc::Receiver<()>>,
    pub custom_plot_width: f32,
    pub custom_plot_height: f32,
    pub plot_resize_start_time: Option<std::time::Instant>,
    pub plot_resize_direction: PlotResizeDirection,
    pub plot_resize_last_repeat: Option<std::time::Instant>,
    pub center_window_frames: u8,
    pub last_console_stats_log: Option<std::time::Instant>,
    #[cfg(feature = "branding")]
    pub branding_manager: BrandingManager,
}

impl Default for SpeedTestApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeedTestApp {
    pub fn bench_thread_active(&self) -> bool {
        self.bench_done_rx.is_some()
    }

    pub fn connection_thread_active(&self) -> bool {
        self.connect_rx.is_some()
    }

    pub fn can_start_test(&self) -> bool {
        !self.is_connecting && !self.connection_thread_active() && !self.bench_thread_active()
    }

    pub fn new() -> Self {
        Self {
            connector: Connector::default(),
            pcileech_device: String::new(),
            duration: 10,
            bench_mode: BenchMode::Read,
            test: None,
            probe_targets: None,
            results: Arc::new(Mutex::new(Vec::new())),
            is_running: false,
            is_connecting: false,
            connect_rx: None,
            connection_cancelled: false,
            was_running: false,
            error_message: None,
            current_throughput: 0.0,
            current_ops_per_sec: 0,
            current_bench_op: None,
            current_latency: 0.0,
            show_config: true,
            was_show_config: true,
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
            was_error_modal: false,
            error_modal_message: String::new(),
            modal_rx: None,
            bench_done_rx: None,
            custom_plot_width: DEFAULT_PLOT_WIDTH,
            custom_plot_height: DEFAULT_PLOT_HEIGHT,
            plot_resize_start_time: None,
            plot_resize_direction: PlotResizeDirection::None,
            plot_resize_last_repeat: None,
            center_window_frames: 0,
            last_console_stats_log: None,
            test_sizes: crate::bench_config::default_gui_chunk_sizes(),
            #[cfg(feature = "branding")]
            branding_manager: BrandingManager::new(),
        }
    }
}
