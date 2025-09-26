use crate::ui::helpers::get_size_label;
use crate::ui::types::{PlotMetric, TestResults};
use eframe::egui;

pub fn render_results_table(
    ui: &mut egui::Ui,
    table_id: &str,
    results: &TestResults,
    metric: PlotMetric,
    title: &str,
) {
    ui.heading(title);
    if let Ok(results) = results.lock() {
        egui::Grid::new(table_id)
            .striped(true)
            .spacing([15.0, 4.0])
            .show(ui, |ui| {
                ui.label("Size");
                ui.label("Min");
                ui.label("Avg");
                ui.label("Max");
                ui.end_row();

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
                        let max_val = points.iter().map(|&(_, y)| y).fold(0.0, f64::max);
                        let min_val = points.iter().map(|&(_, y)| y).fold(f64::INFINITY, f64::min);
                        let avg_val =
                            points.iter().map(|&(_, y)| y).sum::<f64>() / points.len() as f64;

                        ui.label(get_size_label(*read_size));
                        match metric {
                            PlotMetric::Reads => {
                                ui.label(format!("{}", min_val as u64));
                                ui.label(format!("{avg_val:.0}"));
                                ui.label(format!("{}", max_val as u64));
                            }
                            _ => {
                                ui.label(format!("{min_val:.1}"));
                                ui.label(format!("{avg_val:.1}"));
                                ui.label(format!("{max_val:.1}"));
                            }
                        }
                        ui.end_row();
                    }
                }
            });
    }
}
