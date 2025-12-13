use crate::ui::constants::*;
use crate::ui::helpers::{color_for_size, get_size_label};
use crate::ui::types::{PlotMetric, ResultsPanelParams, TestResults};
use eframe::egui;
use egui_plot::{Corner, Legend, Line, Plot, PlotPoints};

#[cfg(feature = "branding")]
use crate::branding;

pub fn render_plot_column(
    ui: &mut egui::Ui,
    heading: &str,
    plot_id: &'static str,
    metric: PlotMetric,
    params: &ResultsPanelParams<'_>,
    width: f32,
    height: f32,
) {
    ui.heading(heading);
    ui.add_space(6.0);
    render_plot(
        ui,
        plot_id,
        params.results,
        metric,
        params.duration,
        width,
        height,
    );
}

fn base_plot(id: &'static str, duration: u64, width: f32, height: f32) -> Plot<'static> {
    Plot::new(id)
        .height(height)
        .width(width)
        .include_x(0.0)
        .include_x(duration as f64)
        .show_grid(PLOT_SHOW_GRID)
        .allow_drag(PLOT_ALLOW_DRAG)
        .allow_zoom(PLOT_ALLOW_ZOOM)
        .allow_scroll(PLOT_ALLOW_SCROLL)
        .allow_boxed_zoom(PLOT_ALLOW_BOXED_ZOOM)
        .allow_double_click_reset(PLOT_ALLOW_DOUBLE_CLICK_RESET)
}

fn y_range_for(results: &TestResults, metric: PlotMetric) -> (f64, f64) {
    let mut min_v: Option<f64> = None;
    let mut max_v: Option<f64> = None;
    if let Ok(results) = results.lock() {
        for (_, (throughput_points, reads_points, latency_points)) in results.iter() {
            let pts: &Vec<(f64, f64)> = match metric {
                PlotMetric::Throughput => throughput_points,
                PlotMetric::Reads => reads_points,
                PlotMetric::Latency => latency_points,
            };
            for &(_, y) in pts.iter() {
                min_v = Some(min_v.map_or(y, |m| m.min(y)));
                max_v = Some(max_v.map_or(y, |m| m.max(y)));
            }
        }
    }
    match (min_v, max_v) {
        (Some(miny), Some(maxy)) if maxy > miny => {
            let range = maxy - miny;
            (miny - 0.1 * range, maxy + 0.1 * range)
        }
        _ => (0.0, 1.0),
    }
}

fn render_plot(
    ui: &mut egui::Ui,
    plot_id: &'static str,
    results: &TestResults,
    metric: PlotMetric,
    duration: u64,
    width: f32,
    height: f32,
) {
    let (y_min, y_max) = y_range_for(results, metric);

    #[cfg(feature = "branding")]
    {
        let (r, g, b) = branding::BACKGROUND_COLOR;
        let alpha = (branding::UI_ELEMENT_OPACITY * 255.0) as u8;
        ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
    }

    base_plot(plot_id, duration, width, height)
        .include_y(y_min)
        .include_y(y_max)
        .legend(Legend::default().position(Corner::RightTop))
        .show(ui, |plot_ui| {
            if let Ok(results) = results.lock() {
                let mut sorted_results: Vec<_> = results.iter().collect();
                sorted_results.sort_by(|a, b| b.0.cmp(&a.0));
                for (read_size, (throughput_points, reads_points, latency_points)) in sorted_results
                {
                    let points = match metric {
                        PlotMetric::Throughput => throughput_points,
                        PlotMetric::Reads => reads_points,
                        PlotMetric::Latency => latency_points,
                    };
                    if !points.is_empty() {
                        let color = color_for_size(*read_size);
                        let plot_points: PlotPoints<'_> =
                            points.iter().map(|&(x, y)| [x, y]).collect();
                        plot_ui.line(
                            Line::new(get_size_label(*read_size), plot_points)
                                .color(color)
                                .width(2.0),
                        );
                    }
                }
            }
        });
}
