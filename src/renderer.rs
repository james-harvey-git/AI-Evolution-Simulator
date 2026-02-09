use macroquad::prelude::*;

use crate::camera::CameraController;
use crate::combat::MeatItem;
use crate::config;
use crate::entity::EntityArena;
use crate::environment;
use crate::sensory::{EntityRays, HitType};
use crate::signals;
use crate::simulation::{FoodItem, SimState};
use crate::visual::VisualSettings;
use crate::world::World;

const BG_COLOR: Color = Color::new(0.01, 0.02, 0.06, 1.0);

#[inline]
fn normalized_alpha(alpha: f32) -> f32 {
    alpha.clamp(0.0, 1.0)
}

#[inline]
fn interpolate_wrapped(world: &World, from: Vec2, to: Vec2, alpha: f32) -> Vec2 {
    let a = normalized_alpha(alpha);
    world.wrap(from + world.delta(from, to) * a)
}

/// Draw the world scene (everything that should be affected by bloom).
/// If render_target is Some, renders into that target; otherwise renders to screen.
pub fn draw_world_scene(
    sim: &SimState,
    camera: &CameraController,
    alpha: f32,
    render_target: Option<RenderTarget>,
    visual_settings: VisualSettings,
) {
    let alpha = normalized_alpha(alpha);
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
    environment::draw_terrain(&sim.environment.terrain, visual_settings, sim.tick_count);
    environment::draw_toxic_zones(&sim.environment.toxic_zones);
    environment::draw_walls(&sim.environment.walls);

    // Pheromone overlay (under everything)
    signals::draw_pheromone_overlay(&sim.pheromone_grid, &sim.world);

    draw_food(&sim.food);
    draw_meat(&sim.meat);

    // Draw signal auras behind entities
    for (idx, entity) in sim.arena.iter_alive() {
        if idx < sim.signals.len() {
            let pos = interpolate_wrapped(&sim.world, entity.prev_pos, entity.pos, alpha);
            signals::draw_signal_aura(pos, entity.radius, &sim.signals[idx]);
        }
    }

    draw_entities(
        &sim.world,
        &sim.arena,
        alpha,
        sim.tick_count,
        visual_settings,
        camera.smooth_zoom,
        camera.following.map(|id| id.index as usize),
    );

    // Draw sensor rays if enabled
    if sim.show_rays {
        draw_sensor_rays(&sim.last_rays);
    }

    // Draw combat lines
    for event in &sim.combat_events {
        draw_line(
            event.attacker_pos.x,
            event.attacker_pos.y,
            event.target_pos.x,
            event.target_pos.y,
            2.0,
            Color::new(1.0, 0.3, 0.1, 0.6),
        );
    }

    // Particles
    sim.particles.draw();

    // Storm visual
    if let Some(ref storm) = sim.environment.storm {
        if visual_settings.shelter_highlight_enabled {
            environment::draw_storm_shelter_highlights(
                storm,
                &sim.environment.walls,
                &sim.world,
                visual_settings,
            );
        }
        environment::draw_storm(storm, sim.tick_count, visual_settings);
    }

    // Day/night tint overlay
    environment::draw_day_night_overlay(sim.environment.day_brightness());
    draw_atmosphere_overlay(
        &sim.world,
        sim.environment.season,
        sim.environment.day_brightness(),
        sim.environment.storm.is_some(),
        sim.tick_count,
        visual_settings,
    );
}

