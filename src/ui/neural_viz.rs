use egui;

use crate::brain::BrainStorage;
use crate::config;

const SENSOR_LABELS: &[&str] = &[
    "L.Prox", "R.Prox", "Food", "Entity", "Obstacle", "Phero", "Sig.R", "Sig.G", "Sig.B", "Energy",
    "Health", "Age", "Speed", "Carry", "Adjacent",
];

const MOTOR_LABELS: &[&str] = &[
    "Fwd", "Turn", "Eat", "Attack", "Share", "Pickup", "Repro", "Out.R", "Out.G", "Out.B",
];

/// Draw neural network visualization content for the selected entity.
pub fn draw_neural_viz_content(ui: &mut egui::Ui, brains: &BrainStorage, slot: usize) {
    if slot >= brains.active.len() || !brains.active[slot] {
        return;
    }

    let outputs = match brains.slot_outputs(slot) {
        Some(o) if !o.is_empty() => o,
        _ => return,
    };
    let states = match brains.slot_states(slot) {
        Some(s) if s.len() == outputs.len() => s,
        _ => return,
    };
    let weights = match brains.slot_weights(slot) {
        Some(w) if w.len() == outputs.len() * outputs.len() => w,
        _ => return,
    };

    let available = ui.available_size();
    let (response, painter) = ui.allocate_painter(available, egui::Sense::hover());
    let rect = response.rect;

    let total_n = outputs.len();
    let sensor_n = config::BRAIN_SENSOR_NEURONS.min(total_n);
    let motor_n = config::BRAIN_MOTOR_NEURONS.min(total_n.saturating_sub(sensor_n));
    let inter_n = total_n.saturating_sub(sensor_n + motor_n);

    let col_x = [rect.left() + 60.0, rect.center().x, rect.right() - 60.0];
    let neuron_positions: Vec<egui::Pos2> = (0..total_n)
        .map(|i| {
            if i < sensor_n {
                let spacing = (rect.height() - 20.0) / sensor_n.max(1) as f32;
                egui::pos2(col_x[0], rect.top() + 10.0 + spacing * (i as f32 + 0.5))
            } else if i < sensor_n + inter_n {
                let local = i - sensor_n;
                let spacing = (rect.height() - 20.0) / inter_n.max(1) as f32;
                egui::pos2(col_x[1], rect.top() + 10.0 + spacing * (local as f32 + 0.5))
            } else {
                let local = i - sensor_n - inter_n;
                let spacing = (rect.height() - 20.0) / motor_n.max(1) as f32;
                egui::pos2(col_x[2], rect.top() + 10.0 + spacing * (local as f32 + 0.5))
            }
        })
        .collect();

    for to in 0..total_n {
        for from in 0..total_n {
            let w = weights[to * total_n + from];
            if w.abs() < 0.5 {
                continue;
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

    for i in 0..total_n {
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

        painter.circle(
            pos,
            10.0,
            fill,
            egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
        );

        let label = if i < sensor_n {
            SENSOR_LABELS.get(i).copied().unwrap_or("?").to_string()
        } else if i < sensor_n + inter_n {
            format!("Inter.{}", i - sensor_n)
        } else {
            let local = i - sensor_n - inter_n;
            MOTOR_LABELS.get(local).copied().unwrap_or("?").to_string()
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
}
