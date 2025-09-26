use anyhow::Result;
use eframe::egui;

use crate::ui::config_panel::render_config_panel;
use crate::ui::modal::show_modal;
use crate::ui::results_panel::render_results_panel;
use crate::ui::test_management::handle_stats_update;
use crate::ui::types::{
    ConfigParams, PlotControls, ResultsPanelParams, StatsUpdateParams, TestState,
};

use super::state::SpeedTestApp;

impl SpeedTestApp {
    pub fn run(self) -> Result<()> {
        let options = eframe::NativeOptions {
            window_builder: Some(Box::new(|builder| {
                builder
                    .with_inner_size(egui::vec2(
                        super::super::constants::CONFIG_WINDOW_MIN_WIDTH,
                        super::super::constants::CONFIG_WINDOW_MIN_HEIGHT,
                    ))
                    .with_min_inner_size(egui::vec2(
                        super::super::constants::CONFIG_WINDOW_MIN_WIDTH,
                        super::super::constants::CONFIG_WINDOW_MIN_HEIGHT,
                    ))
                    .with_max_inner_size(egui::vec2(
                        super::super::constants::CONFIG_WINDOW_MAX_WIDTH,
                        super::super::constants::CONFIG_WINDOW_MAX_HEIGHT,
                    ))
                    .with_resizable(true)
                    .with_decorations(true)
                    .with_title("DMA Speed Test")
            })),
            ..Default::default()
        };

        eframe::run_native(
            "DMA Speed Test",
            options,
            Box::new(|cc| {
                let mut fonts = egui::FontDefinitions::default();
                egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
                cc.egui_ctx.set_fonts(fonts);

                Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(self))
            }),
        )
        .map_err(|e| anyhow::anyhow!("Failed to run GUI: {e}"))?;

        Ok(())
    }
}

impl eframe::App for SpeedTestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(self.ui_scale * 1.3);

        if self.is_running && !self.was_running {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                super::super::constants::TEST_WINDOW_MIN_WIDTH,
                super::super::constants::TEST_WINDOW_MIN_HEIGHT,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(egui::vec2(
                super::super::constants::TEST_WINDOW_MAX_WIDTH,
                super::super::constants::TEST_WINDOW_MAX_HEIGHT,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                super::super::constants::TEST_WINDOW_MIN_WIDTH,
                super::super::constants::TEST_WINDOW_MIN_HEIGHT,
            )));
        }
        self.was_running = self.is_running;

        self.console.show(ctx);

        if self.show_error_modal {
            let message = self.error_modal_message.clone();
            let mut on_close = || {
                self.show_error_modal = false;
            };
            show_modal(ctx, &message, &mut on_close);
            if !self.show_error_modal {
                self.error_modal_message.clear();
            }
            return;
        }

        if self.is_running
            && let Some(rx) = &mut self.stats_rx
        {
            let mut stats_params = StatsUpdateParams {
                current_throughput: &mut self.current_throughput,
                current_reads: &mut self.current_reads,
                current_latency: &mut self.current_latency,
                current_test_size: &mut self.current_test_size,
                test_start_time: &mut self.test_start_time,
                max_throughput: &mut self.max_throughput,
                completed_chunks: &mut self.completed_chunks,
            };

            let stats_closed =
                handle_stats_update(rx, &mut stats_params, &self.results, &self.console);

            if stats_closed {
                self.stop_test_impl();
            }
        }

        if let Some(rx) = &self.modal_rx
            && let Ok(error_msg) = rx.try_recv()
        {
            self.show_error_modal = true;
            self.error_modal_message = error_msg;
            self.stop_test_impl();
            self.show_config = true;
        }

        if self.show_config {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut should_start_test = false;
                let mut config_params = ConfigParams {
                    connector: &mut self.connector,
                    pcileech_device: &mut self.pcileech_device,
                    duration: &mut self.duration,
                    ui_scale: &mut self.ui_scale,
                    ui_scale_text: &mut self.ui_scale_text,
                    test_sizes: &mut self.test_sizes,
                    show_error_modal: &mut self.show_error_modal,
                    error_modal_message: &mut self.error_modal_message,
                    show_config: &mut self.show_config,
                };
                render_config_panel(ui, &mut config_params, || should_start_test = true);
                if should_start_test {
                    self.start_test_impl();
                }
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut should_stop_test = false;
                let mut should_start_test = false;
                let mut should_toggle_console = false;

                let plot_controls = PlotControls {
                    custom_plot_width: &mut self.custom_plot_width,
                    custom_plot_height: &mut self.custom_plot_height,
                    plot_resize_start_time: &mut self.plot_resize_start_time,
                    plot_resize_direction: &mut self.plot_resize_direction,
                    plot_resize_last_repeat: &mut self.plot_resize_last_repeat,
                };

                let test_state = TestState {
                    is_running: self.is_running,
                    current_throughput: self.current_throughput,
                    current_reads: self.current_reads,
                    current_latency: self.current_latency,
                    current_test_size: self.current_test_size,
                    test_start_time: self.test_start_time,
                    test_end_time: self.test_end_time,
                    completed_chunks: &self.completed_chunks,
                };

                let mut results_params = ResultsPanelParams {
                    results: &self.results,
                    duration: self.duration,
                    plot_controls,
                    console: &self.console,
                    ui_scale: &mut self.ui_scale,
                    ui_scale_text: &mut self.ui_scale_text,
                    test_state,
                    test_sizes: &self.test_sizes,
                    show_config: &mut self.show_config,
                };

                render_results_panel(
                    ui,
                    &mut results_params,
                    || should_stop_test = true,
                    || should_start_test = true,
                    &mut should_toggle_console,
                );

                if should_stop_test {
                    self.stop_test_impl();
                }
                if should_start_test {
                    self.start_test_impl();
                }
                if should_toggle_console {
                    self.console.toggle();
                }
            });
        }

        if self.is_running {
            ctx.request_repaint();
        }
    }
}
