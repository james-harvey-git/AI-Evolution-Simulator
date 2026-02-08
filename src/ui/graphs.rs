use egui;

use crate::stats::SimStats;

/// Draw population and energy graphs.
pub fn draw_graphs(ctx: &egui::Context, stats: &SimStats) {
    egui::Window::new("Statistics")
        .default_pos(egui::pos2(300.0, 420.0))
        .default_size(egui::vec2(400.0, 300.0))
        .resizable(true)
        .show(ctx, |ui| {
            ui.collapsing("Population", |ui| {
                draw_line_graph(ui, &stats.population, "pop_graph", egui::Color32::from_rgb(100, 200, 100));
            });

            ui.collapsing("Average Energy", |ui| {
                draw_line_graph(ui, &stats.avg_energy, "energy_graph", egui::Color32::from_rgb(200, 200, 100));
            });

            ui.collapsing("Food Count", |ui| {
                draw_line_graph(ui, &stats.food_count, "food_graph", egui::Color32::from_rgb(100, 200, 100));
            });

            ui.collapsing("Births / Deaths", |ui| {
                let size = egui::vec2(ui.available_width(), 80.0);
                let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
                let rect = response.rect;

                draw_line_in_rect(&painter, &stats.births, rect, egui::Color32::from_rgb(100, 180, 255));
                draw_line_in_rect(&painter, &stats.deaths, rect, egui::Color32::from_rgb(255, 100, 100));

                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(100, 180, 255), "Births");
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "Deaths");
                });
            });

            ui.collapsing("Average Generation", |ui| {
                draw_line_graph(ui, &stats.avg_generation, "gen_graph", egui::Color32::from_rgb(200, 150, 255));
            });
        });
}

fn draw_line_graph(
    ui: &mut egui::Ui,
    buffer: &crate::stats::RingBuffer,
    _id: &str,
    color: egui::Color32,
) {
    let size = egui::vec2(ui.available_width(), 80.0);
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 2.0, egui::Color32::from_gray(20));

    draw_line_in_rect(&painter, buffer, rect, color);

    // Current value label
    if let Some(val) = buffer.last() {
        painter.text(
            egui::pos2(rect.right() - 4.0, rect.top() + 2.0),
            egui::Align2::RIGHT_TOP,
            format!("{val:.0}"),
            egui::FontId::proportional(10.0),
            egui::Color32::from_gray(200),
        );
    }
}

fn draw_line_in_rect(
    painter: &egui::Painter,
    buffer: &crate::stats::RingBuffer,
    rect: egui::Rect,
    color: egui::Color32,
) {
    let len = buffer.len();
    if len < 2 {
        return;
    }

    let samples: Vec<f32> = buffer.iter().collect();

    let max_val = samples
        .iter()
        .cloned()
        .fold(1.0f32, f32::max);
    let min_val = samples
        .iter()
        .cloned()
        .fold(max_val, f32::min);
    let range = (max_val - min_val).max(1.0);

    let points: Vec<egui::Pos2> = samples
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = rect.left() + (i as f32 / (len - 1) as f32) * rect.width();
            let y = rect.bottom() - ((v - min_val) / range) * rect.height();
            egui::pos2(x, y)
        })
        .collect();

    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.5, color));
    }
}
