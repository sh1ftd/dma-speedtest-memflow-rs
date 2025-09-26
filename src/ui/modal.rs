use eframe::egui;
use egui_phosphor::regular::*;

const MODAL_WIDTH: f32 = 460.0;
const MODAL_HEIGHT: f32 = 180.0;
const ACCENT_COLOR: egui::Color32 = egui::Color32::from_rgb(231, 76, 60);

pub fn show_modal(ctx: &egui::Context, message: &str, on_close: &mut dyn FnMut()) {
    egui::Area::new(egui::Id::new("modal"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(24.0, 24.0))
        .show(ctx, |ui| {
            let outer_size = egui::vec2(MODAL_WIDTH, MODAL_HEIGHT);
            let (rect, _) = ui.allocate_exact_size(outer_size, egui::Sense::hover());

            #[allow(deprecated)]
            let mut child = ui.child_ui(rect, egui::Layout::top_down(egui::Align::Min), None);

            egui::Frame::new()
                .fill(child.visuals().extreme_bg_color)
                .stroke(egui::Stroke::new(1.5, ACCENT_COLOR))
                .corner_radius(10.0)
                .inner_margin(8.0)
                .show(&mut child, |ui| {
                    render_header(ui, rect.width());
                    ui.add_space(6.0);

                    render_separator(ui, rect.width());
                    ui.add_space(6.0);

                    render_message(ui, message, rect.width());
                    ui.add_space(8.0);

                    render_action_row(ui, on_close);
                });
        });
}

fn render_header(ui: &mut egui::Ui, width: f32) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(WARNING).color(ACCENT_COLOR).size(18.0));
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Error")
                .color(ACCENT_COLOR)
                .size(16.0)
                .strong(),
        );
    });
    ui.set_max_width(width - 16.0);
}

fn render_separator(ui: &mut egui::Ui, width: f32) {
    let separator_color = ACCENT_COLOR.linear_multiply(0.3);
    let top_left = ui.cursor().min;
    let bottom_right = top_left + egui::vec2(width - 16.0, 1.0);
    ui.painter().rect_filled(
        egui::Rect::from_min_max(top_left, bottom_right),
        0.0,
        separator_color,
    );
}

fn render_message(ui: &mut egui::Ui, message: &str, width: f32) {
    let padding = 6.0;
    ui.set_max_width((width - padding * 2.0 - 16.0).max(0.0));
    ui.label(egui::RichText::new(message).size(14.0));
}

fn render_action_row(ui: &mut egui::Ui, on_close: &mut dyn FnMut()) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let ok_button = egui::Button::new(
            egui::RichText::new("OK")
                .color(egui::Color32::WHITE)
                .size(16.0)
                .strong(),
        )
        .fill(ACCENT_COLOR)
        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(192, 57, 43)))
        .corner_radius(8.0)
        .min_size(egui::vec2(96.0, 36.0));

        if ui.add(ok_button).clicked() {
            on_close();
        }
    });
}
