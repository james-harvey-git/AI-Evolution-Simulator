use macroquad::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

use crate::config;
use crate::entity::EntityArena;
use crate::visual::VisualSettings;
use crate::world::World;

/// Terrain types with different properties.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerrainType {
    Plains,
    Forest,
    Desert,
    Water,
    Toxic,
}

impl TerrainType {
    /// Movement speed multiplier on this terrain.
    pub fn friction_mult(&self) -> f32 {
        match self {
            TerrainType::Plains => 1.0,
            TerrainType::Forest => 0.6,
            TerrainType::Desert => 0.9,
            TerrainType::Water => 0.2,
            TerrainType::Toxic => 0.8,
        }
    }

    /// Food spawn rate multiplier on this terrain.
    pub fn food_spawn_mult(&self) -> f32 {
        match self {
            TerrainType::Plains => 1.0,
            TerrainType::Forest => 2.0,
            TerrainType::Desert => 0.3,
            TerrainType::Water => 0.0,
            TerrainType::Toxic => 0.0,
        }
    }

    /// Energy drain per second on this terrain.
    pub fn damage_per_sec(&self) -> f32 {
        match self {
            TerrainType::Toxic => 3.0,
            _ => 0.0,
        }
    }

    /// Render color for this terrain.
    pub fn color(&self) -> Color {
        match self {
            TerrainType::Plains => Color::new(0.08, 0.20, 0.12, 1.0),
            TerrainType::Forest => Color::new(0.05, 0.24, 0.11, 1.0),
            TerrainType::Desert => Color::new(0.22, 0.18, 0.09, 1.0),
            TerrainType::Water => Color::new(0.05, 0.15, 0.26, 1.0),
            TerrainType::Toxic => Color::new(0.24, 0.08, 0.20, 1.0),
        }
    }
}

/// Terrain grid covering the world.
pub struct TerrainGrid {
    pub cells: Vec<TerrainType>,
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    inv_cell_size: f32,
}

impl TerrainGrid {
    pub fn generate(world_w: f32, world_h: f32, cell_size: f32, seed: u32) -> Self {
        let width = (world_w / cell_size).ceil() as usize;
        let height = (world_h / cell_size).ceil() as usize;

        let fbm: Fbm<Perlin> = Fbm::new(seed);
        let mut cells = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * 4.0;
                let ny = y as f64 / height as f64 * 4.0;
                let val = fbm.get([nx, ny]) as f32;

                let terrain = match val {
                    v if v < -0.45 => TerrainType::Water,
                    v if v < -0.1 => TerrainType::Forest,
                    v if v < 0.3 => TerrainType::Plains,
                    v if v < 0.55 => TerrainType::Desert,
                    _ => TerrainType::Toxic,
                };
                cells.push(terrain);
            }
        }

        Self {
            cells,
            width,
            height,
            cell_size,
            inv_cell_size: 1.0 / cell_size,
        }
    }

    pub fn get_at(&self, pos: Vec2) -> TerrainType {
        let cx = ((pos.x * self.inv_cell_size) as usize).min(self.width.saturating_sub(1));
        let cy = ((pos.y * self.inv_cell_size) as usize).min(self.height.saturating_sub(1));
        self.cells[cy * self.width + cx]
    }
}

/// Season cycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn food_multiplier(&self) -> f32 {
        match self {
            Season::Spring => 1.2,
            Season::Summer => 1.0,
            Season::Autumn => 0.8,
            Season::Winter => 0.5,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Autumn => "Autumn",
            Season::Winter => "Winter",
        }
    }
}

/// Storm event.
#[derive(Clone, Debug)]
pub struct Storm {
    pub center: Vec2,
    pub radius: f32,
    pub velocity: Vec2,
    pub timer: f32,
}

/// User-placed wall segment that blocks movement and line of sight.
#[derive(Clone, Debug)]
pub struct WallSegment {
    pub start: Vec2,
    pub end: Vec2,
}

/// Temporary toxic hazard zone.
#[derive(Clone, Debug)]
pub struct ToxicZone {
    pub center: Vec2,
    pub radius: f32,
    pub timer: f32,
}

/// Full environment state.
pub struct EnvironmentState {
    pub terrain: TerrainGrid,
    pub time_of_day: f32,  // [0, 1) where 0.5 = noon
    pub day_progress: f32, // total time in current cycle
    pub season: Season,
    pub season_progress: f32,
    pub storm: Option<Storm>,
    pub storm_cooldown: f32,
    pub walls: Vec<WallSegment>,
    pub toxic_zones: Vec<ToxicZone>,
}

