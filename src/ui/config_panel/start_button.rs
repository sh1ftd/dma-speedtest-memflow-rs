use eframe::egui;

pub fn render_start_button(
    ui: &mut egui::Ui,
    connector_requires_device: bool,
    has_selected_size: bool,
    on_start: impl FnOnce(),
    show_error_modal: &mut bool,
    error_modal_message: &mut String,
    show_config: &mut bool,
) {
    ui.vertical_centered(|ui| {
        let start_button =
            egui::Button::new(egui::RichText::new("START TEST").color(egui::Color32::BLACK))
                .fill(egui::Color32::from_rgb(46, 204, 113))
                .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(39, 174, 96)));

        if ui.add_sized([250.0, 55.0], start_button).clicked() {
            if connector_requires_device {
                *show_error_modal = true;
                *error_modal_message = "PCILeech device must be specified".to_string();
            } else if !has_selected_size {
                *show_error_modal = true;
                *error_modal_message = "At least one test size must be selected".to_string();
            } else {
                *show_config = false;
                on_start();
            }
        }
    });
}
