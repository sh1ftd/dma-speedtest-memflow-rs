use crate::speedtest::Connector;
use eframe::egui;
use egui_phosphor::regular::*;

pub fn render_connector_section(
    ui: &mut egui::Ui,
    connector: &mut Connector,
    pcileech_device: &mut String,
) {
    ui.label(format!("{PLUG} Connector Type"));
    ui.horizontal(|ui| {
        render_connector_button(
            ui,
            connector,
            Connector::Pcileech,
            format!("{GRAPHICS_CARD} PCILeech"),
        );

        ui.add_space(12.0);

        render_connector_button(ui, connector, Connector::Native, format!("{MEMORY} Native"));
    });

    if matches!(connector, Connector::Pcileech) {
        render_pcileech_device(ui, pcileech_device);
    }
}

fn render_connector_button(
    ui: &mut egui::Ui,
    connector: &mut Connector,
    variant: Connector,
    label: String,
) {
    let is_selected = *connector == variant;
    let display = if is_selected {
        format!("{label} {CHECK}")
    } else {
        label
    };

    let button = egui::Button::new(egui::RichText::new(display).color(egui::Color32::BLACK))
        .fill(if is_selected {
            egui::Color32::from_rgb(46, 204, 113)
        } else {
            egui::Color32::from_rgb(52, 152, 219)
        })
        .stroke(if is_selected {
            egui::Stroke::new(2.0, egui::Color32::from_rgb(39, 174, 96))
        } else {
            egui::Stroke::new(1.0, egui::Color32::from_rgb(41, 128, 185))
        });

    if ui.add_sized([120.0, 35.0], button).clicked() {
        *connector = variant;
    }
}

fn render_pcileech_device(ui: &mut egui::Ui, pcileech_device: &mut String) {
    if pcileech_device.is_empty() {
        pcileech_device.push_str("FPGA");
    }

    ui.add_space(8.0);
    ui.label(format!("{DESKTOP} PCILeech Device"));
    ui.horizontal(|ui| {
        let height = ui.spacing().interact_size.y;
        let width = 220.0;
        ui.add_sized([width, height], egui::TextEdit::singleline(pcileech_device));
    });
}
