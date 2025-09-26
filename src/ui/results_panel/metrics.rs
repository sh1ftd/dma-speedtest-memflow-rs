use crate::ui::helpers::get_size_label;
use crate::ui::types::ResultsPanelParams;
use eframe::egui;
use egui_phosphor::regular::*;

pub fn render_running_metrics(ui: &mut egui::Ui, params: &ResultsPanelParams<'_>) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{} {:.1} MB/s",
                LIGHTNING, params.test_state.current_throughput
            ))
            .size(16.0)
            .color(egui::Color32::from_rgb(52, 152, 219)),
        );
        ui.label("•");
        ui.label(
            egui::RichText::new(format!(
                "{} {} reads/s",
                CHART_BAR, params.test_state.current_reads
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
            ui.label(
                egui::RichText::new(format!("{} {}", TARGET, get_size_label(current_size)))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(155, 89, 182)),
            );
        }
    });
}
