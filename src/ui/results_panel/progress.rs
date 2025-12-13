use crate::ui::helpers::get_size_label;
use crate::ui::types::ResultsPanelParams;
use eframe::egui;
use egui_phosphor::regular::*;

#[cfg(feature = "branding")]
use crate::branding;

pub fn render_chunk_progress(ui: &mut egui::Ui, params: &ResultsPanelParams<'_>) {
    if !params.test_state.is_running && params.test_state.completed_chunks.is_empty() {
        return;
    }

    ui.add_space(5.0);
    ui.label(format!("{CHART_BAR} Chunk Progress:"));

    ui.scope(|ui| {
        #[cfg(feature = "branding")]
        {
            let (r, g, b) = branding::BACKGROUND_COLOR;
            let alpha = (branding::UI_ELEMENT_OPACITY * 255.0) as u8;
            ui.visuals_mut().widgets.noninteractive.bg_fill =
                egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
        }

        for (size, _) in params.test_sizes.iter().filter(|(_, enabled)| *enabled) {
            let (progress, progress_text, color) = chunk_progress_row(params, *size);
            let text_color = if progress_text.contains("Waiting") {
                egui::Color32::WHITE
            } else {
                egui::Color32::BLACK
            };

            ui.add(
                egui::ProgressBar::new(progress)
                    .text(egui::RichText::new(progress_text).color(text_color))
                    .fill(color),
            );
        }
    });
}

fn chunk_progress_row(
    params: &ResultsPanelParams<'_>,
    size: usize,
) -> (f32, String, egui::Color32) {
    if let Some((_, completion_time)) = params
        .test_state
        .completed_chunks
        .iter()
        .find(|(s, _)| s == &size)
    {
        return (
            1.0,
            format!(
                "{}: {:.1}s {}",
                get_size_label(size),
                completion_time,
                CHECK
            ),
            egui::Color32::from_rgb(46, 204, 113),
        );
    }

    if params.test_state.current_test_size == Some(size) {
        if let Some(start_time) = params.test_state.test_start_time {
            let elapsed = params
                .test_state
                .test_end_time
                .unwrap_or_else(|| start_time.elapsed().as_secs_f64());
            let progress = (elapsed / params.duration as f64).min(1.0) as f32;
            let color = if params.test_state.test_end_time.is_some() {
                egui::Color32::from_rgb(46, 204, 113)
            } else {
                egui::Color32::from_rgb(52, 152, 219)
            };
            return (
                progress,
                format!(
                    "{}: {:.1}s / {}s",
                    get_size_label(size),
                    elapsed,
                    params.duration
                ),
                color,
            );
        }
        return (
            0.0,
            format!("{}: Waiting...", get_size_label(size)),
            egui::Color32::BLACK,
        );
    }

    (
        0.0,
        format!("{}: Waiting...", get_size_label(size)),
        egui::Color32::BLACK,
    )
}
