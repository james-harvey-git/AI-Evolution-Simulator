use macroquad::prelude::*;

use crate::config;
use crate::entity::{EntityArena, EntityId};

pub struct CameraController {
    pub target: Vec2,
    pub zoom: f32,
    pub following: Option<EntityId>,
    pub smooth_target: Vec2,
    pub smooth_zoom: f32,
    is_dragging: bool,
    drag_start: Vec2,
    drag_cam_start: Vec2,
}

impl CameraController {
    pub fn new(initial_target: Vec2) -> Self {
        let initial_zoom = 0.3;
        Self {
            target: initial_target,
            zoom: initial_zoom,
            following: None,
            smooth_target: initial_target,
            smooth_zoom: initial_zoom,
            is_dragging: false,
            drag_start: Vec2::ZERO,
            drag_cam_start: Vec2::ZERO,
        }
    }

    pub fn update(&mut self, arena: &EntityArena, dt: f32) {
        // Follow selected entity
        if let Some(id) = self.following {
            if let Some(entity) = arena.get(id) {
                self.target = entity.pos;
            } else {
                self.following = None;
            }
        }

        // WASD pan (only when not following)
        if self.following.is_none() {
            let pan_speed = config::CAMERA_PAN_SPEED / self.zoom;
            if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                self.target.y -= pan_speed * dt;
            }
            if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                self.target.y += pan_speed * dt;
            }
            if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
                self.target.x -= pan_speed * dt;
            }
            if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
                self.target.x += pan_speed * dt;
            }
        }

        // Middle mouse drag
        if is_mouse_button_pressed(MouseButton::Middle) {
            self.is_dragging = true;
            self.drag_start = Vec2::from(mouse_position());
            self.drag_cam_start = self.target;
            self.following = None;
        }
        if is_mouse_button_released(MouseButton::Middle) {
            self.is_dragging = false;
        }
        if self.is_dragging {
            let mouse_pos = Vec2::from(mouse_position());
            let delta = (self.drag_start - mouse_pos) / self.smooth_zoom;
            self.target = self.drag_cam_start + delta;
        }

        // Scroll zoom
        let (_, scroll_y) = mouse_wheel();
        if scroll_y != 0.0 {
            let zoom_factor = 1.0 + scroll_y.signum() * config::CAMERA_ZOOM_SPEED;
            self.zoom = (self.zoom * zoom_factor).clamp(config::CAMERA_ZOOM_MIN, config::CAMERA_ZOOM_MAX);
        }

        // Smooth interpolation
        let smooth = 1.0 - (-config::CAMERA_SMOOTH_SPEED * dt).exp();
        self.smooth_target = self.smooth_target.lerp(self.target, smooth);
        self.smooth_zoom += (self.zoom - self.smooth_zoom) * smooth;
    }

    pub fn to_macroquad_camera(&self) -> Camera2D {
        Camera2D {
            target: self.smooth_target,
            zoom: vec2(
                self.smooth_zoom / screen_width() * 2.0,
                -self.smooth_zoom / screen_height() * 2.0,
            ),
            ..Default::default()
        }
    }

    /// Convert screen position to world position.
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let cam = self.to_macroquad_camera();
        let ndc_x = (screen_pos.x / screen_width()) * 2.0 - 1.0;
        let ndc_y = -((screen_pos.y / screen_height()) * 2.0 - 1.0);
        vec2(
            self.smooth_target.x + ndc_x / cam.zoom.x,
            self.smooth_target.y + ndc_y / cam.zoom.y,
        )
    }

    /// Find the entity closest to a world position within a given radius.
    pub fn pick_entity(
        &self,
        world_pos: Vec2,
        arena: &EntityArena,
        max_dist: f32,
    ) -> Option<EntityId> {
        let max_dist_sq = max_dist * max_dist;
        let mut best: Option<(f32, EntityId)> = None;

        for (idx, entity) in arena.iter_alive() {
            let dist_sq = (entity.pos - world_pos).length_squared();
            if dist_sq < max_dist_sq {
                if best.is_none() || dist_sq < best.unwrap().0 {
                    best = Some((
                        dist_sq,
                        EntityId {
                            index: idx as u32,
                            generation: arena.generations[idx],
                        },
                    ));
                }
            }
        }

        best.map(|(_, id)| id)
    }
}
