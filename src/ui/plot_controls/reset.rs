use crate::ui::constants::{DEFAULT_PLOT_HEIGHT, DEFAULT_PLOT_WIDTH};
use crate::ui::types::PlotResizeDirection;
use eframe::egui;
use egui::Ui;

pub fn render_reset_row(
    ui: &mut Ui,
    custom_plot_width: &mut f32,
    custom_plot_height: &mut f32,
    plot_resize_direction: &mut PlotResizeDirection,
    plot_resize_start_time: &mut Option<std::time::Instant>,
    plot_resize_last_repeat: &mut Option<std::time::Instant>,
) {
    use egui::{Button, RichText};
    use egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE;

    ui.horizontal(|ui| {
        ui.label("Reset:");
        let btn_size = egui::vec2(22.0, ui.spacing().interact_size.y);
        if ui
            .add_sized(
                btn_size,
                Button::new(RichText::new(ARROW_COUNTER_CLOCKWISE)),
            )
            .clicked()
        {
            *custom_plot_width = DEFAULT_PLOT_WIDTH;
            *custom_plot_height = DEFAULT_PLOT_HEIGHT;
            *plot_resize_direction = PlotResizeDirection::None;
            *plot_resize_start_time = None;
            *plot_resize_last_repeat = None;
        }
        ui.label(format!(
            "{}×{}",
            DEFAULT_PLOT_WIDTH as u32, DEFAULT_PLOT_HEIGHT as u32
        ));
    });
}
