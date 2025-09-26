use eframe::egui;
use std::time::Instant;

use crate::ui::constants::{PLOT_RESIZE_INITIAL_DELAY, PLOT_RESIZE_REPEAT_RATE};
use crate::ui::types::PlotResizeDirection;

#[allow(clippy::too_many_arguments)]
pub fn handle_resize_button(
    ui: &egui::Ui,
    response: &egui::Response,
    value: &mut f32,
    delta: f32,
    min: f32,
    max: f32,
    direction: PlotResizeDirection,
    plot_resize_direction: &mut PlotResizeDirection,
    plot_resize_start_time: &mut Option<Instant>,
    plot_resize_last_repeat: &mut Option<Instant>,
) {
    if response.is_pointer_button_down_on() {
        if *plot_resize_direction != direction {
            *value = (*value + delta).clamp(min, max);
            *plot_resize_start_time = Some(Instant::now());
            *plot_resize_direction = direction;
            *plot_resize_last_repeat = None;
        } else if let Some(start_time) = plot_resize_start_time {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed > PLOT_RESIZE_INITIAL_DELAY {
                let should_repeat = match plot_resize_last_repeat {
                    Some(last_repeat) => {
                        last_repeat.elapsed().as_secs_f32() >= PLOT_RESIZE_REPEAT_RATE
                    }
                    None => true,
                };

                if should_repeat {
                    *value = (*value + delta).clamp(min, max);
                    *plot_resize_last_repeat = Some(Instant::now());
                }
            }
        }

        ui.ctx().request_repaint();
    } else if *plot_resize_direction == direction {
        *plot_resize_direction = PlotResizeDirection::None;
        *plot_resize_start_time = None;
        *plot_resize_last_repeat = None;
    }
}
