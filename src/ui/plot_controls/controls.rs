use crate::ui::types::PlotResizeDirection;
use eframe::egui;
use std::time::Instant;

use super::dimension::render_dimension_control;
use super::reset::render_reset_row;

pub fn render_plot_size_controls(
    ui: &mut egui::Ui,
    custom_plot_width: &mut f32,
    custom_plot_height: &mut f32,
    plot_resize_start_time: &mut Option<Instant>,
    plot_resize_direction: &mut PlotResizeDirection,
    plot_resize_last_repeat: &mut Option<Instant>,
) {
    ui.vertical(|ui| {
        ui.label(format!("{} Plot Size", egui_phosphor::regular::CHART_BAR));
        ui.add_space(4.0);

        render_dimension_control(
            ui,
            "Width:",
            custom_plot_width,
            PlotResizeDirection::WidthDecrease,
            PlotResizeDirection::WidthIncrease,
            plot_resize_start_time,
            plot_resize_direction,
            plot_resize_last_repeat,
        );

        ui.add_space(4.0);

        render_dimension_control(
            ui,
            "Height:",
            custom_plot_height,
            PlotResizeDirection::HeightDecrease,
            PlotResizeDirection::HeightIncrease,
            plot_resize_start_time,
            plot_resize_direction,
            plot_resize_last_repeat,
        );

        ui.add_space(4.0);
        render_reset_row(
            ui,
            custom_plot_width,
            custom_plot_height,
            plot_resize_direction,
            plot_resize_start_time,
            plot_resize_last_repeat,
        );
    });
}
