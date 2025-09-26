use crate::ui::types::ResultsPanelParams;
use eframe::egui;
use egui_phosphor::regular::*;

pub fn render_console_and_scale_controls(
    ui: &mut egui::Ui,
    params: &mut ResultsPanelParams<'_>,
    on_toggle_console: &mut bool,
) {
    ui.horizontal(|ui| {
        let console_text = if params.console.is_visible() {
            format!("{EYE_SLASH} Hide Console")
        } else {
            format!("{EYE} Show Console")
        };
        if ui
            .add_sized([140.0, 35.0], egui::Button::new(console_text))
            .clicked()
        {
            *on_toggle_console = true;
        }

        ui.separator();

        ui.label(format!("{MAGNIFYING_GLASS} UI Scale"));
        let btn_size = egui::vec2(22.0, ui.spacing().interact_size.y);

        if ui.add_sized(btn_size, egui::Button::new("-")).clicked() {
            *params.ui_scale = (*params.ui_scale - 0.1).max(0.3);
        }

        if params.ui_scale_text.parse::<f32>().unwrap_or(1.0) != *params.ui_scale {
            *params.ui_scale_text = format!("{:.1}", params.ui_scale);
        }
        let width = 70.0;
        if ui
            .add_sized(
                [width, ui.spacing().interact_size.y],
                egui::TextEdit::singleline(params.ui_scale_text),
            )
            .changed()
            && let Ok(new_scale) = params.ui_scale_text.parse::<f32>()
        {
            *params.ui_scale = new_scale.clamp(0.3, 3.0);
        }

        if ui.add_sized(btn_size, egui::Button::new("+")).clicked() {
            *params.ui_scale = (*params.ui_scale + 0.1).min(3.0);
        }
    });
}
