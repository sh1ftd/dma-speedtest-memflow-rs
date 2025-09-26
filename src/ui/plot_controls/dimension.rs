use crate::ui::constants::{PLOT_MAX_SIZE, PLOT_MIN_SIZE, PLOT_RESIZEE_INCREMENT};
use crate::ui::types::PlotResizeDirection;
use eframe::egui;
use std::time::Instant;

use super::handlers::handle_resize_button;

#[allow(clippy::too_many_arguments)]
pub fn render_dimension_control(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    decrement_direction: PlotResizeDirection,
    increment_direction: PlotResizeDirection,
    plot_resize_start_time: &mut Option<Instant>,
    plot_resize_direction: &mut PlotResizeDirection,
    plot_resize_last_repeat: &mut Option<Instant>,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let btn_size = egui::vec2(22.0, ui.spacing().interact_size.y);

        let minus_response = ui.add_sized(btn_size, egui::Button::new("-"));
        handle_resize_button(
            ui,
            &minus_response,
            value,
            -PLOT_RESIZEE_INCREMENT,
            PLOT_MIN_SIZE,
            PLOT_MAX_SIZE,
            decrement_direction,
            plot_resize_direction,
            plot_resize_start_time,
            plot_resize_last_repeat,
        );

        ui.label(format!("{value:.0}px"));

        let plus_response = ui.add_sized(btn_size, egui::Button::new("+"));
        handle_resize_button(
            ui,
            &plus_response,
            value,
            PLOT_RESIZEE_INCREMENT,
            PLOT_MIN_SIZE,
            PLOT_MAX_SIZE,
            increment_direction,
            plot_resize_direction,
            plot_resize_start_time,
            plot_resize_last_repeat,
        );
    });
}