impl EnvironmentState {
    pub fn new(world_w: f32, world_h: f32, seed: u32) -> Self {
        Self {
            terrain: TerrainGrid::generate(world_w, world_h, 50.0, seed),
            time_of_day: 0.25, // start at dawn
            day_progress: 0.0,
            season: Season::Spring,
            season_progress: 0.0,
            storm: None,
            storm_cooldown: config::STORM_INTERVAL_MIN,
            walls: Vec::new(),
            toxic_zones: Vec::new(),
        }
    }

    pub fn tick(&mut self, dt: f32, world: &World, rng: &mut impl ::rand::Rng) {
        // Day/night cycle
        self.day_progress += dt;
        self.time_of_day = (self.day_progress / config::DAY_LENGTH).fract();

        // Season cycle
        self.season_progress += dt / config::SEASON_LENGTH;
        if self.season_progress >= 1.0 {
            self.season_progress -= 1.0;
            self.season = match self.season {
                Season::Spring => Season::Summer,
                Season::Summer => Season::Autumn,
                Season::Autumn => Season::Winter,
                Season::Winter => Season::Spring,
            };
        }

        // Storm management
        if let Some(ref mut storm) = self.storm {
            storm.timer -= dt;
            storm.center += storm.velocity * dt;
            // Wrap storm center
            storm.center = world.wrap(storm.center);
            if storm.timer <= 0.0 {
                self.storm = None;
                self.storm_cooldown =
                    rng.gen_range(config::STORM_INTERVAL_MIN..config::STORM_INTERVAL_MAX);
            }
        } else {
            self.storm_cooldown -= dt;
            if self.storm_cooldown <= 0.0 {
                self.storm = Some(Storm {
                    center: vec2(
                        rng.gen_range(0.0..world.width),
                        rng.gen_range(0.0..world.height),
                    ),
                    radius: config::STORM_RADIUS,
                    velocity: Vec2::from_angle(rng.gen_range(0.0..std::f32::consts::TAU)) * 30.0,
                    timer: config::STORM_DURATION,
                });
            }
        }

        for zone in &mut self.toxic_zones {
            zone.timer -= dt;
        }
        self.toxic_zones.retain(|zone| zone.timer > 0.0);
    }

    /// Is it daytime? (roughly 6am to 6pm)
    pub fn is_day(&self) -> bool {
        self.time_of_day > 0.25 && self.time_of_day < 0.75
    }

    /// Day brightness factor [0.3, 1.0].
    pub fn day_brightness(&self) -> f32 {
        // Smooth sine curve: bright at noon, dim at midnight
        let phase = (self.time_of_day - 0.25) * std::f32::consts::TAU;
        let raw = (phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        0.3 + raw * 0.7
    }

    /// Food spawn multiplier considering season + time of day.
    pub fn food_rate_multiplier(&self) -> f32 {
        let season_mult = self.season.food_multiplier();
        let day_mult = if self.is_day() { 1.5 } else { 0.0 };
        season_mult * day_mult
    }

    pub fn add_toxic_zone(&mut self, center: Vec2, radius: f32, duration: f32) {
        self.toxic_zones.push(ToxicZone {
            center,
            radius,
            timer: duration,
        });
    }

    pub fn add_wall(&mut self, start: Vec2, end: Vec2) {
        if start.distance_squared(end) < 25.0 {
            return;
        }
        self.walls.push(WallSegment { start, end });
    }
}

/// Apply terrain effects to entities (damage from toxic, push from water).
pub fn apply_terrain_effects(
    arena: &mut EntityArena,
    terrain: &TerrainGrid,
    _world: &World,
    dt: f32,
) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            if !entity.alive {
                continue;
            }
            let t = terrain.get_at(entity.pos);
            let damage = t.damage_per_sec() * dt;
            if damage > 0.0 {
                entity.energy -= damage;
                entity.health -= damage;
            }

            // Push entities out of water
            if t == TerrainType::Water {
                // Slow them down heavily and drain energy
                entity.velocity *= 0.9;
                entity.energy -= 1.0 * dt;
            }
        }
    }
}

