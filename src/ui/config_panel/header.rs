use eframe::egui;
use egui_phosphor::regular::*;

pub fn render_header(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(10.0);
        ui.heading(format!("{LIGHTNING} DMA Speed Test"));
        ui.add_space(5.0);
        ui.label("Configure your test parameters below");
        ui.add_space(15.0);
    });
}
