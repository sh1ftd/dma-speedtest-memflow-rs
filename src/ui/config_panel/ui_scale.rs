use eframe::egui;
use egui_phosphor::regular::*;

pub fn render_ui_scale_controls(ui: &mut egui::Ui, ui_scale: &mut f32, ui_scale_text: &mut String) {
    ui.horizontal(|ui| {
        ui.label(format!("{MAGNIFYING_GLASS} UI Scale"));
        let button_size = egui::vec2(22.0, ui.spacing().interact_size.y);

        if ui.add_sized(button_size, egui::Button::new("-")).clicked() {
            *ui_scale = (*ui_scale - 0.1).max(0.3);
        }

        if ui_scale_text.parse::<f32>().unwrap_or(1.0) != *ui_scale {
            *ui_scale_text = format!("{ui_scale:.1}");
        }

        let text_width = 70.0;
        if ui
            .add_sized(
                [text_width, ui.spacing().interact_size.y],
                egui::TextEdit::singleline(ui_scale_text),
            )
            .changed()
            && let Ok(parsed_scale) = ui_scale_text.parse::<f32>()
        {
            *ui_scale = parsed_scale.clamp(0.3, 3.0);
        }

        if ui.add_sized(button_size, egui::Button::new("+")).clicked() {
            *ui_scale = (*ui_scale + 0.1).min(3.0);
        }
    });
}