/// Apply storm effects to entities within the storm radius.
/// Entities can receive shelter from both forest terrain and nearby walls.
pub fn apply_storm_effects(
    arena: &mut EntityArena,
    storm: &Storm,
    world: &World,
    terrain: &TerrainGrid,
    walls: &[WallSegment],
    dt: f32,
) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            if !entity.alive {
                continue;
            }
            let dist_sq = world.distance_sq(entity.pos, storm.center);
            if dist_sq < storm.radius * storm.radius {
                let shelter_mult =
                    combined_storm_shelter_multiplier(entity.pos, terrain, walls, world);

                // Storm damage
                entity.energy -= config::STORM_DAMAGE * shelter_mult * dt;
                // Wind push
                let push_dir = world.delta(storm.center, entity.pos);
                if push_dir.length_squared() > 0.001 {
                    entity.velocity += push_dir.normalize() * 20.0 * shelter_mult * dt;
                }
            }
        }
    }
}

/// Return distance from `pos` to nearest wall segment, if any walls exist.
pub fn nearest_wall_distance(pos: Vec2, walls: &[WallSegment], world: &World) -> Option<f32> {
    let mut best_sq = f32::INFINITY;
    for wall in walls {
        let cp = closest_point_on_segment(wall.start, wall.end, pos);
        let dist_sq = world.distance_sq(pos, cp);
        if dist_sq < best_sq {
            best_sq = dist_sq;
        }
    }

    if best_sq.is_finite() {
        Some(best_sq.sqrt())
    } else {
        None
    }
}

/// Wall-based storm shelter multiplier with linear distance falloff.
/// 1.0 means no shelter (full storm effect), lower values mean stronger shelter.
pub fn wall_shelter_multiplier(pos: Vec2, walls: &[WallSegment], world: &World) -> f32 {
    let Some(distance) = nearest_wall_distance(pos, walls, world) else {
        return 1.0;
    };

    let range = config::STORM_WALL_SHELTER_RANGE.max(1.0);
    let t = (distance / range).clamp(0.0, 1.0);
    config::STORM_WALL_SHELTER_MIN_MULT + t * (1.0 - config::STORM_WALL_SHELTER_MIN_MULT)
}

/// Combined shelter multiplier from terrain + wall proximity.
pub fn combined_storm_shelter_multiplier(
    pos: Vec2,
    terrain: &TerrainGrid,
    walls: &[WallSegment],
    world: &World,
) -> f32 {
    let forest_mult = if terrain.get_at(pos) == TerrainType::Forest {
        config::STORM_FOREST_SHELTER_MULT
    } else {
        1.0
    };
    let wall_mult = wall_shelter_multiplier(pos, walls, world);

    (forest_mult * wall_mult).clamp(config::STORM_COMBINED_SHELTER_MIN_MULT, 1.0)
}

pub fn apply_toxic_zone_effects(
    arena: &mut EntityArena,
    zones: &[ToxicZone],
    world: &World,
    dt: f32,
) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            if !entity.alive {
                continue;
            }

            let mut zone_damage = 0.0;
            for zone in zones {
                let dist_sq = world.distance_sq(entity.pos, zone.center);
                if dist_sq < zone.radius * zone.radius {
                    zone_damage += config::TOXIC_ZONE_DAMAGE * dt;
                }
            }

            if zone_damage > 0.0 {
                entity.energy -= zone_damage;
                entity.health -= zone_damage;
            }
        }
    }
}

