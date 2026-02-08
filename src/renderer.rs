use macroquad::prelude::*;

use crate::camera::CameraController;
use crate::combat::MeatItem;
use crate::entity::EntityArena;
use crate::environment;
use crate::sensory::{EntityRays, HitType};
use crate::signals::{self, SignalState};
use crate::simulation::{FoodItem, SimState};
use crate::world::World;

const BG_COLOR: Color = Color::new(0.02, 0.03, 0.08, 1.0);

/// Draw the world scene (everything that should be affected by bloom).
/// If render_target is Some, renders into that target; otherwise renders to screen.
pub fn draw_world_scene(
    sim: &SimState,
    camera: &CameraController,
    alpha: f32,
    render_target: Option<RenderTarget>,
) {
    if let Some(ref rt) = render_target {
        // Render to offscreen target
        let cam = Camera2D {
            target: camera.smooth_target,
            zoom: vec2(
                camera.smooth_zoom / rt.texture.width() * 2.0,
                -camera.smooth_zoom / rt.texture.height() * 2.0,
            ),
            render_target: Some(rt.clone()),
            ..Default::default()
        };
        set_camera(&cam);
    } else {
        let cam = camera.to_macroquad_camera();
        set_camera(&cam);
    }

    // Note: when rendering to a target, we clear it; otherwise rely on main draw() clearing
    if render_target.is_some() {
        clear_background(BG_COLOR);
    }

    draw_world_background(&sim.world, camera);

    // Terrain
    environment::draw_terrain(&sim.environment.terrain);

    // Pheromone overlay (under everything)
    signals::draw_pheromone_overlay(&sim.pheromone_grid, &sim.world);

    draw_food(&sim.food);
    draw_meat(&sim.meat);

    // Draw signal auras behind entities
    for (idx, entity) in sim.arena.iter_alive() {
        if idx < sim.signals.len() {
            let pos = entity.prev_pos.lerp(entity.pos, alpha);
            signals::draw_signal_aura(pos, entity.radius, &sim.signals[idx]);
        }
    }

    draw_entities(&sim.arena, &sim.signals, alpha);

    // Draw sensor rays if enabled
    if sim.show_rays {
        draw_sensor_rays(&sim.last_rays);
    }

    // Draw combat lines
    for event in &sim.combat_events {
        draw_line(
            event.attacker_pos.x, event.attacker_pos.y,
            event.target_pos.x, event.target_pos.y,
            2.0, Color::new(1.0, 0.3, 0.1, 0.6),
        );
    }

    // Particles
    sim.particles.draw();

    // Storm visual
    if let Some(ref storm) = sim.environment.storm {
        environment::draw_storm(storm);
    }

    // Day/night tint overlay
    environment::draw_day_night_overlay(sim.environment.day_brightness());
}

/// Standard draw (no bloom): renders directly to screen.
pub fn draw(sim: &SimState, camera: &CameraController, alpha: f32) {
    clear_background(BG_COLOR);

    draw_world_scene(sim, camera, alpha, None);

    set_default_camera();
    draw_hud(
        &sim.arena,
        sim.tick_count,
        sim.paused,
        sim.food.len(),
        sim.environment.season.name(),
        sim.environment.is_day(),
        sim.environment.storm.is_some(),
    );
}

/// Draw with bloom pipeline.
pub fn draw_with_bloom(
    sim: &SimState,
    camera: &CameraController,
    alpha: f32,
    bloom: &crate::post_processing::BloomPipeline,
) {
    // Render world scene to bloom's scene render target
    draw_world_scene(sim, camera, alpha, Some(bloom.scene_render_target()));

    // Run bloom post-processing and composite to screen
    bloom.apply();

    // Draw HUD on top (after bloom, in screen space)
    draw_hud(
        &sim.arena,
        sim.tick_count,
        sim.paused,
        sim.food.len(),
        sim.environment.season.name(),
        sim.environment.is_day(),
        sim.environment.storm.is_some(),
    );
}

fn draw_world_background(world: &World, camera: &CameraController) {
    draw_rectangle_lines(
        0.0, 0.0, world.width, world.height, 2.0,
        Color::new(0.15, 0.18, 0.25, 1.0),
    );

    if camera.smooth_zoom > 0.15 {
        let grid_size = 100.0;
        let a = ((camera.smooth_zoom - 0.15) / 0.3).clamp(0.0, 1.0) * 0.5;
        let grid_color = Color::new(0.08, 0.10, 0.15, a);

        let mut x = 0.0;
        while x <= world.width {
            draw_line(x, 0.0, x, world.height, 1.0, grid_color);
            x += grid_size;
        }
        let mut y = 0.0;
        while y <= world.height {
            draw_line(0.0, y, world.width, y, 1.0, grid_color);
            y += grid_size;
        }
    }
}

fn draw_food(food: &[FoodItem]) {
    for item in food {
        draw_circle(item.pos.x, item.pos.y, 6.0, Color::new(0.1, 0.5, 0.1, 0.3));
        draw_circle(item.pos.x, item.pos.y, 3.5, Color::new(0.2, 0.85, 0.2, 0.9));
    }
}

