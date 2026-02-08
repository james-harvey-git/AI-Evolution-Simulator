use egui;

use crate::brain::BrainStorage;
use crate::config;
use crate::genome::N;

const NEURON_LABELS: &[&str] = &[
    "L.Prox", "R.Prox", "Food", "Entity", "Energy", "Env", // sensors
    "Inter.0", "Inter.1",                                     // interneurons
    "Fwd", "Turn", "Attack", "Signal",                       // motors
];

/// Draw a neural network visualization for the selected entity's brain.
pub fn draw_neural_viz(ctx: &egui::Context, brains: &BrainStorage, slot: usize) {
    if slot >= brains.active.len() || !brains.active[slot] {
        return;
    }

    egui::Window::new("Neural Network")
        .default_pos(egui::pos2(300.0, 60.0))
        .default_size(egui::vec2(360.0, 340.0))
        .resizable(true)
        .show(ctx, |ui| {
            let outputs = &brains.outputs[slot];
            let weights = &brains.weights[slot];
            let states = &brains.states[slot];

            let available = ui.available_size();
            let (response, painter) =
                ui.allocate_painter(available, egui::Sense::hover());
            let rect = response.rect;

            let sensor_n = config::BRAIN_SENSOR_NEURONS;
            let inter_n = config::BRAIN_INTERNEURONS;
            let motor_n = N - sensor_n - inter_n;

            // Layout neurons in 3 columns: sensors | interneurons | motors
            let col_x = [
                rect.left() + 60.0,
                rect.center().x,
                rect.right() - 60.0,
            ];

            let neuron_positions: Vec<egui::Pos2> = (0..N)
                .map(|i| {
                    if i < sensor_n {
                        // Sensor column
                        let spacing = (rect.height() - 20.0) / sensor_n as f32;
                        egui::pos2(col_x[0], rect.top() + 10.0 + spacing * (i as f32 + 0.5))
                    } else if i < sensor_n + inter_n {
                        // Interneuron column
                        let local = i - sensor_n;
                        let spacing = (rect.height() - 20.0) / inter_n as f32;
                        egui::pos2(col_x[1], rect.top() + 10.0 + spacing * (local as f32 + 0.5))
                    } else {
                        // Motor column
                        let local = i - sensor_n - inter_n;
                        let spacing = (rect.height() - 20.0) / motor_n as f32;
                        egui::pos2(col_x[2], rect.top() + 10.0 + spacing * (local as f32 + 0.5))
                    }
                })
                .collect();

            // Draw connections (weight lines)
            for to in 0..N {
                for from in 0..N {
                    let w = weights[to][from];
                    if w.abs() < 0.5 {
                        continue; // skip weak connections
                    }
                    let alpha = (w.abs() / 16.0).clamp(0.0, 1.0);
                    let width = 0.5 + alpha * 2.5;
                    let color = if w > 0.0 {
                        egui::Color32::from_rgba_unmultiplied(100, 200, 100, (alpha * 180.0) as u8)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(200, 80, 80, (alpha * 180.0) as u8)
                    };
                    painter.line_segment(
                        [neuron_positions[from], neuron_positions[to]],
                        egui::Stroke::new(width, color),
                    );
                }
            }

            // Draw neurons
            for i in 0..N {
                let pos = neuron_positions[i];
                let activation = outputs[i];
                let brightness = (activation * 255.0).clamp(0.0, 255.0) as u8;

                let fill = if i < sensor_n {
                    egui::Color32::from_rgb(brightness / 2, brightness, brightness / 2)
                } else if i < sensor_n + inter_n {
                    egui::Color32::from_rgb(brightness, brightness, brightness / 2)
                } else {
                    egui::Color32::from_rgb(brightness / 2, brightness / 2, brightness)
                };

                let radius = 10.0;
                painter.circle(
                    pos,
                    radius,
                    fill,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
                );

                // Label
                let label = if i < NEURON_LABELS.len() {
                    NEURON_LABELS[i]
                } else {
                    "?"
                };

                let label_x = if i < sensor_n {
                    pos.x - 55.0
                } else if i >= sensor_n + inter_n {
                    pos.x + 14.0
                } else {
                    pos.x - 16.0
                };

                painter.text(
                    egui::pos2(label_x, pos.y - 5.0),
                    if i < sensor_n {
                        egui::Align2::RIGHT_CENTER
                    } else {
                        egui::Align2::LEFT_CENTER
                    },
                    format!("{label}\n{:.2}", states[i]),
                    egui::FontId::proportional(9.0),
                    egui::Color32::from_gray(200),
                );
            }
        });
}