/// Standard draw (no bloom): renders directly to screen.
pub fn draw(
    sim: &SimState,
    camera: &CameraController,
    alpha: f32,
    visual_settings: VisualSettings,
) {
    clear_background(BG_COLOR);

    draw_world_scene(sim, camera, alpha, None, visual_settings);

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
    visual_settings: VisualSettings,
) {
    // Render world scene to bloom's scene render target
    draw_world_scene(
        sim,
        camera,
        alpha,
        Some(bloom.scene_render_target()),
        visual_settings,
    );

    // Run bloom post-processing and composite to screen
    bloom.apply(visual_settings);

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
    let outer = 420.0;
    draw_rectangle(
        -outer,
        -outer,
        world.width + outer * 2.0,
        world.height + outer * 2.0,
        Color::new(0.005, 0.012, 0.035, 1.0),
    );

    draw_rectangle(
        0.0,
        0.0,
        world.width,
        world.height,
        Color::new(0.012, 0.03, 0.055, 0.94),
    );

    for i in 0..5 {
        let t = i as f32 / 5.0;
        let pad = 6.0 + t * 36.0;
        let alpha = (0.18 * (1.0 - t)).clamp(0.0, 1.0);
        draw_rectangle_lines(
            -pad,
            -pad,
            world.width + pad * 2.0,
            world.height + pad * 2.0,
            1.0,
            Color::new(0.08, 0.14, 0.26, alpha),
        );
    }

    draw_rectangle_lines(
        0.0,
        0.0,
        world.width,
        world.height,
        2.2,
        Color::new(0.22, 0.32, 0.5, 0.95),
    );

    if camera.smooth_zoom > 0.12 {
        let grid_size = 100.0;
        let a = ((camera.smooth_zoom - 0.12) / 0.35).clamp(0.0, 1.0) * 0.38;
        let grid_color = Color::new(0.09, 0.14, 0.2, a);

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

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
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
        draw_circle(
            item.pos.x,
            item.pos.y,
            5.0,
            Color::new(0.6, 0.2, 0.15, 0.3 * fade),
        );
        draw_circle(
            item.pos.x,
            item.pos.y,
            3.0,
            Color::new(0.8, 0.3, 0.2, 0.85 * fade),
        );
    }
}

fn draw_entities(
    world: &World,
    arena: &EntityArena,
    alpha: f32,
    tick_count: u64,
    visual_settings: VisualSettings,
    camera_zoom: f32,
    selected_slot: Option<usize>,
) {
    let show_vitals = camera_zoom >= 0.30;
    for (idx, entity) in arena.iter_alive() {
        let pos = interpolate_wrapped(world, entity.prev_pos, entity.pos, alpha);
        draw_entity_trail(
            world,
            entity.prev_pos,
            pos,
            entity.heading,
            entity.velocity.length(),
            entity.color,
            visual_settings,
        );
        draw_entity_shape(
            pos,
            entity.heading,
            entity.radius,
            entity.color,
            entity.energy,
            entity.velocity.length(),
            tick_count,
            visual_settings,
            show_vitals,
        );

        if Some(idx) == selected_slot {
            draw_circle_lines(
                pos.x,
                pos.y,
                entity.radius * 2.2,
                1.5,
                Color::new(0.92, 0.94, 1.0, 0.68),
            );
        }
    }
}

fn draw_entity_trail(
    world: &World,
    prev_pos: Vec2,
    pos: Vec2,
    heading: f32,
    speed: f32,
    color: Color,
    visual_settings: VisualSettings,
) {
    if !visual_settings.trails_enabled {
        return;
    }

    let speed_frac = (speed / config::ENTITY_MAX_SPEED).clamp(0.0, 1.0);
    let trail_len = (config::VISUAL_TRAIL_BASE_LENGTH
        + speed * config::VISUAL_TRAIL_SPEED_SCALE
        + world.distance(prev_pos, pos) * 0.6)
        .clamp(0.0, config::VISUAL_TRAIL_MAX_LENGTH);
    if trail_len < 1.0 {
        return;
    }

    let dir = if speed > 0.001 {
        world.delta(prev_pos, pos).normalize_or_zero()
    } else {
        Vec2::from_angle(heading)
    };
    let from = pos - dir * trail_len;
    let alpha = config::VISUAL_TRAIL_ALPHA * clamp01(speed_frac * 1.2 + 0.2);
    let base = Color::new(color.r * 0.8, color.g * 0.85, color.b * 0.95, alpha);
    draw_line(from.x, from.y, pos.x, pos.y, 2.2, base);
    draw_line(
        from.x,
        from.y,
        pos.x,
        pos.y,
        1.1,
        Color::new(0.82, 0.9, 1.0, alpha * 0.65),
    );
}

