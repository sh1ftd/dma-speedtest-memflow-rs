use crate::speedtest::BenchMode;
use eframe::egui;
use egui_phosphor::regular::*;

pub fn render_bench_mode_controls(ui: &mut egui::Ui, bench_mode: &mut BenchMode) {
    ui.add_space(8.0);
    ui.label(format!("{ARROWS_LEFT_RIGHT} Benchmark mode"));
    ui.horizontal(|ui| {
        ui.radio_value(bench_mode, BenchMode::Read, "Read");
        ui.radio_value(bench_mode, BenchMode::Write, "Write");
        ui.radio_value(bench_mode, BenchMode::Both, "Both");
    });
    ui.label(
        egui::RichText::new(
            "Write/both mutates target process memory at an auto-selected writable probe region; restore is best-effort.",
        )
        .small()
        .weak(),
    );
}
