use macroquad::prelude::*;
use noise::{NoiseFn, Fbm, Perlin};

use crate::config;
use crate::entity::EntityArena;
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
            TerrainType::Plains => Color::new(0.04, 0.06, 0.03, 1.0),
            TerrainType::Forest => Color::new(0.02, 0.08, 0.03, 1.0),
            TerrainType::Desert => Color::new(0.08, 0.06, 0.02, 1.0),
            TerrainType::Water => Color::new(0.02, 0.04, 0.10, 1.0),
            TerrainType::Toxic => Color::new(0.08, 0.02, 0.06, 1.0),
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

/// Full environment state.
pub struct EnvironmentState {
    pub terrain: TerrainGrid,
    pub time_of_day: f32, // [0, 1) where 0.5 = noon
    pub day_progress: f32, // total time in current cycle
    pub season: Season,
    pub season_progress: f32,
    pub storm: Option<Storm>,
    pub storm_cooldown: f32,
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
                self.storm_cooldown = rng.gen_range(config::STORM_INTERVAL_MIN..config::STORM_INTERVAL_MAX);
            }
        } else {
            self.storm_cooldown -= dt;
            if self.storm_cooldown <= 0.0 {
                self.storm = Some(Storm {
                    center: vec2(rng.gen_range(0.0..world.width), rng.gen_range(0.0..world.height)),
                    radius: config::STORM_RADIUS,
                    velocity: Vec2::from_angle(rng.gen_range(0.0..std::f32::consts::TAU)) * 30.0,
                    timer: config::STORM_DURATION,
                });
            }
        }
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
        let day_mult = if self.is_day() { 1.5 } else { 0.5 };
        season_mult * day_mult
    }
}

/// Apply terrain effects to entities (damage from toxic, push from water).
pub fn apply_terrain_effects(arena: &mut EntityArena, terrain: &TerrainGrid, _world: &World, dt: f32) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
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
/// Entities on Forest terrain receive shelter (reduced damage and push).
pub fn apply_storm_effects(arena: &mut EntityArena, storm: &Storm, world: &World, terrain: &TerrainGrid, dt: f32) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            let dist_sq = world.distance_sq(entity.pos, storm.center);
            if dist_sq < storm.radius * storm.radius {
                // Shelter: forest terrain reduces storm damage by 70%
                let terrain_type = terrain.get_at(entity.pos);
                let shelter_mult = if terrain_type == TerrainType::Forest { 0.3 } else { 1.0 };

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

/// Draw terrain grid.
pub fn draw_terrain(terrain: &TerrainGrid) {
    for y in 0..terrain.height {
        for x in 0..terrain.width {
            let t = terrain.cells[y * terrain.width + x];
            let color = t.color();
            draw_rectangle(
                x as f32 * terrain.cell_size,
                y as f32 * terrain.cell_size,
                terrain.cell_size,
                terrain.cell_size,
                color,
            );
        }
    }
}

/// Draw storm visual.
pub fn draw_storm(storm: &Storm) {
    // Multiple concentric circles for the storm
    let alpha_base = 0.15;
    for i in 0..3 {
        let r = storm.radius * (0.5 + i as f32 * 0.25);
        let alpha = alpha_base * (1.0 - i as f32 * 0.3);
        draw_circle(
            storm.center.x,
            storm.center.y,
            r,
            Color::new(0.4, 0.4, 0.6, alpha),
        );
    }
    // Storm center marker
    draw_circle(
        storm.center.x,
        storm.center.y,
        8.0,
        Color::new(0.6, 0.6, 0.8, 0.4),
    );
}

/// Draw day/night overlay tint (called after all world objects, before HUD).
pub fn draw_day_night_overlay(brightness: f32) {
    if brightness < 0.95 {
        let darkness = 1.0 - brightness;
        // Blue-tinted darkness overlay
        draw_rectangle(
            -10000.0, -10000.0, 20000.0, 20000.0,
            Color::new(0.0, 0.0, 0.15, darkness * 0.6),
        );
    }
}