fn draw_entity_shape(
    pos: Vec2,
    heading: f32,
    radius: f32,
    color: Color,
    energy: f32,
    speed: f32,
    tick_count: u64,
    visual_settings: VisualSettings,
    show_vitals: bool,
) {
    let dir = Vec2::from_angle(heading);
    let perp = Vec2::new(-dir.y, dir.x);
    let speed_frac = (speed / config::ENTITY_MAX_SPEED).clamp(0.0, 1.0);
    let morphology = ((color.r * 1.7 + color.g * 2.9 + color.b * 3.7) * 10.0).fract();
    let vitality = (energy / crate::config::MAX_ENTITY_ENERGY).clamp(0.0, 1.0);
    let body_len = radius * (1.35 + morphology * 0.45);
    let body_width = radius * (0.78 + (1.0 - morphology) * 0.35);
    let wobble_phase =
        tick_count as f32 * (0.08 + speed_frac * 0.12) + pos.x * 0.017 + pos.y * 0.011;
    let sway = if visual_settings.creature_detail_enabled {
        wobble_phase.sin() * radius * 0.10 * speed_frac
    } else {
        0.0
    };
    let body_pos = pos + perp * sway;

    draw_circle(
        body_pos.x + 1.2,
        body_pos.y + radius * 0.28,
        radius * (1.0 + speed_frac * 0.15),
        Color::new(0.0, 0.0, 0.0, 0.22),
    );

    let nose = body_pos + dir * body_len;
    let tail = body_pos - dir * radius * 1.05;
    let flank_left = body_pos - dir * radius * 0.25 + perp * body_width;
    let flank_right = body_pos - dir * radius * 0.25 - perp * body_width;
    let under_left = body_pos + dir * radius * 0.1 + perp * body_width * 0.66;
    let under_right = body_pos + dir * radius * 0.1 - perp * body_width * 0.66;

    let dorsal = color;
    let belly = Color::new(color.r * 0.68, color.g * 0.68, color.b * 0.7, 1.0);
    let rim = Color::new(
        (color.r * 1.2).clamp(0.0, 1.0),
        (color.g * 1.2).clamp(0.0, 1.0),
        (color.b * 1.2).clamp(0.0, 1.0),
        0.5,
    );
    draw_triangle(nose, flank_left, flank_right, dorsal);
    draw_triangle(tail, flank_left, flank_right, belly);
    draw_triangle(nose - dir * radius * 0.45, under_left, under_right, belly);
    draw_line(
        flank_left.x,
        flank_left.y,
        nose.x,
        nose.y,
        1.0,
        Color::new(rim.r, rim.g, rim.b, 0.45),
    );
    draw_line(
        flank_right.x,
        flank_right.y,
        nose.x,
        nose.y,
        1.0,
        Color::new(rim.r, rim.g, rim.b, 0.45),
    );

    let core = Color::new(color.r * 0.88, color.g * 0.88, color.b * 0.9, 0.95);
    draw_circle(body_pos.x, body_pos.y, radius * 0.54, core);
    draw_circle(
        nose.x,
        nose.y,
        radius * (0.20 + morphology * 0.12),
        Color::new(color.r * 0.85, color.g * 0.9, color.b, 0.92),
    );
    draw_circle_lines(
        body_pos.x,
        body_pos.y,
        radius * 0.62,
        1.0,
        Color::new(rim.r, rim.g, rim.b, 0.36),
    );

    let eye_offset = radius * 0.35;
    let eye_pos = body_pos + dir * radius * 0.55;
    let eye_l = eye_pos + perp * eye_offset;
    let eye_r = eye_pos - perp * eye_offset;
    let pupil_offset = dir * radius * 0.03;
    draw_circle(
        eye_l.x,
        eye_l.y,
        radius * 0.12,
        Color::new(0.9, 0.95, 1.0, 0.9),
    );
    draw_circle(
        eye_r.x,
        eye_r.y,
        radius * 0.12,
        Color::new(0.9, 0.95, 1.0, 0.9),
    );
    if visual_settings.creature_detail_enabled {
        for stripe in 0..3 {
            let t = stripe as f32 / 2.0;
            let center = body_pos - dir * radius * (0.48 - t * 0.46);
            let span = body_width * (0.9 - t * 0.25);
            draw_line(
                center.x - perp.x * span,
                center.y - perp.y * span,
                center.x + perp.x * span,
                center.y + perp.y * span,
                0.9,
                Color::new(0.95, 0.98, 1.0, 0.16),
            );
        }

        draw_circle(
            eye_l.x + pupil_offset.x,
            eye_l.y + pupil_offset.y,
            radius * 0.06,
            Color::new(0.1, 0.1, 0.12, 0.95),
        );
        draw_circle(
            eye_r.x + pupil_offset.x,
            eye_r.y + pupil_offset.y,
            radius * 0.06,
            Color::new(0.1, 0.1, 0.12, 0.95),
        );
        let fin_phase = (wobble_phase * 1.6).sin();
        let fin_sweep = radius * (0.35 + 0.15 * fin_phase.abs());
        let fin_back = body_pos - dir * radius * 0.15;
        let fin_l = fin_back + perp * (body_width * 0.85 + fin_sweep * 0.15);
        let fin_r = fin_back - perp * (body_width * 0.85 + fin_sweep * 0.15);
        draw_triangle(
            fin_back + dir * radius * 0.45,
            fin_l,
            fin_back,
            Color::new(color.r * 0.7, color.g * 0.75, color.b * 0.8, 0.52),
        );
        draw_triangle(
            fin_back + dir * radius * 0.45,
            fin_r,
            fin_back,
            Color::new(color.r * 0.7, color.g * 0.75, color.b * 0.8, 0.52),
        );
    }

    draw_circle_lines(
        body_pos.x,
        body_pos.y,
        radius * (1.08 + speed_frac * 0.18),
        1.0,
        Color::new(0.88, 0.94, 1.0, 0.12 + vitality * 0.18),
    );

    if show_vitals {
        let bar_width = radius * 2.0;
        let bar_y = body_pos.y - radius * 2.0;
        let energy_frac = (energy / crate::config::MAX_ENTITY_ENERGY).clamp(0.0, 1.0);
        let bar_color = if energy_frac > 0.5 {
            Color::new(0.2, 0.9, 0.2, 0.7)
        } else if energy_frac > 0.25 {
            Color::new(0.9, 0.9, 0.2, 0.7)
        } else {
            Color::new(0.9, 0.2, 0.2, 0.7)
        };

        draw_line(
            body_pos.x - bar_width * 0.5,
            bar_y,
            body_pos.x + bar_width * 0.5,
            bar_y,
            2.0,
            Color::new(0.15, 0.15, 0.15, 0.5),
        );
        draw_line(
            body_pos.x - bar_width * 0.5,
            bar_y,
            body_pos.x - bar_width * 0.5 + bar_width * energy_frac,
            bar_y,
            2.0,
            bar_color,
        );
    }
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
                    HitType::Hazard => Color::new(1.0, 0.2, 0.3, 0.45),
                };
                draw_line(start.x, start.y, end.x, end.y, 1.0, color);
            }
        }
    }
}

