use eframe::egui;
use egui_phosphor::regular::*;

use crate::ui::helpers::{color_for_size, get_size_label};

pub fn render_test_size_controls(test_sizes: &mut [(usize, bool)], ui: &mut egui::Ui) {
    ui.label(format!("{CHART_BAR} Test Sizes"));
    ui.label("Select which read sizes to test:");

    render_bulk_actions(test_sizes, ui);
    ui.add_space(8.0);
    render_size_grid(test_sizes, ui);
}

fn render_bulk_actions(test_sizes: &mut [(usize, bool)], ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("Select All").clicked() {
            for (_, enabled) in test_sizes.iter_mut() {
                *enabled = true;
            }
        }

        ui.add_space(8.0);

        if ui.button("Deselect All").clicked() {
            for (_, enabled) in test_sizes.iter_mut() {
                *enabled = false;
            }
        }

        ui.add_space(8.0);

        if ui.button("Default").clicked() {
            for (size, enabled) in test_sizes.iter_mut() {
                *enabled = matches!(*size, 4096 | 8192 | 16384 | 32768);
            }
        }
    });
}

fn render_size_grid(test_sizes: &mut [(usize, bool)], ui: &mut egui::Ui) {
    egui::Grid::new("test_sizes_grid")
        .num_columns(3)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            for (index, (size, enabled)) in test_sizes.iter_mut().enumerate() {
                let label = get_size_label(*size);
                let color = color_for_size(*size);

                let is_selected = *enabled;
                let checkmark = if is_selected { CHECK } else { "" };
                let text = egui::RichText::new(format!("{checkmark} {label}"))
                    .color(color)
                    .strong();

                let stroke = if is_selected {
                    egui::Stroke::new(2.0, color)
                } else {
                    ui.visuals().widgets.inactive.bg_stroke
                };

                let fill = if is_selected {
                    ui.visuals().faint_bg_color
                } else {
                    ui.visuals().extreme_bg_color
                };

                let button = egui::Button::new(text)
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(6.0);

                if ui
                    .add_sized([140.0, ui.spacing().interact_size.y * 1.2], button)
                    .clicked()
                {
                    *enabled = !*enabled;
                }

                if (index + 1).is_multiple_of(3) {
                    ui.end_row();
                }
            }

            if !test_sizes.len().is_multiple_of(3) {
                ui.end_row();
            }
        });
}