fn draw_meat(meat: &[MeatItem]) {
    for item in meat {
        let fade = (item.decay_timer / crate::config::MEAT_DECAY_TIME).clamp(0.0, 1.0);
        draw_circle(item.pos.x, item.pos.y, 5.0, Color::new(0.6, 0.2, 0.15, 0.3 * fade));
        draw_circle(item.pos.x, item.pos.y, 3.0, Color::new(0.8, 0.3, 0.2, 0.85 * fade));
    }
}

fn draw_entities(arena: &EntityArena, _signals: &[SignalState], alpha: f32) {
    for (_idx, entity) in arena.iter_alive() {
        let pos = entity.prev_pos.lerp(entity.pos, alpha);
        draw_entity_shape(pos, entity.heading, entity.radius, entity.color, entity.energy);
    }
}

fn draw_entity_shape(pos: Vec2, heading: f32, radius: f32, color: Color, energy: f32) {
    let dir = Vec2::from_angle(heading);
    let perp = Vec2::new(-dir.y, dir.x);

    let front = pos + dir * radius * 1.6;
    let back_left = pos - dir * radius * 0.8 + perp * radius * 0.9;
    let back_right = pos - dir * radius * 0.8 - perp * radius * 0.9;
    draw_triangle(front, back_left, back_right, color);

    let body_color = Color::new(color.r * 0.85, color.g * 0.85, color.b * 0.85, 1.0);
    draw_circle(pos.x, pos.y, radius * 0.55, body_color);

    let eye_offset = radius * 0.35;
    let eye_pos = pos + dir * radius * 0.5;
    let eye_l = eye_pos + perp * eye_offset;
    let eye_r = eye_pos - perp * eye_offset;
    draw_circle(eye_l.x, eye_l.y, radius * 0.12, Color::new(0.9, 0.95, 1.0, 0.9));
    draw_circle(eye_r.x, eye_r.y, radius * 0.12, Color::new(0.9, 0.95, 1.0, 0.9));

    // Energy bar
    let bar_width = radius * 2.0;
    let bar_y = pos.y - radius * 2.0;
    let energy_frac = (energy / crate::config::MAX_ENTITY_ENERGY).clamp(0.0, 1.0);
    let bar_color = if energy_frac > 0.5 {
        Color::new(0.2, 0.9, 0.2, 0.7)
    } else if energy_frac > 0.25 {
        Color::new(0.9, 0.9, 0.2, 0.7)
    } else {
        Color::new(0.9, 0.2, 0.2, 0.7)
    };

    draw_line(
        pos.x - bar_width * 0.5, bar_y,
        pos.x + bar_width * 0.5, bar_y,
        2.0, Color::new(0.15, 0.15, 0.15, 0.5),
    );
    draw_line(
        pos.x - bar_width * 0.5, bar_y,
        pos.x - bar_width * 0.5 + bar_width * energy_frac, bar_y,
        2.0, bar_color,
    );
}

fn draw_sensor_rays(all_rays: &[Option<EntityRays>]) {
    for slot_rays in all_rays {
        if let Some(ref rays) = slot_rays {
            for (start, end, hit_type) in &rays.rays {
                let color = match hit_type {
                    HitType::Nothing => Color::new(0.3, 0.3, 0.3, 0.15),
                    HitType::Entity => Color::new(1.0, 0.3, 0.3, 0.4),
                    HitType::Food => Color::new(0.3, 1.0, 0.3, 0.4),
                    HitType::Wall => Color::new(0.5, 0.5, 0.8, 0.4),
                };
                draw_line(start.x, start.y, end.x, end.y, 1.0, color);
            }
        }
    }
}

fn draw_hud(
    arena: &EntityArena,
    tick_count: u64,
    paused: bool,
    food_count: usize,
    season: &str,
    is_day: bool,
    storm_active: bool,
) {
    let tc = Color::new(0.7, 0.75, 0.8, 1.0);
    let sh = Color::new(0.0, 0.0, 0.0, 0.5);

    let fps_text = format!("FPS: {}", get_fps());
    draw_text(&fps_text, 11.0, 21.0, 18.0, sh);
    draw_text(&fps_text, 10.0, 20.0, 18.0, tc);

    let ent_text = format!("Entities: {}", arena.count);
    draw_text(&ent_text, 11.0, 41.0, 18.0, sh);
    draw_text(&ent_text, 10.0, 40.0, 18.0, tc);

    let food_text = format!("Food: {}", food_count);
    draw_text(&food_text, 11.0, 61.0, 18.0, sh);
    draw_text(&food_text, 10.0, 60.0, 18.0, tc);

    let tick_text = format!("Tick: {}", tick_count);
    draw_text(&tick_text, 11.0, 81.0, 18.0, sh);
    draw_text(&tick_text, 10.0, 80.0, 18.0, tc);

    let day_str = if is_day { "Day" } else { "Night" };
    let env_text = format!("{} | {} {}", season, day_str, if storm_active { "| STORM" } else { "" });
    draw_text(&env_text, 11.0, 101.0, 18.0, sh);
    draw_text(&env_text, 10.0, 100.0, 18.0, tc);

    if paused {
        let pause_text = "PAUSED (Space to resume)";
        let tw = measure_text(pause_text, None, 24, 1.0).width;
        let x = screen_width() * 0.5 - tw * 0.5;
        draw_text(pause_text, x + 1.0, 31.0, 24.0, sh);
        draw_text(pause_text, x, 30.0, 24.0, Color::new(1.0, 0.8, 0.2, 0.9));
    }
}
