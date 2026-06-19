use super::{
    controls::render_console_and_scale_controls, metrics::render_running_metrics,
    plot::render_plot_column, progress::render_chunk_progress, table::render_results_table,
};
use crate::ui::plot_controls::render_plot_size_controls;
use crate::ui::types::{PlotMetric, ResultsPanelParams};
use eframe::egui;
use egui_phosphor::regular::*;

#[cfg(feature = "branding")]
use crate::branding;

pub fn render_results_panel(
    ui: &mut egui::Ui,
    params: &mut ResultsPanelParams<'_>,
    on_stop_test: impl FnOnce(),
    on_test_again: impl FnOnce(),
    on_toggle_console: &mut bool,
    on_export_csv: &mut bool,
    on_export_json: &mut bool,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(5.0);
        ui.heading(format!("{CHART_LINE_UP} DMA Speed Test Results"));
        ui.add_space(10.0);
    });

    let fill_color = {
        #[cfg(feature = "branding")]
        {
            let (r, g, b) = branding::BACKGROUND_COLOR;
            let alpha = (branding::UI_PANEL_OPACITY * 255.0) as u8;
            egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
        }
        #[cfg(not(feature = "branding"))]
        {
            ui.visuals().extreme_bg_color
        }
    };

    egui::Frame::new()
        .fill(fill_color)
        .corner_radius(6.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    render_plot_size_controls(
                        ui,
                        params.plot_controls.custom_plot_width,
                        params.plot_controls.custom_plot_height,
                        params.plot_controls.plot_resize_start_time,
                        params.plot_controls.plot_resize_direction,
                        params.plot_controls.plot_resize_last_repeat,
                    );

                    ui.add_space(8.0);
                    render_console_and_scale_controls(ui, params, on_toggle_console);

                    if params.test_state.is_connecting {
                        ui.separator();
                        ui.label("Connecting to device...");
                    } else if params.test_state.is_running {
                        ui.separator();
                        render_running_metrics(ui, params);
                        render_chunk_progress(ui, params);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if params.test_state.is_running || params.test_state.is_connecting {
                        let stop_label = if params.test_state.is_connecting {
                            format!("{X_CIRCLE} CANCEL")
                        } else {
                            format!("{X_CIRCLE} STOP TEST")
                        };
                        let stop_button = egui::Button::new(
                            egui::RichText::new(stop_label).color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::from_rgb(231, 76, 60))
                        .stroke(egui::Stroke::new(
                            2.0_f32,
                            egui::Color32::from_rgb(192, 57, 43),
                        ));
                        if ui.add_sized([150.0, 40.0], stop_button).clicked() {
                            on_stop_test();
                        }
                    } else if params.test_state.test_end_time.is_some() {
                        let again_label = if params.test_state.can_restart {
                            format!("{LIGHTNING} TEST AGAIN")
                        } else {
                            "PLEASE WAIT...".to_string()
                        };
                        let again_button = egui::Button::new(
                            egui::RichText::new(again_label.clone()).color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::from_rgb(46, 204, 113))
                        .stroke(egui::Stroke::new(
                            2.0_f32,
                            egui::Color32::from_rgb(39, 174, 96),
                        ));
                        let again_clicked = if params.test_state.can_restart {
                            ui.add_sized([170.0, 40.0], again_button).clicked()
                        } else {
                            ui.add_enabled(false, egui::Label::new(&again_label));
                            false
                        };
                        if again_clicked {
                            on_test_again();
                            *params.show_config = false;
                        }

                        ui.add_space(8.0);

                        let back_button = egui::Button::new(
                            egui::RichText::new("RETURN TO CONFIG").color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::from_rgb(241, 196, 15))
                        .stroke(egui::Stroke::new(
                            2.0_f32,
                            egui::Color32::from_rgb(243, 156, 18),
                        ));
                        if ui.add_sized([190.0, 40.0], back_button).clicked() {
                            *params.show_config = true;
                        }

                        ui.add_space(8.0);

                        let csv_button = egui::Button::new(
                            egui::RichText::new(format!("{FLOPPY_DISK} CSV"))
                                .color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::from_rgb(52, 152, 219))
                        .stroke(egui::Stroke::new(
                            2.0_f32,
                            egui::Color32::from_rgb(41, 128, 185),
                        ));
                        if ui.add_sized([100.0, 40.0], csv_button).clicked() {
                            *on_export_csv = true;
                        }

                        ui.add_space(8.0);

                        let json_button = egui::Button::new(
                            egui::RichText::new(format!("{FLOPPY_DISK} JSON"))
                                .color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::from_rgb(155, 89, 182))
                        .stroke(egui::Stroke::new(
                            2.0_f32,
                            egui::Color32::from_rgb(142, 68, 173),
                        ));
                        if ui.add_sized([110.0, 40.0], json_button).clicked() {
                            *on_export_json = true;
                        }
                    }
                });
            });

            render_report_export_status(ui, params);
        });

    ui.add_space(10.0);

    ui.columns(3, |columns| {
        let width = *params.plot_controls.custom_plot_width;
        let height = *params.plot_controls.custom_plot_height;

        let specs = [
            (
                "Throughput (MiB/s)",
                "throughput_plot",
                PlotMetric::Throughput,
                "throughput_results",
                "Results (MiB/s)",
            ),
            (
                "Ops per Second",
                "reads_plot",
                PlotMetric::Reads,
                "reads_results",
                "Results (ops/s)",
            ),
            (
                "Latency (μs)",
                "latency_plot",
                PlotMetric::Latency,
                "latency_results",
                "Results (μs)",
            ),
        ];

        for (column, (heading, plot_id, metric, table_id, title)) in columns.iter_mut().zip(specs) {
            column.vertical(|ui| {
                render_plot_column(ui, heading, plot_id, metric, params, width, height);
                ui.add_space(10.0);
                render_results_table(ui, table_id, params.results, metric, title);
            });
        }
    });
}

fn render_report_export_status(ui: &mut egui::Ui, params: &ResultsPanelParams<'_>) {
    let Some(status) = params.test_state.report_export_status else {
        return;
    };
    if params.test_state.test_end_time.is_none() {
        return;
    }

    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        let (icon, color) = if status.is_error {
            (WARNING, egui::Color32::from_rgb(231, 76, 60))
        } else {
            (CHECK, egui::Color32::from_rgb(46, 204, 113))
        };
        ui.label(
            egui::RichText::new(format!("{icon} {}", status.message))
                .color(color)
                .strong(),
        );
    });
}