/// Returns closest point on segment [a,b] to p.
pub fn closest_point_on_segment(a: Vec2, b: Vec2, p: Vec2) -> Vec2 {
    let ab = b - a;
    let ab_len_sq = ab.length_squared();
    if ab_len_sq <= f32::EPSILON {
        return a;
    }
    let t = ((p - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    a + ab * t
}

pub fn point_near_any_wall(pos: Vec2, walls: &[WallSegment], world: &World, radius: f32) -> bool {
    let radius_sq = radius * radius;
    for wall in walls {
        let cp = closest_point_on_segment(wall.start, wall.end, pos);
        if world.distance_sq(pos, cp) <= radius_sq {
            return true;
        }
    }
    false
}

#[derive(Clone, Debug)]
struct StormWindSegment {
    start: Vec2,
    end: Vec2,
    alpha: f32,
}

fn mix_hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

fn hash4(a: u32, b: u32, c: u32, d: u32) -> u32 {
    let mut h = 0x9e37_79b9u32;
    h ^= mix_hash(a.wrapping_add(0x85eb_ca6b));
    h = h.rotate_left(13);
    h ^= mix_hash(b.wrapping_add(0xc2b2_ae35));
    h = h.rotate_left(11);
    h ^= mix_hash(c.wrapping_add(0x27d4_eb2d));
    h = h.rotate_left(7);
    h ^ mix_hash(d.wrapping_add(0x1656_67b1))
}

fn hash01(value: u32) -> f32 {
    value as f32 / u32::MAX as f32
}

fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn shade_color(c: Color, scale: f32) -> Color {
    Color::new(
        (c.r * scale).clamp(0.0, 1.0),
        (c.g * scale).clamp(0.0, 1.0),
        (c.b * scale).clamp(0.0, 1.0),
        c.a,
    )
}

fn macro_mask(x: usize, y: usize, tick_bucket: u32, bucket: u32) -> f32 {
    let bx = (x as u32) / bucket.max(1);
    let by = (y as u32) / bucket.max(1);
    hash01(hash4(bx, by, tick_bucket / 4, 0xa741_2c9d))
}

fn terrain_shaded_color(
    kind: TerrainType,
    x: usize,
    y: usize,
    tick_bucket: u32,
    detail: bool,
) -> Color {
    let base = kind.color();
    let micro = hash01(hash4(x as u32, y as u32, tick_bucket, 0x54f3_a19d)) - 0.5;
    let large_a = macro_mask(x, y, tick_bucket, 5) - 0.5;
    let large_b = macro_mask(x, y, tick_bucket.wrapping_add(17), 9) - 0.5;
    let large_c = macro_mask(x, y, tick_bucket.wrapping_add(43), 15) - 0.5;
    let strength = if detail { 0.30 } else { 0.16 };
    let macro_mix = large_a * 0.9 + large_b * 0.6;
    let mut shaded = shade_color(
        base,
        1.0 + micro * strength + macro_mix * 0.18 + large_c * if detail { 0.08 } else { 0.04 },
    );

    // Subtle biome tinting to avoid flat tile look.
    let tint = match kind {
        TerrainType::Plains => Color::new(0.16, 0.30, 0.17, 1.0),
        TerrainType::Forest => Color::new(0.08, 0.34, 0.15, 1.0),
        TerrainType::Desert => Color::new(0.33, 0.27, 0.13, 1.0),
        TerrainType::Water => Color::new(0.10, 0.22, 0.36, 1.0),
        TerrainType::Toxic => Color::new(0.32, 0.12, 0.27, 1.0),
    };
    let tint_strength = if detail { 0.24 } else { 0.12 };
    let bias = ((macro_mix + large_c) * 0.5 + 0.5).clamp(0.0, 1.0);
    shaded = lerp_color(shaded, tint, tint_strength * bias);
    shaded
}

/// Draw terrain with deterministic per-cell shading and lightweight boundary blending.
pub fn draw_terrain(terrain: &TerrainGrid, visual_settings: VisualSettings, tick_count: u64) {
    let detail = visual_settings.creature_detail_enabled;
    let tick_bucket = (tick_count / 180) as u32;

    for y in 0..terrain.height {
        for x in 0..terrain.width {
            let idx = y * terrain.width + x;
            let kind = terrain.cells[idx];
            let color = terrain_shaded_color(kind, x, y, tick_bucket, detail);
            let x0 = x as f32 * terrain.cell_size;
            let y0 = y as f32 * terrain.cell_size;

            draw_rectangle(x0, y0, terrain.cell_size, terrain.cell_size, color);
            let y_frac = y as f32 / terrain.height.max(1) as f32;
            let haze_alpha = ((0.5 - (y_frac - 0.5).abs()) * 0.16).clamp(0.0, 0.1);
            if haze_alpha > 0.0 {
                draw_rectangle(
                    x0,
                    y0,
                    terrain.cell_size,
                    terrain.cell_size,
                    Color::new(0.13, 0.2, 0.24, haze_alpha),
                );
            }

            if detail {
                if x + 1 < terrain.width {
                    let right_kind = terrain.cells[idx + 1];
                    if right_kind != kind {
                        let right_color =
                            terrain_shaded_color(right_kind, x + 1, y, tick_bucket, true);
                        let blend = lerp_color(color, right_color, 0.5);
                        draw_rectangle(
                            x0 + terrain.cell_size * 0.86,
                            y0,
                            terrain.cell_size * 0.18,
                            terrain.cell_size,
                            Color::new(blend.r, blend.g, blend.b, 0.22),
                        );
                    }
                }
                if y + 1 < terrain.height {
                    let down_kind = terrain.cells[idx + terrain.width];
                    if down_kind != kind {
                        let down_color =
                            terrain_shaded_color(down_kind, x, y + 1, tick_bucket, true);
                        let blend = lerp_color(color, down_color, 0.5);
                        draw_rectangle(
                            x0,
                            y0 + terrain.cell_size * 0.86,
                            terrain.cell_size,
                            terrain.cell_size * 0.18,
                            Color::new(blend.r, blend.g, blend.b, 0.18),
                        );
                    }
                }
            }
        }
    }
}

fn storm_seed(storm: &Storm) -> u32 {
    mix_hash(storm.center.x.to_bits())
        ^ mix_hash(storm.center.y.to_bits())
        ^ mix_hash(storm.radius.to_bits())
        ^ mix_hash(storm.velocity.x.to_bits())
        ^ mix_hash(storm.velocity.y.to_bits())
}

fn storm_tick_bucket(tick_count: u64) -> u32 {
    (tick_count / config::VISUAL_STORM_TICK_QUANTIZATION.max(1)) as u32
}

fn generate_storm_wind_segments(
    storm: &Storm,
    tick_count: u64,
    visual_settings: VisualSettings,
) -> Vec<StormWindSegment> {
    if !visual_settings.storm_fx_enabled {
        return Vec::new();
    }

    let cap = visual_settings.storm_line_cap();
    if cap == 0 {
        return Vec::new();
    }

    let cell_size = config::VISUAL_STORM_CELL_SIZE.max(8.0);
    let seed = storm_seed(storm);
    let tick_bucket = storm_tick_bucket(tick_count);
    let wind_dir = if storm.velocity.length_squared() > 0.001 {
        storm.velocity.normalize()
    } else {
        vec2(1.0, 0.0)
    };

    let min_cx = ((storm.center.x - storm.radius) / cell_size).floor() as i32;
    let max_cx = ((storm.center.x + storm.radius) / cell_size).ceil() as i32;
    let min_cy = ((storm.center.y - storm.radius) / cell_size).floor() as i32;
    let max_cy = ((storm.center.y + storm.radius) / cell_size).ceil() as i32;

    let mut candidates: Vec<(u32, i32, i32)> = Vec::new();
    for cy in min_cy..=max_cy {
        for cx in min_cx..=max_cx {
            let cell_center = vec2((cx as f32 + 0.5) * cell_size, (cy as f32 + 0.5) * cell_size);
            if cell_center.distance_squared(storm.center) <= storm.radius * storm.radius {
                let h = hash4(cx as u32, cy as u32, tick_bucket, seed);
                candidates.push((h, cx, cy));
            }
        }
    }

    candidates.sort_by_key(|(h, _, _)| *h);

    let mut lines = Vec::with_capacity(cap);
    for (h, cx, cy) in candidates {
        if lines.len() >= cap {
            break;
        }
        let hx = hash4(h, 0, seed, tick_bucket);
        let hy = hash4(h, 1, seed, tick_bucket);
        let hj = hash4(h, 2, seed, tick_bucket);
        let hl = hash4(h, 3, seed, tick_bucket);

        let cell_center = vec2((cx as f32 + 0.5) * cell_size, (cy as f32 + 0.5) * cell_size);
        let jitter = vec2(
            (hash01(hx) - 0.5) * cell_size * 0.9,
            (hash01(hy) - 0.5) * cell_size * 0.9,
        );
        let start = cell_center + jitter;
        let dist = start.distance(storm.center);
        if dist > storm.radius {
            continue;
        }

        let dist_frac = (dist / storm.radius.max(1.0)).clamp(0.0, 1.0);
        let jitter_angle = (hash01(hj) - 0.5) * 0.9;
        let dir = rotate(wind_dir, jitter_angle).normalize_or_zero();
        let len = config::VISUAL_STORM_LINE_BASE_LENGTH
            + config::VISUAL_STORM_LINE_LENGTH_JITTER * hash01(hl);
        let end = start + dir * len;
        let alpha = (config::VISUAL_STORM_LINE_ALPHA
            * (1.0 - dist_frac).powf(0.55)
            * (0.6 + 0.4 * hash01(hash4(h, 4, seed, tick_bucket))))
        .clamp(0.0, 1.0);
        lines.push(StormWindSegment { start, end, alpha });
    }

    lines
}

fn shelter_highlight_alpha(norm_dist: f32, storm_factor: f32) -> f32 {
    let d = norm_dist.clamp(0.0, 1.0);
    let sf = storm_factor.clamp(0.0, 1.0);
    (config::VISUAL_SHELTER_HIGHLIGHT_ALPHA * (1.0 - d).powf(1.25) * sf).clamp(0.0, 1.0)
}

/// Draw shelter range highlights around walls when storms are active.
pub fn draw_storm_shelter_highlights(
    storm: &Storm,
    walls: &[WallSegment],
    world: &World,
    visual_settings: VisualSettings,
) {
    if !visual_settings.shelter_highlight_enabled || walls.is_empty() {
        return;
    }

    let range = config::STORM_WALL_SHELTER_RANGE.max(1.0);
    let bands = visual_settings.shelter_band_count().max(1);

    for wall in walls {
        let cp = closest_point_on_segment(wall.start, wall.end, storm.center);
        let dist_to_storm = world.distance_sq(cp, storm.center).sqrt();
        let storm_factor = if dist_to_storm > storm.radius + range {
            0.0
        } else if dist_to_storm <= storm.radius {
            1.0
        } else {
            1.0 - (dist_to_storm - storm.radius) / range
        };
        if storm_factor <= 0.0 {
            continue;
        }

        for band in 0..bands {
            let t = band as f32 / bands as f32;
            let width = config::WALL_THICKNESS + range * (1.0 - t);
            let alpha = shelter_highlight_alpha(t, storm_factor);
            let color = Color::new(0.32, 0.56, 0.68, alpha);
            draw_line(
                wall.start.x,
                wall.start.y,
                wall.end.x,
                wall.end.y,
                width,
                color,
            );
        }

        draw_line(
            wall.start.x,
            wall.start.y,
            wall.end.x,
            wall.end.y,
            1.5,
            Color::new(0.80, 0.92, 1.0, 0.55 * storm_factor.clamp(0.0, 1.0)),
        );
    }
}

fn draw_storm_gust_arcs(
    storm: &Storm,
    tick_count: u64,
    visual_settings: VisualSettings,
    wind_dir: Vec2,
) {
    let seed = storm_seed(storm);
    let tick_bucket = storm_tick_bucket(tick_count);
    for i in 0..visual_settings.storm_gust_cap() {
        let h0 = hash4(i as u32, 11, tick_bucket, seed);
        let h1 = hash4(i as u32, 12, tick_bucket, seed);
        let h2 = hash4(i as u32, 13, tick_bucket, seed);
        let h3 = hash4(i as u32, 14, tick_bucket, seed);

        let angle = hash01(h0) * std::f32::consts::TAU;
        let radius_frac = 0.2 + 0.75 * hash01(h1);
        let center = storm.center + vec2(angle.cos(), angle.sin()) * storm.radius * radius_frac;
        let dist_frac = radius_frac.clamp(0.0, 1.0);
        let tangent = vec2(-angle.sin(), angle.cos());
        let gust_dir = (wind_dir * 0.75 + tangent * 0.25).normalize_or_zero();
        let len = 10.0 + 26.0 * hash01(h2);
        let curve = 3.0 + 7.0 * hash01(h3);
        let offset = vec2(-gust_dir.y, gust_dir.x) * curve;

        let p0 = center - gust_dir * len * 0.5 - offset * 0.35;
        let p1 = center + offset * 0.2;
        let p2 = center + gust_dir * len * 0.5 + offset * 0.35;
        let alpha = (config::VISUAL_STORM_GUST_ARC_ALPHA * (1.0 - dist_frac)).clamp(0.0, 1.0);
        let c = Color::new(0.76, 0.84, 1.0, alpha);
        draw_line(p0.x, p0.y, p1.x, p1.y, 1.2, c);
        draw_line(p1.x, p1.y, p2.x, p2.y, 1.2, c);
    }
}

/// Draw storm visual with directional wind streaks and gust arcs.
pub fn draw_storm(storm: &Storm, tick_count: u64, visual_settings: VisualSettings) {
    let wind_dir = if storm.velocity.length_squared() > 0.001 {
        storm.velocity.normalize()
    } else {
        vec2(1.0, 0.0)
    };

    for i in 0..4 {
        let t = i as f32 / 4.0;
        let r = storm.radius * (0.45 + t * 0.70);
        let alpha = 0.18 * (1.0 - t).powf(1.1);
        draw_circle(
            storm.center.x,
            storm.center.y,
            r,
            Color::new(0.32 + t * 0.08, 0.38 + t * 0.06, 0.52 + t * 0.1, alpha),
        );
    }

    if visual_settings.storm_fx_enabled {
        for segment in generate_storm_wind_segments(storm, tick_count, visual_settings) {
            draw_line(
                segment.start.x,
                segment.start.y,
                segment.end.x,
                segment.end.y,
                1.4,
                Color::new(0.8, 0.88, 1.0, segment.alpha),
            );
        }
        draw_storm_gust_arcs(storm, tick_count, visual_settings, wind_dir);
    }

    let marker_dir = wind_dir * (storm.radius * 0.2);
    draw_line(
        storm.center.x,
        storm.center.y,
        storm.center.x + marker_dir.x,
        storm.center.y + marker_dir.y,
        2.0,
        Color::new(0.9, 0.95, 1.0, 0.5),
    );
    draw_circle(
        storm.center.x,
        storm.center.y,
        7.0,
        Color::new(0.72, 0.8, 0.96, 0.55),
    );
}

pub fn draw_walls(walls: &[WallSegment]) {
    for wall in walls {
        draw_line(
            wall.start.x,
            wall.start.y,
            wall.end.x,
            wall.end.y,
            config::WALL_THICKNESS + 5.0,
            Color::new(0.12, 0.16, 0.22, 0.25),
        );
        draw_line(
            wall.start.x,
            wall.start.y,
            wall.end.x,
            wall.end.y,
            config::WALL_THICKNESS,
            Color::new(0.48, 0.58, 0.68, 0.88),
        );
        draw_line(
            wall.start.x,
            wall.start.y,
            wall.end.x,
            wall.end.y,
            1.5,
            Color::new(0.9, 0.96, 1.0, 0.82),
        );
    }
}

pub fn draw_toxic_zones(zones: &[ToxicZone]) {
    for zone in zones {
        let fade = (zone.timer / config::TOXIC_ZONE_DURATION).clamp(0.0, 1.0);
        draw_circle(
            zone.center.x,
            zone.center.y,
            zone.radius,
            Color::new(0.85, 0.15, 0.22, 0.10 * fade),
        );
        draw_circle_lines(
            zone.center.x,
            zone.center.y,
            zone.radius,
            2.0,
            Color::new(0.95, 0.2, 0.25, 0.35 * fade),
        );
    }
}

/// Draw day/night overlay tint (called after all world objects, before HUD).
pub fn draw_day_night_overlay(brightness: f32) {
    if brightness < 0.95 {
        let darkness = 1.0 - brightness;
        // Blue-tinted darkness overlay
        draw_rectangle(
            -10000.0,
            -10000.0,
            20000.0,
            20000.0,
            Color::new(0.0, 0.0, 0.15, darkness * 0.6),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;
    use crate::visual::{VisualQuality, VisualSettings};
    use ::rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn uniform_terrain(kind: TerrainType) -> TerrainGrid {
        TerrainGrid {
            cells: vec![kind],
            width: 1,
            height: 1,
            cell_size: 1000.0,
            inv_cell_size: 1.0 / 1000.0,
        }
    }

    fn test_entity(pos: Vec2) -> Entity {
        Entity {
            pos,
            prev_pos: pos,
            velocity: Vec2::ZERO,
            heading: 0.0,
            radius: 8.0,
            color: WHITE,
            energy: 100.0,
            carried_energy: 0.0,
            health: 100.0,
            max_health: 100.0,
            age: 0.0,
            alive: true,
            speed_multiplier: 1.0,
            sensor_range: 1.0,
            metabolic_rate: 1.0,
            generation_depth: 0,
            parent_id: None,
            offspring_count: 0,
            tick_born: 0,
        }
    }

    #[test]
    fn food_rate_multiplier_is_zero_at_night() {
        let mut env = EnvironmentState::new(200.0, 200.0, 1);
        env.season = Season::Summer;
        env.time_of_day = 0.9; // night
        assert_eq!(env.food_rate_multiplier(), 0.0);

        env.time_of_day = 0.5; // day
        assert!(env.food_rate_multiplier() > 0.0);
    }

    #[test]
    fn toxic_zones_expire_after_duration() {
        let world = World::new(200.0, 200.0, false);
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let mut env = EnvironmentState::new(200.0, 200.0, 2);
        env.add_toxic_zone(vec2(80.0, 80.0), 20.0, 0.1);

        assert_eq!(env.toxic_zones.len(), 1);
        env.tick(0.2, &world, &mut rng);
        assert!(env.toxic_zones.is_empty());
    }

    #[test]
    fn wall_shelter_is_one_without_walls() {
        let world = World::new(300.0, 300.0, false);
        let mult = wall_shelter_multiplier(vec2(100.0, 100.0), &[], &world);
        assert!((mult - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wall_shelter_is_stronger_when_near_wall() {
        let world = World::new(300.0, 300.0, false);
        let walls = vec![WallSegment {
            start: vec2(50.0, 50.0),
            end: vec2(250.0, 50.0),
        }];

        let near = wall_shelter_multiplier(vec2(120.0, 52.0), &walls, &world);
        let far = wall_shelter_multiplier(vec2(120.0, 200.0), &walls, &world);

        assert!(near < far);
    }

    #[test]
    fn combined_shelter_is_clamped_and_not_stronger_than_components() {
        let world = World::new(300.0, 300.0, false);
        let walls = vec![WallSegment {
            start: vec2(30.0, 40.0),
            end: vec2(230.0, 40.0),
        }];
        let terrain = uniform_terrain(TerrainType::Forest);
        let pos = vec2(120.0, 40.0);

        let wall_mult = wall_shelter_multiplier(pos, &walls, &world);
        let combined = combined_storm_shelter_multiplier(pos, &terrain, &walls, &world);

        assert!(combined <= wall_mult + 1e-6);
        assert!(combined <= config::STORM_FOREST_SHELTER_MULT + 1e-6);
        assert!(combined >= config::STORM_COMBINED_SHELTER_MIN_MULT - 1e-6);
    }

    #[test]
    fn storm_effects_are_reduced_near_wall() {
        let world = World::new(300.0, 300.0, false);
        let terrain = uniform_terrain(TerrainType::Plains);
        let walls = vec![WallSegment {
            start: vec2(40.0, 50.0),
            end: vec2(240.0, 50.0),
        }];

        let storm = Storm {
            center: vec2(150.0, 140.0),
            radius: 220.0,
            velocity: vec2(10.0, 0.0),
            timer: 10.0,
        };

        let mut arena = EntityArena::new(2);
        let near_id = arena.spawn(test_entity(vec2(150.0, 52.0))).unwrap();
        let far_id = arena.spawn(test_entity(vec2(150.0, 220.0))).unwrap();

        let near_energy_before = arena.get(near_id).unwrap().energy;
        let far_energy_before = arena.get(far_id).unwrap().energy;

        apply_storm_effects(&mut arena, &storm, &world, &terrain, &walls, 1.0);

        let near = arena.get(near_id).unwrap();
        let far = arena.get(far_id).unwrap();

        let near_loss = near_energy_before - near.energy;
        let far_loss = far_energy_before - far.energy;
        assert!(near_loss < far_loss);
        assert!(near.velocity.length() < far.velocity.length());
    }

    #[test]
    fn storm_wind_segments_are_deterministic() {
        let storm = Storm {
            center: vec2(120.0, 160.0),
            radius: 180.0,
            velocity: vec2(22.0, -9.0),
            timer: 10.0,
        };
        let settings = VisualSettings::with_quality(VisualQuality::High);

        let a = generate_storm_wind_segments(&storm, 512, settings);
        let b = generate_storm_wind_segments(&storm, 512, settings);
        assert_eq!(a.len(), b.len());

        for (la, lb) in a.iter().zip(b.iter()) {
            assert!(la.start.distance(lb.start) < 1e-6);
            assert!(la.end.distance(lb.end) < 1e-6);
            assert!((la.alpha - lb.alpha).abs() < 1e-6);
        }
    }

    #[test]
    fn storm_wind_segments_are_finite_and_capped() {
        let storm = Storm {
            center: vec2(100.0, 80.0),
            radius: 210.0,
            velocity: vec2(-12.0, 7.0),
            timer: 8.0,
        };

        for quality in VisualQuality::ALL {
            let settings = VisualSettings::with_quality(quality);
            let lines = generate_storm_wind_segments(&storm, 300, settings);
            assert!(lines.len() <= settings.storm_line_cap());
            for line in lines {
                assert!(line.start.x.is_finite());
                assert!(line.start.y.is_finite());
                assert!(line.end.x.is_finite());
                assert!(line.end.y.is_finite());
                assert!(line.alpha.is_finite());
                assert!(line.alpha >= 0.0 && line.alpha <= 1.0);
            }
        }
    }

    #[test]
    fn shelter_highlight_alpha_clamps_to_valid_range() {
        let a = shelter_highlight_alpha(-5.0, 2.0);
        let b = shelter_highlight_alpha(10.0, -3.0);
        assert!(a >= 0.0 && a <= 1.0);
        assert!(b >= 0.0 && b <= 1.0);
        assert!(a > b);
    }
}
