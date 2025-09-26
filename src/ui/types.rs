use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlotResizeDirection {
    None,
    WidthIncrease,
    WidthDecrease,
    HeightIncrease,
    HeightDecrease,
}

#[derive(Copy, Clone)]
pub enum PlotMetric {
    Throughput,
    Reads,
    Latency,
}

/// Type alias for data points (time, value)
pub type DataPoints = Vec<(f64, f64)>;

/// Type alias for metric data (throughput, reads, latency)
pub type MetricData = (DataPoints, DataPoints, DataPoints);

/// Type alias for size-based results (size, metrics)
pub type SizeResults = (usize, MetricData);

/// Type alias for test results storage
/// Format: (read_size, (throughput_points, reads_points, latency_points))
pub type TestResults = Arc<Mutex<Vec<SizeResults>>>;

pub struct ConfigParams<'a> {
    pub connector: &'a mut crate::speedtest::Connector,
    pub pcileech_device: &'a mut String,
    pub duration: &'a mut u64,
    pub ui_scale: &'a mut f32,
    pub ui_scale_text: &'a mut String,
    pub test_sizes: &'a mut [(usize, bool)],
    pub show_error_modal: &'a mut bool,
    pub error_modal_message: &'a mut String,
    pub show_config: &'a mut bool,
}

pub struct TestState<'a> {
    pub is_running: bool,
    pub current_throughput: f64,
    pub current_reads: u64,
    pub current_latency: f64,
    pub current_test_size: Option<usize>,
    pub test_start_time: Option<std::time::Instant>,
    pub test_end_time: Option<f64>,
    pub completed_chunks: &'a [(usize, f64)],
}

pub struct PlotControls<'a> {
    pub custom_plot_width: &'a mut f32,
    pub custom_plot_height: &'a mut f32,
    pub plot_resize_start_time: &'a mut Option<std::time::Instant>,
    pub plot_resize_direction: &'a mut PlotResizeDirection,
    pub plot_resize_last_repeat: &'a mut Option<std::time::Instant>,
}

pub struct StatsUpdateParams<'a> {
    pub current_throughput: &'a mut f64,
    pub current_reads: &'a mut u64,
    pub current_latency: &'a mut f64,
    pub current_test_size: &'a mut Option<usize>,
    pub test_start_time: &'a mut Option<std::time::Instant>,
    pub max_throughput: &'a mut f64,
    pub completed_chunks: &'a mut Vec<(usize, f64)>,
}

pub struct ResultsPanelParams<'a> {
    pub results: &'a TestResults,
    pub duration: u64,
    pub plot_controls: PlotControls<'a>,
    pub console: &'a crate::ui::console::ConsoleWindow,
    pub ui_scale: &'a mut f32,
    pub ui_scale_text: &'a mut String,
    pub test_state: TestState<'a>,
    pub test_sizes: &'a [(usize, bool)],
    pub show_config: &'a mut bool,
}
