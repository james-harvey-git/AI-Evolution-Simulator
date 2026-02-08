use egui;
use macroquad::prelude::*;

use crate::camera::CameraController;
use crate::simulation::SimState;

const MINIMAP_SIZE: f32 = 180.0;

/// Draw a minimap showing entity positions, food, and camera viewport.
pub fn draw_minimap(ctx: &egui::Context, sim: &SimState, camera: &CameraController) {
    egui::Window::new("Minimap")
        .default_pos(egui::pos2(
            macroquad::prelude::screen_width() - MINIMAP_SIZE - 20.0,
            macroquad::prelude::screen_height() - MINIMAP_SIZE - 60.0,
        ))
        .fixed_size(egui::vec2(MINIMAP_SIZE, MINIMAP_SIZE))
        .title_bar(false)
        .show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(egui::vec2(MINIMAP_SIZE, MINIMAP_SIZE), egui::Sense::click());
            let rect = response.rect;

            // Background
            painter.rect_filled(rect, 2.0, egui::Color32::from_rgba_unmultiplied(10, 15, 25, 220));

            let world_w = sim.world.width;
            let world_h = sim.world.height;

            let to_minimap = |world_pos: Vec2| -> egui::Pos2 {
                egui::pos2(
                    rect.left() + (world_pos.x / world_w) * MINIMAP_SIZE,
                    rect.top() + (world_pos.y / world_h) * MINIMAP_SIZE,
                )
            };

            // Draw food as tiny green dots
            for food in &sim.food {
                let p = to_minimap(food.pos);
                painter.circle_filled(p, 1.0, egui::Color32::from_rgb(50, 150, 50));
            }

            // Draw meat as tiny red dots
            for item in &sim.meat {
                let p = to_minimap(item.pos);
                painter.circle_filled(p, 1.0, egui::Color32::from_rgb(150, 60, 50));
            }

            // Draw entities
            for (_idx, entity) in sim.arena.iter_alive() {
                let p = to_minimap(entity.pos);
                let c = entity.color;
                let color = egui::Color32::from_rgb(
                    (c.r * 255.0) as u8,
                    (c.g * 255.0) as u8,
                    (c.b * 255.0) as u8,
                );
                painter.circle_filled(p, 2.0, color);
            }

            // Draw storm
            if let Some(ref storm) = sim.environment.storm {
                let center = to_minimap(storm.center);
                let r = (storm.radius / world_w) * MINIMAP_SIZE;
                painter.circle(
                    center,
                    r,
                    egui::Color32::from_rgba_unmultiplied(100, 100, 150, 60),
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(150, 150, 200, 100)),
                );
            }

            // Draw camera viewport rectangle
            let cam_center = camera.smooth_target;
            let half_w = macroquad::prelude::screen_width() / (2.0 * camera.smooth_zoom);
            let half_h = macroquad::prelude::screen_height() / (2.0 * camera.smooth_zoom);

            let tl = to_minimap(vec2(cam_center.x - half_w, cam_center.y - half_h));
            let br = to_minimap(vec2(cam_center.x + half_w, cam_center.y + half_h));
            let cam_rect = egui::Rect::from_min_max(tl, br);
            painter.rect_stroke(
                cam_rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120)),
                egui::StrokeKind::Outside,
            );

            // Border
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                egui::StrokeKind::Inside,
            );
        });
}