fn season_atmosphere_tint(season: environment::Season) -> Color {
    match season {
        environment::Season::Spring => Color::new(0.12, 0.22, 0.14, 1.0),
        environment::Season::Summer => Color::new(0.14, 0.20, 0.10, 1.0),
        environment::Season::Autumn => Color::new(0.22, 0.16, 0.10, 1.0),
        environment::Season::Winter => Color::new(0.10, 0.16, 0.24, 1.0),
    }
}

fn draw_atmosphere_overlay(
    world: &World,
    season: environment::Season,
    day_brightness: f32,
    storm_active: bool,
    tick_count: u64,
    visual_settings: VisualSettings,
) {
    if !visual_settings.atmosphere_enabled {
        return;
    }

    let tint = season_atmosphere_tint(season);
    let darkness = clamp01(1.0 - day_brightness);
    let strength = clamp01(visual_settings.atmosphere_strength() * (0.4 + darkness));
    if strength <= 0.001 {
        return;
    }

    // Broad depth cue: darken lower half slightly more to fake aerial perspective.
    let depth_alpha = (0.08 + darkness * 0.12) * strength;
    draw_rectangle(
        0.0,
        world.height * 0.35,
        world.width,
        world.height * 0.65,
        Color::new(0.03, 0.05, 0.08, clamp01(depth_alpha)),
    );

    draw_rectangle(
        0.0,
        0.0,
        world.width,
        world.height,
        Color::new(tint.r, tint.g, tint.b, strength),
    );

    // Add broad drifting haze bands.
    let band_count = match visual_settings.quality {
        crate::visual::VisualQuality::Low => 3,
        crate::visual::VisualQuality::Medium => 4,
        crate::visual::VisualQuality::High => 5,
        crate::visual::VisualQuality::Ultra => 7,
    };
    let tick = tick_count as f32 * 0.003;
    for i in 0..band_count {
        let phase = tick + i as f32 * 1.37;
        let y = (phase.sin() * 0.5 + 0.5) * world.height;
        let band_h = world.height * 0.14;
        let a =
            config::VISUAL_ATMOSPHERE_NOISE_ALPHA * (0.35 + 0.65 * phase.cos().abs()) * strength;
        draw_rectangle(
            0.0,
            y - band_h * 0.5,
            world.width,
            band_h,
            Color::new(tint.r * 0.9, tint.g * 0.95, tint.b, clamp01(a)),
        );
    }

    if storm_active {
        draw_rectangle(
            0.0,
            0.0,
            world.width,
            world.height,
            Color::new(0.20, 0.24, 0.32, clamp01(strength * 0.28)),
        );
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
    let tc = Color::new(0.74, 0.8, 0.88, 0.96);
    let sh = Color::new(0.0, 0.0, 0.0, 0.35);
    draw_rectangle(8.0, 8.0, 230.0, 104.0, Color::new(0.02, 0.04, 0.08, 0.5));
    draw_rectangle_lines(8.0, 8.0, 230.0, 104.0, 1.0, Color::new(0.16, 0.24, 0.36, 0.5));

    let fps_text = format!("FPS: {}", get_fps());
    draw_text(&fps_text, 17.0, 29.0, 18.0, sh);
    draw_text(&fps_text, 16.0, 28.0, 18.0, tc);

    let ent_text = format!("Entities: {}", arena.count);
    draw_text(&ent_text, 17.0, 49.0, 18.0, sh);
    draw_text(&ent_text, 16.0, 48.0, 18.0, tc);

    let food_text = format!("Food: {}", food_count);
    draw_text(&food_text, 17.0, 69.0, 18.0, sh);
    draw_text(&food_text, 16.0, 68.0, 18.0, tc);

    let tick_text = format!("Tick: {}", tick_count);
    draw_text(&tick_text, 17.0, 89.0, 18.0, sh);
    draw_text(&tick_text, 16.0, 88.0, 18.0, tc);

    let day_str = if is_day { "Day" } else { "Night" };
    let env_text = format!(
        "{} | {} {}",
        season,
        day_str,
        if storm_active { "| STORM" } else { "" }
    );
    draw_text(&env_text, 17.0, 109.0, 18.0, sh);
    draw_text(&env_text, 16.0, 108.0, 18.0, tc);

    if paused {
        let pause_text = "PAUSED (Space to resume)";
        let tw = measure_text(pause_text, None, 24, 1.0).width;
        let x = screen_width() * 0.5 - tw * 0.5;
        draw_text(pause_text, x + 1.0, 31.0, 24.0, sh);
        draw_text(pause_text, x, 30.0, 24.0, Color::new(1.0, 0.8, 0.2, 0.9));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp01_bounds_alpha_values() {
        assert_eq!(clamp01(-0.5), 0.0);
        assert!((clamp01(0.6) - 0.6).abs() < 1e-6);
        assert_eq!(clamp01(2.0), 1.0);
    }

    #[test]
    fn season_tint_alpha_is_full() {
        for season in [
            environment::Season::Spring,
            environment::Season::Summer,
            environment::Season::Autumn,
            environment::Season::Winter,
        ] {
            let c = season_atmosphere_tint(season);
            assert!((c.a - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn wrapped_interpolation_clamps_alpha_and_stays_in_world() {
        let world = World::new(200.0, 200.0, true);
        let p = interpolate_wrapped(&world, vec2(190.0, 100.0), vec2(10.0, 100.0), 1.6);
        assert!(p.x >= 0.0 && p.x <= world.width);
        assert!(p.y >= 0.0 && p.y <= world.height);
        assert!((p.x - 10.0).abs() < 1e-4);
    }

    #[test]
    fn wrapped_interpolation_uses_shortest_toroidal_path() {
        let world = World::new(200.0, 200.0, true);
        let halfway = interpolate_wrapped(&world, vec2(190.0, 120.0), vec2(10.0, 120.0), 0.5);
        // Shortest toroidal path goes forward by +20, so midpoint should be near x=0 (wrapped to 0/200 edge).
        assert!(halfway.x <= 1.0 || halfway.x >= 199.0);
        assert!((halfway.y - 120.0).abs() < 1e-4);
    }
}
