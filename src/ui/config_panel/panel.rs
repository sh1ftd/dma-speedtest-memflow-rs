use eframe::egui;
use egui_phosphor::regular::*;

use crate::speedtest::Connector;
use crate::ui::types::ConfigParams;

use super::{
    connector_section::render_connector_section, header::render_header,
    start_button::render_start_button, test_sizes::render_test_size_controls,
    ui_scale::render_ui_scale_controls,
};

pub fn render_config_panel(
    ui: &mut egui::Ui,
    params: &mut ConfigParams<'_>,
    on_start_test: impl FnOnce(),
) {
    render_header(ui);

    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(8.0)
        .inner_margin(15.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            render_ui_scale_controls(ui, params.ui_scale, params.ui_scale_text);
            ui.add_space(10.0);

            render_connector_section(ui, params.connector, params.pcileech_device);
            render_duration_slider(ui, params.duration);

            render_test_size_controls(params.test_sizes, ui);

            ui.add_space(15.0);
            let needs_pcileech_device = *params.connector == Connector::Pcileech
                && params.pcileech_device.trim().is_empty();
            let any_size_selected = params.test_sizes.iter().any(|(_, enabled)| *enabled);
            render_start_button(
                ui,
                needs_pcileech_device,
                any_size_selected,
                on_start_test,
                params.show_error_modal,
                params.error_modal_message,
                params.show_config,
            );
        });
}

fn render_duration_slider(ui: &mut egui::Ui, duration: &mut u64) {
    ui.add_space(8.0);
    ui.label(format!("{CLOCK} Test Duration"));
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(duration, 1..=60).text("seconds"));
    });
}
