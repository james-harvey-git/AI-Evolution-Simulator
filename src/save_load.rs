use macroquad::prelude::*;
use serde::{Serialize, Deserialize};

use crate::brain::BrainStorage;
use crate::combat::MeatItem;
use crate::config;
use crate::entity::{Entity, EntityArena, EntityId};
use crate::environment::{EnvironmentState, Season, Storm, TerrainType};
use crate::genome::{Genome, N};
use crate::particles::ParticleSystem;
use crate::signals::{PheromoneGrid, SignalState};
use crate::simulation::{FoodItem, SimState};

// Serde-friendly wrapper types for macroquad primitives

#[derive(Clone, Serialize, Deserialize)]
struct SerdVec2 {
    x: f32,
    y: f32,
}

impl From<Vec2> for SerdVec2 {
    fn from(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<SerdVec2> for Vec2 {
    fn from(v: SerdVec2) -> Self {
        vec2(v.x, v.y)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdColor {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl From<Color> for SerdColor {
    fn from(c: Color) -> Self {
        Self { r: c.r, g: c.g, b: c.b, a: c.a }
    }
}

impl From<SerdColor> for Color {
    fn from(c: SerdColor) -> Self {
        Color::new(c.r, c.g, c.b, c.a)
    }
}

// Serializable entity
#[derive(Serialize, Deserialize)]
struct SerdEntity {
    pos: SerdVec2,
    prev_pos: SerdVec2,
    velocity: SerdVec2,
    heading: f32,
    radius: f32,
    color: SerdColor,
    energy: f32,
    health: f32,
    max_health: f32,
    age: f32,
    speed_multiplier: f32,
    sensor_range: f32,
    metabolic_rate: f32,
    generation_depth: u32,
    parent_idx: Option<u32>,
    parent_gen: Option<u32>,
    offspring_count: u32,
    tick_born: u64,
}

#[derive(Serialize, Deserialize)]
struct SerdEntityId {
    index: u32,
    generation: u32,
}

// Serializable brain data for a single slot
#[derive(Serialize, Deserialize)]
struct SerdBrain {
    states: [f32; N],
    tau_inv: [f32; N],
    biases: [f32; N],
    weights: [[f32; N]; N],
    outputs: [f32; N],
}

#[derive(Serialize, Deserialize)]
struct SerdFood {
    pos: SerdVec2,
    energy: f32,
}

#[derive(Serialize, Deserialize)]
struct SerdMeat {
    pos: SerdVec2,
    energy: f32,
    decay_timer: f32,
}

#[derive(Serialize, Deserialize)]
struct SerdStorm {
    center: SerdVec2,
    radius: f32,
    velocity: SerdVec2,
    timer: f32,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdSeason(u8);

impl From<Season> for SerdSeason {
    fn from(s: Season) -> Self {
        SerdSeason(match s {
            Season::Spring => 0,
            Season::Summer => 1,
            Season::Autumn => 2,
            Season::Winter => 3,
        })
    }
}

impl From<SerdSeason> for Season {
    fn from(s: SerdSeason) -> Self {
        match s.0 {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        }
    }
}

/// Complete serializable save state.
#[derive(Serialize, Deserialize)]
pub struct SaveState {
    // Entity arena
    entities: Vec<Option<SerdEntity>>,
    generations: Vec<u32>,
    arena_count: usize,

    // Brains (only active slots)
    active_brain_slots: Vec<usize>,
    brains: Vec<SerdBrain>,

    // Genomes
    genomes: Vec<Option<Vec<f32>>>,

    // Food + meat
    food: Vec<SerdFood>,
    meat: Vec<SerdMeat>,

    // Pheromone grid
    pheromone_cells: Vec<f32>,

    // Environment
    time_of_day: f32,
    day_progress: f32,
    season: SerdSeason,
    season_progress: f32,
    storm: Option<SerdStorm>,
    storm_cooldown: f32,
    terrain_cells: Vec<u8>, // stored as u8 indices

    // RNG state
    rng_seed_state: Vec<u8>,

    // Sim state
    tick_count: u64,
    speed_multiplier: f32,
}

impl SaveState {
    pub fn from_sim(sim: &SimState) -> Self {
        let entities: Vec<Option<SerdEntity>> = sim.arena.entities.iter().map(|slot| {
            slot.as_ref().map(|e| SerdEntity {
                pos: e.pos.into(),
                prev_pos: e.prev_pos.into(),
                velocity: e.velocity.into(),
                heading: e.heading,
                radius: e.radius,
                color: e.color.into(),
                energy: e.energy,
                health: e.health,
                max_health: e.max_health,
                age: e.age,
                speed_multiplier: e.speed_multiplier,
                sensor_range: e.sensor_range,
                metabolic_rate: e.metabolic_rate,
                generation_depth: e.generation_depth,
                parent_idx: e.parent_id.map(|id| id.index),
                parent_gen: e.parent_id.map(|id| id.generation),
                offspring_count: e.offspring_count,
                tick_born: e.tick_born,
            })
        }).collect();

        let mut active_brain_slots = Vec::new();
        let mut brains = Vec::new();
        for (i, &active) in sim.brains.active.iter().enumerate() {
            if active {
                active_brain_slots.push(i);
                brains.push(SerdBrain {
                    states: sim.brains.states[i],
                    tau_inv: sim.brains.tau_inv[i],
                    biases: sim.brains.biases[i],
                    weights: sim.brains.weights[i],
                    outputs: sim.brains.outputs[i],
                });
            }
        }

        let genomes: Vec<Option<Vec<f32>>> = sim.genomes.iter().map(|g| {
            g.as_ref().map(|genome| genome.genes.clone())
        }).collect();

        let food: Vec<SerdFood> = sim.food.iter().map(|f| SerdFood {
            pos: f.pos.into(),
            energy: f.energy,
        }).collect();

        let meat: Vec<SerdMeat> = sim.meat.iter().map(|m| SerdMeat {
            pos: m.pos.into(),
            energy: m.energy,
            decay_timer: m.decay_timer,
        }).collect();

        let terrain_cells: Vec<u8> = sim.environment.terrain.cells.iter().map(|t| match t {
            TerrainType::Plains => 0,
            TerrainType::Forest => 1,
            TerrainType::Desert => 2,
            TerrainType::Water => 3,
            TerrainType::Toxic => 4,
        }).collect();

        let storm = sim.environment.storm.as_ref().map(|s| SerdStorm {
            center: s.center.into(),
            radius: s.radius,
            velocity: s.velocity.into(),
            timer: s.timer,
        });

        // Serialize RNG state via bincode
        let rng_seed_state = bincode::serialize(&sim.rng).unwrap_or_default();

        Self {
            entities,
            generations: sim.arena.generations.clone(),
            arena_count: sim.arena.count,
            active_brain_slots,
            brains,
            genomes,
            food,
            meat,
            pheromone_cells: sim.pheromone_grid.cells.clone(),
            time_of_day: sim.environment.time_of_day,
            day_progress: sim.environment.day_progress,
            season: sim.environment.season.into(),
            season_progress: sim.environment.season_progress,
            storm,
            storm_cooldown: sim.environment.storm_cooldown,
            terrain_cells,
            rng_seed_state,
            tick_count: sim.tick_count,
            speed_multiplier: sim.speed_multiplier,
        }
    }

    pub fn restore(&self) -> SimState {
        use crate::energy::FoodSpawner;
        use crate::spatial_hash::SpatialHash;
        use crate::world::World;
        use ::rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let world = World::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, config::WORLD_TOROIDAL);

        // Restore entity arena
        let entities: Vec<Option<Entity>> = self.entities.iter().map(|slot| {
            slot.as_ref().map(|e| {
                let parent_id = match (e.parent_idx, e.parent_gen) {
                    (Some(idx), Some(gen)) => Some(EntityId { index: idx, generation: gen }),
                    _ => None,
                };
                Entity {
                    pos: e.pos.clone().into(),
                    prev_pos: e.prev_pos.clone().into(),
                    velocity: e.velocity.clone().into(),
                    heading: e.heading,
                    radius: e.radius,
                    color: e.color.clone().into(),
                    energy: e.energy,
                    health: e.health,
                    max_health: e.max_health,
                    age: e.age,
                    alive: true,
                    speed_multiplier: e.speed_multiplier,
                    sensor_range: e.sensor_range,
                    metabolic_rate: e.metabolic_rate,
                    generation_depth: e.generation_depth,
                    parent_id,
                    offspring_count: e.offspring_count,
                    tick_born: e.tick_born,
                }
            })
        }).collect();

        let capacity = entities.len();
        let mut free_list: Vec<u32> = Vec::new();
        for (i, slot) in entities.iter().enumerate().rev() {
            if slot.is_none() {
                free_list.push(i as u32);
            }
        }

        let arena = EntityArena {
            entities,
            generations: self.generations.clone(),
            free_list,
            count: self.arena_count,
        };

        // Restore brains
        let mut brains = BrainStorage::new(capacity);
        for (i, &slot) in self.active_brain_slots.iter().enumerate() {
            if slot < capacity && i < self.brains.len() {
                let b = &self.brains[i];
                brains.states[slot] = b.states;
                brains.tau_inv[slot] = b.tau_inv;
                brains.biases[slot] = b.biases;
                brains.weights[slot] = b.weights;
                brains.outputs[slot] = b.outputs;
                brains.active[slot] = true;
            }
        }

        // Restore genomes
        let genomes: Vec<Option<Genome>> = self.genomes.iter().map(|g| {
            g.as_ref().map(|genes| Genome { genes: genes.clone() })
        }).collect();

        // Restore food + meat
        let food: Vec<FoodItem> = self.food.iter().map(|f| FoodItem {
            pos: f.pos.clone().into(),
            energy: f.energy,
        }).collect();

        let meat: Vec<MeatItem> = self.meat.iter().map(|m| MeatItem {
            pos: m.pos.clone().into(),
            energy: m.energy,
            decay_timer: m.decay_timer,
        }).collect();

        // Restore pheromone grid
        let mut pheromone_grid = PheromoneGrid::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, 32.0);
        if self.pheromone_cells.len() == pheromone_grid.cells.len() {
            pheromone_grid.cells = self.pheromone_cells.clone();
        }

        // Restore terrain
        let terrain_cells: Vec<TerrainType> = self.terrain_cells.iter().map(|&t| match t {
            0 => TerrainType::Plains,
            1 => TerrainType::Forest,
            2 => TerrainType::Desert,
            3 => TerrainType::Water,
            _ => TerrainType::Toxic,
        }).collect();

        let mut environment = EnvironmentState::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, 0);
        environment.time_of_day = self.time_of_day;
        environment.day_progress = self.day_progress;
        environment.season = self.season.clone().into();
        environment.season_progress = self.season_progress;
        environment.storm_cooldown = self.storm_cooldown;
        environment.storm = self.storm.as_ref().map(|s| Storm {
            center: s.center.clone().into(),
            radius: s.radius,
            velocity: s.velocity.clone().into(),
            timer: s.timer,
        });

        if terrain_cells.len() == environment.terrain.cells.len() {
            environment.terrain.cells = terrain_cells;
        }

        // Restore RNG
        let rng: ChaCha8Rng = bincode::deserialize(&self.rng_seed_state)
            .unwrap_or_else(|_| ChaCha8Rng::seed_from_u64(42));

        let spatial_hash = SpatialHash::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, config::SPATIAL_CELL_SIZE);
        let signals = vec![SignalState::default(); capacity];

        SimState {
            arena,
            brains,
            genomes,
            world,
            spatial_hash,
            food,
            food_spawner: FoodSpawner::new(),
            meat,
            signals,
            pheromone_grid,
            combat_events: Vec::new(),
            particles: ParticleSystem::new(),
            environment,
            rng,
            tick_count: self.tick_count,
            paused: false,
            speed_multiplier: self.speed_multiplier,
            show_rays: false,
            last_rays: Vec::new(),
        }
    }
}

/// Save the simulation state to a file.
pub fn save_to_file(sim: &SimState, path: &str) -> Result<(), String> {
    let state = SaveState::from_sim(sim);
    let bytes = bincode::serialize(&state).map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(path, bytes).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

/// Load simulation state from a file.
pub fn load_from_file(path: &str) -> Result<SimState, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Read error: {e}"))?;
    let state: SaveState = bincode::deserialize(&bytes).map_err(|e| format!("Deserialize error: {e}"))?;
    Ok(state.restore())
}
