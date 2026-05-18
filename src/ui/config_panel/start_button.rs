use eframe::egui;

pub struct StartButtonParams<'a> {
    pub can_start: bool,
    pub is_connecting: bool,
    pub connector_requires_device: bool,
    pub has_selected_size: bool,
    pub show_config: &'a mut bool,
}

pub fn render_start_button(
    ui: &mut egui::Ui,
    params: &mut StartButtonParams<'_>,
    on_start: impl FnOnce(),
) {
    ui.vertical_centered(|ui| {
        let label = if params.is_connecting {
            "CONNECTING..."
        } else if !params.can_start {
            "PLEASE WAIT..."
        } else {
            "START TEST"
        };
        let start_button =
            egui::Button::new(egui::RichText::new(label).color(egui::Color32::BLACK))
                .fill(egui::Color32::from_rgb(46, 204, 113))
                .stroke(egui::Stroke::new(
                    2.0_f32,
                    egui::Color32::from_rgb(39, 174, 96),
                ));
        let enabled = params.can_start
            && !params.is_connecting
            && !params.connector_requires_device
            && params.has_selected_size;

        let clicked = if enabled {
            ui.add_sized([250.0, 55.0], start_button).clicked()
        } else {
            ui.add_enabled(false, egui::Label::new(label));
            false
        };

        if clicked {
            *params.show_config = false;
            on_start();
        }
    });
}
