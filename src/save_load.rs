use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::brain::BrainStorage;
use crate::combat::MeatItem;
use crate::config;
use crate::entity::{Entity, EntityArena, EntityId};
use crate::environment::{EnvironmentState, Season, Storm, TerrainType, ToxicZone, WallSegment};
use crate::genome::Genome;
#[cfg(test)]
use crate::genome::TOTAL_GENOME_SIZE;
use crate::particles::ParticleSystem;
use crate::signals::{PheromoneGrid, SignalState};
use crate::simulation::{FoodItem, SimState};

const SAVE_VERSION_V2: u32 = 2;
const LEGACY_N: usize = config::BRAIN_NEURONS_DEFAULT;

fn default_entity_alive() -> bool {
    true
}

fn default_carried_energy() -> f32 {
    0.0
}

// Serde-friendly wrapper types for macroquad primitives.

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
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }
}

impl From<SerdColor> for Color {
    fn from(c: SerdColor) -> Self {
        Color::new(c.r, c.g, c.b, c.a)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdEntity {
    pos: SerdVec2,
    prev_pos: SerdVec2,
    velocity: SerdVec2,
    heading: f32,
    radius: f32,
    color: SerdColor,
    energy: f32,
    #[serde(default = "default_carried_energy")]
    carried_energy: f32,
    health: f32,
    max_health: f32,
    age: f32,
    #[serde(default = "default_entity_alive")]
    alive: bool,
    speed_multiplier: f32,
    sensor_range: f32,
    metabolic_rate: f32,
    generation_depth: u32,
    parent_idx: Option<u32>,
    parent_gen: Option<u32>,
    offspring_count: u32,
    tick_born: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdFood {
    pos: SerdVec2,
    energy: f32,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdMeat {
    pos: SerdVec2,
    energy: f32,
    decay_timer: f32,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdStorm {
    center: SerdVec2,
    radius: f32,
    velocity: SerdVec2,
    timer: f32,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdWall {
    start: SerdVec2,
    end: SerdVec2,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdToxicZone {
    center: SerdVec2,
    radius: f32,
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

#[derive(Clone, Serialize, Deserialize)]
struct SerdBrainV2 {
    neuron_count: u16,
    states: Vec<f32>,
    tau_inv: Vec<f32>,
    biases: Vec<f32>,
    weights: Vec<f32>,
    outputs: Vec<f32>,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdGenomeV2 {
    inter_neurons: u8,
    genes: Vec<f32>,
}

#[derive(Clone, Serialize, Deserialize)]
struct SaveStateV2 {
    version: u32,

    // Entity arena
    entities: Vec<Option<SerdEntity>>,
    generations: Vec<u32>,
    arena_count: usize,

    // Brains (only active slots)
    active_brain_slots: Vec<usize>,
    brains: Vec<SerdBrainV2>,

    // Genomes
    genomes: Vec<Option<SerdGenomeV2>>,

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
    terrain_cells: Vec<u8>,
    walls: Vec<SerdWall>,
    toxic_zones: Vec<SerdToxicZone>,

    // RNG state
    rng_seed_state: Vec<u8>,

    // Sim state
    tick_count: u64,
    speed_multiplier: f32,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdBrainV1 {
    states: [f32; LEGACY_N],
    tau_inv: [f32; LEGACY_N],
    biases: [f32; LEGACY_N],
    weights: [[f32; LEGACY_N]; LEGACY_N],
    outputs: [f32; LEGACY_N],
}

#[derive(Clone, Serialize, Deserialize)]
struct SaveStateV1 {
    entities: Vec<Option<SerdEntity>>,
    generations: Vec<u32>,
    arena_count: usize,

    active_brain_slots: Vec<usize>,
    brains: Vec<SerdBrainV1>,

    genomes: Vec<Option<Vec<f32>>>,

    food: Vec<SerdFood>,
    meat: Vec<SerdMeat>,

    pheromone_cells: Vec<f32>,

    time_of_day: f32,
    day_progress: f32,
    season: SerdSeason,
    season_progress: f32,
    storm: Option<SerdStorm>,
    storm_cooldown: f32,
    terrain_cells: Vec<u8>,
    walls: Vec<SerdWall>,
    toxic_zones: Vec<SerdToxicZone>,

    rng_seed_state: Vec<u8>,

    tick_count: u64,
    speed_multiplier: f32,
}

impl SaveStateV2 {
    pub fn from_sim(sim: &SimState) -> Self {
        let entities: Vec<Option<SerdEntity>> = sim
            .arena
            .entities
            .iter()
            .map(|slot| {
                slot.as_ref().map(|e| SerdEntity {
                    pos: e.pos.into(),
                    prev_pos: e.prev_pos.into(),
                    velocity: e.velocity.into(),
                    heading: e.heading,
                    radius: e.radius,
                    color: e.color.into(),
                    energy: e.energy,
                    carried_energy: e.carried_energy,
                    health: e.health,
                    max_health: e.max_health,
                    age: e.age,
                    alive: e.alive,
                    speed_multiplier: e.speed_multiplier,
                    sensor_range: e.sensor_range,
                    metabolic_rate: e.metabolic_rate,
                    generation_depth: e.generation_depth,
                    parent_idx: e.parent_id.map(|id| id.index),
                    parent_gen: e.parent_id.map(|id| id.generation),
                    offspring_count: e.offspring_count,
                    tick_born: e.tick_born,
                })
            })
            .collect();

        let mut active_brain_slots = Vec::new();
        let mut brains = Vec::new();
        for (i, &active) in sim.brains.active.iter().enumerate() {
            if !active {
                continue;
            }

            let states = sim.brains.states.get(i).cloned().unwrap_or_default();
            let tau_inv = sim.brains.tau_inv.get(i).cloned().unwrap_or_default();
            let biases = sim.brains.biases.get(i).cloned().unwrap_or_default();
            let weights = sim.brains.weights.get(i).cloned().unwrap_or_default();
            let outputs = sim.brains.outputs.get(i).cloned().unwrap_or_default();

            let n = states.len();
            if n == 0 {
                continue;
            }

            active_brain_slots.push(i);
            brains.push(SerdBrainV2 {
                neuron_count: n as u16,
                states,
                tau_inv,
                biases,
                weights,
                outputs,
            });
        }

        let genomes: Vec<Option<SerdGenomeV2>> = sim
            .genomes
            .iter()
            .map(|g| {
                g.as_ref().map(|genome| SerdGenomeV2 {
                    inter_neurons: genome.inter_neurons() as u8,
                    genes: genome.genes.clone(),
                })
            })
            .collect();

        let food: Vec<SerdFood> = sim
            .food
            .iter()
            .map(|f| SerdFood {
                pos: f.pos.into(),
                energy: f.energy,
            })
            .collect();

        let meat: Vec<SerdMeat> = sim
            .meat
            .iter()
            .map(|m| SerdMeat {
                pos: m.pos.into(),
                energy: m.energy,
                decay_timer: m.decay_timer,
            })
            .collect();

        let terrain_cells: Vec<u8> = sim
            .environment
            .terrain
            .cells
            .iter()
            .map(|t| match t {
                TerrainType::Plains => 0,
                TerrainType::Forest => 1,
                TerrainType::Desert => 2,
                TerrainType::Water => 3,
                TerrainType::Toxic => 4,
            })
            .collect();

        let storm = sim.environment.storm.as_ref().map(|s| SerdStorm {
            center: s.center.into(),
            radius: s.radius,
            velocity: s.velocity.into(),
            timer: s.timer,
        });

        let walls = sim
            .environment
            .walls
            .iter()
            .map(|w| SerdWall {
                start: w.start.into(),
                end: w.end.into(),
            })
            .collect();

        let toxic_zones = sim
            .environment
            .toxic_zones
            .iter()
            .map(|z| SerdToxicZone {
                center: z.center.into(),
                radius: z.radius,
                timer: z.timer,
            })
            .collect();

        let rng_seed_state = bincode::serialize(&sim.rng).unwrap_or_default();

        Self {
            version: SAVE_VERSION_V2,
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
            walls,
            toxic_zones,
            rng_seed_state,
            tick_count: sim.tick_count,
            speed_multiplier: sim.speed_multiplier,
        }
    }

    pub fn restore(&self) -> Result<SimState, String> {
        use ::rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let arena = restore_entity_arena(&self.entities, &self.generations, self.arena_count);
        let capacity = arena.entities.len();

        let mut brains = BrainStorage::new(capacity);
        for (i, &slot) in self.active_brain_slots.iter().enumerate() {
            if slot >= capacity || i >= self.brains.len() {
                continue;
            }
            let b = &self.brains[i];
            let n = b.neuron_count as usize;
            let expected_weights = n
                .checked_mul(n)
                .ok_or_else(|| format!("brain neuron count overflow for slot {slot}"))?;

            if b.states.len() != n
                || b.tau_inv.len() != n
                || b.biases.len() != n
                || b.outputs.len() != n
                || b.weights.len() != expected_weights
            {
                return Err(format!(
                    "invalid brain tensor lengths for slot {slot}: n={n}, states={}, tau_inv={}, biases={}, outputs={}, weights={}",
                    b.states.len(),
                    b.tau_inv.len(),
                    b.biases.len(),
                    b.outputs.len(),
                    b.weights.len(),
                ));
            }

            brains.states[slot] = b.states.clone();
            brains.tau_inv[slot] = b.tau_inv.clone();
            brains.biases[slot] = b.biases.clone();
            brains.weights[slot] = b.weights.clone();
            brains.outputs[slot] = b.outputs.clone();
            brains.active[slot] = true;
        }

        let genomes: Vec<Option<Genome>> = self
            .genomes
            .iter()
            .map(|g| {
                g.as_ref().map(|sg| {
                    let inter = Genome::clamp_interneuron_count(sg.inter_neurons as usize);
                    Genome::from_raw(inter, sg.genes.clone())
                })
            })
            .collect();

        let food: Vec<FoodItem> = self
            .food
            .iter()
            .map(|f| FoodItem {
                pos: f.pos.clone().into(),
                energy: f.energy,
            })
            .collect();

        let meat: Vec<MeatItem> = self
            .meat
            .iter()
            .map(|m| MeatItem {
                pos: m.pos.clone().into(),
                energy: m.energy,
                decay_timer: m.decay_timer,
            })
            .collect();

        let mut pheromone_grid =
            PheromoneGrid::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, 32.0);
        if self.pheromone_cells.len() == pheromone_grid.cells.len() {
            pheromone_grid.cells = self.pheromone_cells.clone();
        }

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
        environment.walls = self
            .walls
            .iter()
            .map(|w| WallSegment {
                start: w.start.clone().into(),
                end: w.end.clone().into(),
            })
            .collect();
        environment.toxic_zones = self
            .toxic_zones
            .iter()
            .map(|z| ToxicZone {
                center: z.center.clone().into(),
                radius: z.radius,
                timer: z.timer,
            })
            .collect();

        let terrain_cells: Vec<TerrainType> = self
            .terrain_cells
            .iter()
            .map(|&t| match t {
                0 => TerrainType::Plains,
                1 => TerrainType::Forest,
                2 => TerrainType::Desert,
                3 => TerrainType::Water,
                _ => TerrainType::Toxic,
            })
            .collect();
        if terrain_cells.len() == environment.terrain.cells.len() {
            environment.terrain.cells = terrain_cells;
        }

        let rng: ChaCha8Rng = bincode::deserialize(&self.rng_seed_state)
            .unwrap_or_else(|_| ChaCha8Rng::seed_from_u64(42));

        Ok(build_sim_state(
            arena,
            brains,
            genomes,
            food,
            meat,
            pheromone_grid,
            environment,
            rng,
            self.tick_count,
            self.speed_multiplier,
        ))
    }
}

impl SaveStateV1 {
    #[cfg(test)]
    fn from_sim(sim: &SimState) -> Self {
        let entities: Vec<Option<SerdEntity>> = sim
            .arena
            .entities
            .iter()
            .map(|slot| {
                slot.as_ref().map(|e| SerdEntity {
                    pos: e.pos.into(),
                    prev_pos: e.prev_pos.into(),
                    velocity: e.velocity.into(),
                    heading: e.heading,
                    radius: e.radius,
                    color: e.color.into(),
                    energy: e.energy,
                    carried_energy: e.carried_energy,
                    health: e.health,
                    max_health: e.max_health,
                    age: e.age,
                    alive: e.alive,
                    speed_multiplier: e.speed_multiplier,
                    sensor_range: e.sensor_range,
                    metabolic_rate: e.metabolic_rate,
                    generation_depth: e.generation_depth,
                    parent_idx: e.parent_id.map(|id| id.index),
                    parent_gen: e.parent_id.map(|id| id.generation),
                    offspring_count: e.offspring_count,
                    tick_born: e.tick_born,
                })
            })
            .collect();

        let mut active_brain_slots = Vec::new();
        let mut brains = Vec::new();
        for (i, &active) in sim.brains.active.iter().enumerate() {
            if !active {
                continue;
            }

            let mut states = [0.0; LEGACY_N];
            let mut tau_inv = [1.0; LEGACY_N];
            let mut biases = [0.0; LEGACY_N];
            let mut outputs = [0.0; LEGACY_N];
            let mut weights = [[0.0; LEGACY_N]; LEGACY_N];

            let slot_states = sim.brains.states.get(i).map(Vec::as_slice).unwrap_or(&[]);
            let slot_tau = sim.brains.tau_inv.get(i).map(Vec::as_slice).unwrap_or(&[]);
            let slot_biases = sim.brains.biases.get(i).map(Vec::as_slice).unwrap_or(&[]);
            let slot_outputs = sim.brains.outputs.get(i).map(Vec::as_slice).unwrap_or(&[]);
            let slot_weights = sim.brains.weights.get(i).map(Vec::as_slice).unwrap_or(&[]);
            let n = slot_states.len().min(LEGACY_N);

            for idx in 0..n {
                states[idx] = slot_states[idx];
                tau_inv[idx] = slot_tau.get(idx).copied().unwrap_or(1.0);
                biases[idx] = slot_biases.get(idx).copied().unwrap_or(0.0);
                outputs[idx] = slot_outputs.get(idx).copied().unwrap_or(0.0);
            }
            for to in 0..n {
                for from in 0..n {
                    weights[to][from] = slot_weights
                        .get(to * slot_states.len() + from)
                        .copied()
                        .unwrap_or(0.0);
                }
            }

            active_brain_slots.push(i);
            brains.push(SerdBrainV1 {
                states,
                tau_inv,
                biases,
                weights,
                outputs,
            });
        }

        let genomes: Vec<Option<Vec<f32>>> = sim
            .genomes
            .iter()
            .map(|g| {
                g.as_ref().map(|genome| {
                    let mut adjusted = genome.genes.clone();
                    if adjusted.len() < TOTAL_GENOME_SIZE {
                        adjusted.resize(TOTAL_GENOME_SIZE, 0.5);
                    } else if adjusted.len() > TOTAL_GENOME_SIZE {
                        adjusted.truncate(TOTAL_GENOME_SIZE);
                    }
                    adjusted
                })
            })
            .collect();

        let food: Vec<SerdFood> = sim
            .food
            .iter()
            .map(|f| SerdFood {
                pos: f.pos.into(),
                energy: f.energy,
            })
            .collect();

        let meat: Vec<SerdMeat> = sim
            .meat
            .iter()
            .map(|m| SerdMeat {
                pos: m.pos.into(),
                energy: m.energy,
                decay_timer: m.decay_timer,
            })
            .collect();

        let terrain_cells: Vec<u8> = sim
            .environment
            .terrain
            .cells
            .iter()
            .map(|t| match t {
                TerrainType::Plains => 0,
                TerrainType::Forest => 1,
                TerrainType::Desert => 2,
                TerrainType::Water => 3,
                TerrainType::Toxic => 4,
            })
            .collect();

        let storm = sim.environment.storm.as_ref().map(|s| SerdStorm {
            center: s.center.into(),
            radius: s.radius,
            velocity: s.velocity.into(),
            timer: s.timer,
        });

        let walls = sim
            .environment
            .walls
            .iter()
            .map(|w| SerdWall {
                start: w.start.into(),
                end: w.end.into(),
            })
            .collect();

        let toxic_zones = sim
            .environment
            .toxic_zones
            .iter()
            .map(|z| SerdToxicZone {
                center: z.center.into(),
                radius: z.radius,
                timer: z.timer,
            })
            .collect();

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
            walls,
            toxic_zones,
            rng_seed_state,
            tick_count: sim.tick_count,
            speed_multiplier: sim.speed_multiplier,
        }
    }

    fn restore(&self) -> Result<SimState, String> {
        use ::rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let arena = restore_entity_arena(&self.entities, &self.generations, self.arena_count);
        let capacity = arena.entities.len();

        let mut brains = BrainStorage::new(capacity);
        for (i, &slot) in self.active_brain_slots.iter().enumerate() {
            if slot >= capacity || i >= self.brains.len() {
                continue;
            }
            let b = &self.brains[i];
            let n = LEGACY_N;

            let mut weights = vec![0.0; n * n];
            for to in 0..n {
                for from in 0..n {
                    weights[to * n + from] = b.weights[to][from];
                }
            }

            brains.states[slot] = b.states.to_vec();
            brains.tau_inv[slot] = b.tau_inv.to_vec();
            brains.biases[slot] = b.biases.to_vec();
            brains.weights[slot] = weights;
            brains.outputs[slot] = b.outputs.to_vec();
            brains.active[slot] = true;
        }

        let genomes: Vec<Option<Genome>> = self
            .genomes
            .iter()
            .map(|g| {
                g.as_ref().map(|genes| {
                    Genome::from_raw(config::BRAIN_INTERNEURONS_DEFAULT, genes.clone())
                })
            })
            .collect();

        let food: Vec<FoodItem> = self
            .food
            .iter()
            .map(|f| FoodItem {
                pos: f.pos.clone().into(),
                energy: f.energy,
            })
            .collect();

        let meat: Vec<MeatItem> = self
            .meat
            .iter()
            .map(|m| MeatItem {
                pos: m.pos.clone().into(),
                energy: m.energy,
                decay_timer: m.decay_timer,
            })
            .collect();

        let mut pheromone_grid =
            PheromoneGrid::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, 32.0);
        if self.pheromone_cells.len() == pheromone_grid.cells.len() {
            pheromone_grid.cells = self.pheromone_cells.clone();
        }

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
        environment.walls = self
            .walls
            .iter()
            .map(|w| WallSegment {
                start: w.start.clone().into(),
                end: w.end.clone().into(),
            })
            .collect();
        environment.toxic_zones = self
            .toxic_zones
            .iter()
            .map(|z| ToxicZone {
                center: z.center.clone().into(),
                radius: z.radius,
                timer: z.timer,
            })
            .collect();

        let terrain_cells: Vec<TerrainType> = self
            .terrain_cells
            .iter()
            .map(|&t| match t {
                0 => TerrainType::Plains,
                1 => TerrainType::Forest,
                2 => TerrainType::Desert,
                3 => TerrainType::Water,
                _ => TerrainType::Toxic,
            })
            .collect();
        if terrain_cells.len() == environment.terrain.cells.len() {
            environment.terrain.cells = terrain_cells;
        }

        let rng: ChaCha8Rng = bincode::deserialize(&self.rng_seed_state)
            .unwrap_or_else(|_| ChaCha8Rng::seed_from_u64(42));

        Ok(build_sim_state(
            arena,
            brains,
            genomes,
            food,
            meat,
            pheromone_grid,
            environment,
            rng,
            self.tick_count,
            self.speed_multiplier,
        ))
    }
}

fn restore_entity_arena(
    entities: &[Option<SerdEntity>],
    generations: &[u32],
    arena_count: usize,
) -> EntityArena {
    let restored_entities: Vec<Option<Entity>> = entities
        .iter()
        .map(|slot| {
            slot.as_ref().map(|e| {
                let parent_id = match (e.parent_idx, e.parent_gen) {
                    (Some(idx), Some(gen)) => Some(EntityId {
                        index: idx,
                        generation: gen,
                    }),
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
                    carried_energy: e.carried_energy,
                    health: e.health,
                    max_health: e.max_health,
                    age: e.age,
                    alive: e.alive,
                    speed_multiplier: e.speed_multiplier,
                    sensor_range: e.sensor_range,
                    metabolic_rate: e.metabolic_rate,
                    generation_depth: e.generation_depth,
                    parent_id,
                    offspring_count: e.offspring_count,
                    tick_born: e.tick_born,
                }
            })
        })
        .collect();

    let capacity = restored_entities.len();
    let mut free_list: Vec<u32> = Vec::new();
    for (i, slot) in restored_entities.iter().enumerate().rev() {
        if slot.is_none() {
            free_list.push(i as u32);
        }
    }

    let mut fixed_generations = generations.to_vec();
    if fixed_generations.len() < capacity {
        fixed_generations.resize(capacity, 0);
    } else if fixed_generations.len() > capacity {
        fixed_generations.truncate(capacity);
    }

    let live_slots = restored_entities
        .iter()
        .filter(|slot| slot.is_some())
        .count();

    EntityArena {
        entities: restored_entities,
        generations: fixed_generations,
        free_list,
        count: arena_count.min(live_slots),
    }
}

fn build_sim_state(
    arena: EntityArena,
    brains: BrainStorage,
    genomes: Vec<Option<Genome>>,
    food: Vec<FoodItem>,
    meat: Vec<MeatItem>,
    pheromone_grid: PheromoneGrid,
    environment: EnvironmentState,
    rng: rand_chacha::ChaCha8Rng,
    tick_count: u64,
    speed_multiplier: f32,
) -> SimState {
    use crate::energy::FoodSpawner;
    use crate::spatial_hash::SpatialHash;
    use crate::world::World;

    let world = World::new(
        config::WORLD_WIDTH,
        config::WORLD_HEIGHT,
        config::WORLD_TOROIDAL,
    );
    let capacity = arena.entities.len();
    let food_cap = food.len();
    let meat_cap = meat.len();

    let mut sim = SimState {
        arena,
        brains,
        genomes,
        world,
        spatial_hash: SpatialHash::new(
            config::WORLD_WIDTH,
            config::WORLD_HEIGHT,
            config::SPATIAL_CELL_SIZE,
        ),
        food,
        food_spawner: FoodSpawner::new(),
        meat,
        signals: vec![SignalState::default(); capacity],
        pheromone_grid,
        combat_events: Vec::new(),
        particles: ParticleSystem::new(),
        environment,
        rng,
        tick_count,
        paused: false,
        speed_multiplier,
        show_rays: false,
        last_rays: Vec::new(),
        births_last_tick: 0,
        deaths_last_tick: 0,
        motors_scratch: Vec::with_capacity(capacity),
        attack_intents_scratch: Vec::with_capacity(capacity),
        eat_intents_scratch: Vec::with_capacity(capacity),
        pickup_intents_scratch: Vec::with_capacity(capacity),
        share_intents_scratch: Vec::with_capacity(capacity),
        reproduce_intents_scratch: Vec::with_capacity(capacity),
        signal_colors_scratch: Vec::with_capacity(capacity),
        food_positions_scratch: Vec::with_capacity(food_cap),
        meat_positions_scratch: Vec::with_capacity(meat_cap),
        cached_avg_energy: 0.0,
        cached_avg_age: 0.0,
        cached_avg_size: 0.0,
        cached_avg_generation: 0.0,
        cached_species_estimate: 0,
        cached_species_tick: tick_count,
    };
    sim.refresh_population_cache(true);
    sim
}

/// Save the simulation state to a file.
pub fn save_to_file(sim: &SimState, path: &str) -> Result<(), String> {
    let state = SaveStateV2::from_sim(sim);
    let bytes = bincode::serialize(&state).map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(path, bytes).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

/// Load simulation state from a file.
pub fn load_from_file(path: &str) -> Result<SimState, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Read error: {e}"))?;

    match bincode::deserialize::<SaveStateV2>(&bytes) {
        Ok(v2) if v2.version == SAVE_VERSION_V2 => return v2.restore(),
        Ok(_) | Err(_) => {}
    }

    match bincode::deserialize::<SaveStateV1>(&bytes) {
        Ok(v1) => v1.restore(),
        Err(e) => Err(format!(
            "Deserialize error: not valid v2 or legacy v1 save ({e})"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::rand::SeedableRng;
    use macroquad::prelude::vec2;
    use rand_chacha::ChaCha8Rng;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("/tmp/{}_{}.bin", name, nanos)
    }

    #[test]
    fn save_restore_preserves_entity_alive_state() {
        let mut sim = SimState::new(1, 123);
        let idx = sim.arena.iter_alive().next().unwrap().0;
        if let Some(entity) = sim.arena.get_mut_by_index(idx) {
            entity.alive = false;
        }

        let save_state = SaveStateV2::from_sim(&sim);
        let restored = save_state.restore().unwrap();
        let restored_entity = restored.arena.get_by_index(idx).unwrap();
        assert!(!restored_entity.alive);
    }

    #[test]
    fn save_restore_preserves_carried_energy_and_environment_tools() {
        let mut sim = SimState::new(1, 99);
        let idx = sim.arena.iter_alive().next().unwrap().0;
        if let Some(entity) = sim.arena.get_mut_by_index(idx) {
            entity.carried_energy = 37.5;
        }

        sim.environment.add_wall(vec2(20.0, 20.0), vec2(80.0, 30.0));
        sim.environment
            .add_toxic_zone(vec2(110.0, 120.0), 45.0, 7.0);

        let save_state = SaveStateV2::from_sim(&sim);
        let restored = save_state.restore().unwrap();

        let restored_entity = restored.arena.get_by_index(idx).unwrap();
        assert!((restored_entity.carried_energy - 37.5).abs() < 1e-5);
        assert_eq!(restored.environment.walls.len(), 1);
        assert_eq!(restored.environment.toxic_zones.len(), 1);
    }

    #[test]
    fn v2_roundtrip_preserves_topology_and_brain_payload() {
        let mut sim = SimState::new(1, 17);
        let idx = sim.arena.iter_alive().next().unwrap().0;

        let mut rng = ChaCha8Rng::seed_from_u64(44);
        let genome = Genome::random_with_inter_neurons(&mut rng, 12);
        sim.genomes[idx] = Some(genome.clone());
        sim.brains.init_from_genome(idx, &genome);
        sim.brains.states[idx][0] = 1.25;

        let save_state = SaveStateV2::from_sim(&sim);
        let restored = save_state.restore().unwrap();

        let restored_genome = restored.genomes[idx].as_ref().unwrap();
        assert_eq!(restored_genome.inter_neurons(), 12);
        assert_eq!(restored_genome.genes.len(), genome.genes.len());
        assert_eq!(
            restored.brains.neuron_count(idx),
            Some(genome.total_neurons())
        );
        assert!((restored.brains.states[idx][0] - 1.25).abs() < 1e-6);
    }

    #[test]
    fn legacy_v1_bytes_load_via_fallback() {
        let sim = SimState::new(3, 33);
        let legacy = SaveStateV1::from_sim(&sim);

        let bytes = bincode::serialize(&legacy).unwrap();
        let path = temp_file("legacy_v1");
        std::fs::write(&path, bytes).unwrap();

        let restored = load_from_file(&path).unwrap();
        assert_eq!(restored.arena.count, sim.arena.count);
        assert_eq!(restored.food.len(), sim.food.len());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn restore_adjusts_legacy_genome_lengths() {
        let sim = SimState::new(2, 7);
        let mut save_state = SaveStateV1::from_sim(&sim);

        let first = save_state
            .genomes
            .iter()
            .position(|g| g.is_some())
            .expect("expected at least one genome slot");
        save_state.genomes[first] = Some(vec![0.25; TOTAL_GENOME_SIZE - 3]);

        let second = save_state
            .genomes
            .iter()
            .enumerate()
            .find(|(i, g)| *i != first && g.is_some())
            .map(|(i, _)| i)
            .expect("expected at least two genome slots");
        save_state.genomes[second] = Some(vec![0.75; TOTAL_GENOME_SIZE + 5]);

        let restored = save_state.restore().unwrap();
        let short_fixed = restored.genomes[first].as_ref().unwrap();
        let long_fixed = restored.genomes[second].as_ref().unwrap();

        assert_eq!(
            short_fixed.genes.len(),
            Genome::total_gene_len_for_inter(config::BRAIN_INTERNEURONS_DEFAULT)
        );
        assert_eq!(
            long_fixed.genes.len(),
            Genome::total_gene_len_for_inter(config::BRAIN_INTERNEURONS_DEFAULT)
        );
        assert!((short_fixed.genes[short_fixed.genes.len() - 1] - 0.5).abs() < 1e-5);
        assert!((long_fixed.genes[long_fixed.genes.len() - 1] - 0.75).abs() < 1e-5);
    }

    #[test]
    fn corrupt_v2_tensor_lengths_fail_load() {
        let sim = SimState::new(1, 77);
        let mut v2 = SaveStateV2::from_sim(&sim);

        if let Some(brain) = v2.brains.first_mut() {
            brain.states.pop();
        }

        let path = temp_file("bad_v2");
        let bytes = bincode::serialize(&v2).unwrap();
        std::fs::write(&path, bytes).unwrap();

        let err = match load_from_file(&path) {
            Ok(_) => panic!("expected load error for corrupt v2 payload"),
            Err(err) => err,
        };
        assert!(err.contains("invalid brain tensor lengths"));

        let _ = std::fs::remove_file(path);
    }
}
