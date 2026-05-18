use crate::speedtest::BenchOp;
use crate::ui::helpers::get_size_label;
use crate::ui::types::ResultsPanelParams;
use eframe::egui;
use egui_phosphor::regular::*;

pub fn render_running_metrics(ui: &mut egui::Ui, params: &ResultsPanelParams<'_>) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} {:.1} MiB/s",
                    LIGHTNING, params.test_state.current_throughput
                ))
                .size(16.0)
                .color(egui::Color32::from_rgb(52, 152, 219)),
            );
            ui.label("•");
            let ops_label = params
                .test_state
                .current_bench_op
                .map(|o| o.ops_per_sec_label())
                .unwrap_or("ops/s");
            ui.label(
                egui::RichText::new(format!(
                    "{} {} {}",
                    CHART_BAR, params.test_state.current_ops_per_sec, ops_label
                ))
                .size(14.0)
                .color(egui::Color32::from_rgb(155, 89, 182)),
            );
            ui.label("•");
            ui.label(
                egui::RichText::new(format!(
                    "{} {:.1} μs",
                    CLOCK, params.test_state.current_latency
                ))
                .size(14.0)
                .color(egui::Color32::from_rgb(231, 76, 60)),
            );

            if let Some(current_size) = params.test_state.current_test_size {
                ui.label("•");
                let op_prefix = params
                    .test_state
                    .current_bench_op
                    .map(|o| format!("{} ", o.label()))
                    .unwrap_or_default();
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}{}",
                        TARGET,
                        op_prefix,
                        get_size_label(current_size)
                    ))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(155, 89, 182)),
                );
            }
        });

        if let (Some(BenchOp::Write), Some(targets), Some(chunk)) = (
            params.test_state.current_bench_op,
            params.test_state.probe_targets,
            params.test_state.current_test_size,
        ) && let Some(line) = targets.format_write_live(chunk)
        {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("{PENCIL_SIMPLE} {line}"))
                    .size(13.0)
                    .color(egui::Color32::from_rgb(241, 196, 15)),
            );
        }
    });
}
