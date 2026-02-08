use macroquad::prelude::*;
use ::rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::brain::BrainStorage;
use crate::combat::{self, CombatEvent, MeatItem};
use crate::config;
use crate::energy::{self, FoodSpawner};
use crate::entity::EntityArena;
use crate::environment::{self, EnvironmentState};
use crate::genome::Genome;
use crate::particles::ParticleSystem;
use crate::physics;
use crate::reproduction;
use crate::sensory::{self, EntityRays};
use crate::signals::{self, PheromoneGrid, SignalState};
use crate::spatial_hash::SpatialHash;
use crate::world::World;

/// Food item in the world.
#[derive(Clone, Debug)]
pub struct FoodItem {
    pub pos: Vec2,
    pub energy: f32,
}

pub struct SimState {
    pub arena: EntityArena,
    pub brains: BrainStorage,
    pub genomes: Vec<Option<Genome>>,
    pub world: World,
    pub spatial_hash: SpatialHash,
    pub food: Vec<FoodItem>,
    pub food_spawner: FoodSpawner,
    pub meat: Vec<MeatItem>,
    pub signals: Vec<SignalState>,
    pub pheromone_grid: PheromoneGrid,
    pub combat_events: Vec<CombatEvent>,
    pub particles: ParticleSystem,
    pub environment: EnvironmentState,
    pub rng: ChaCha8Rng,
    pub tick_count: u64,
    pub paused: bool,
    pub speed_multiplier: f32,
    pub show_rays: bool,
    pub last_rays: Vec<Option<EntityRays>>,
}

impl SimState {
    pub fn new(entity_count: usize, seed: u64) -> Self {
        let world = World::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, config::WORLD_TOROIDAL);
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut arena = EntityArena::new(config::MAX_ENTITY_COUNT);
        let mut brains = BrainStorage::new(config::MAX_ENTITY_COUNT);
        let mut genomes: Vec<Option<Genome>> = vec![None; config::MAX_ENTITY_COUNT];

        for _ in 0..entity_count {
            let pos = vec2(
                rng.gen_range(50.0..world.width - 50.0),
                rng.gen_range(50.0..world.height - 50.0),
            );
            let genome = Genome::random(&mut rng);
            let entity = crate::entity::Entity::new_from_genome_rng(&genome, pos, 0, &mut rng);
            if let Some(id) = arena.spawn(entity) {
                let slot = id.index as usize;
                brains.init_from_genome(slot, &genome);
                genomes[slot] = Some(genome);
            }
        }

        let mut food = Vec::with_capacity(config::INITIAL_FOOD_COUNT * 2);
        for _ in 0..config::INITIAL_FOOD_COUNT {
            food.push(FoodItem {
                pos: vec2(rng.gen_range(0.0..world.width), rng.gen_range(0.0..world.height)),
                energy: config::FOOD_ENERGY,
            });
        }

        let spatial_hash =
            SpatialHash::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, config::SPATIAL_CELL_SIZE);
        let pheromone_grid = PheromoneGrid::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, 32.0);

        Self {
            arena,
            brains,
            genomes,
            world,
            spatial_hash,
            food,
            food_spawner: FoodSpawner::new(),
            meat: Vec::new(),
            signals: vec![SignalState::default(); config::MAX_ENTITY_COUNT],
            pheromone_grid,
            combat_events: Vec::new(),
            particles: ParticleSystem::new(),
            environment: EnvironmentState::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, seed as u32),
            rng,
            tick_count: 0,
            paused: false,
            speed_multiplier: 1.0,
            show_rays: false,
            last_rays: Vec::new(),
        }
    }

    pub fn food_positions(&self) -> Vec<Vec2> {
        self.food.iter().map(|f| f.pos).collect()
    }

    pub fn tick(&mut self) {
        let dt = config::FIXED_DT;

        // Rebuild spatial hash
        self.spatial_hash.rebuild(&self.arena);

        // Sensory + Brain
        let food_pos = self.food_positions();
        let (sensor_inputs, rays) = sensory::compute_all_sensors(
            &self.arena,
            &food_pos,
            &self.spatial_hash,
            &self.world,
            &self.environment,
            self.show_rays,
        );
        self.last_rays = rays;
        self.brains.step_all(&sensor_inputs, dt);

        // Extract all motor outputs at once
        let entity_count = self.arena.entities.len();
        let mut motor_pairs = Vec::with_capacity(entity_count);
        let mut attack_intents = Vec::with_capacity(entity_count);
        let mut signal_intensities = Vec::with_capacity(entity_count);

        for slot in 0..entity_count {
            if self.brains.active.get(slot).copied().unwrap_or(false) {
                let (fwd, turn, attack, signal) = self.brains.motor_outputs(slot);
                motor_pairs.push((fwd, turn));
                attack_intents.push(attack);
                signal_intensities.push(signal);
            } else {
                motor_pairs.push((0.0, 0.0));
                attack_intents.push(0.0);
                signal_intensities.push(0.0);
            }
        }

        // Physics
        physics::apply_motor_outputs(&mut self.arena, &motor_pairs, dt);
        physics::integrate(&mut self.arena, &self.world, dt);
        self.spatial_hash.rebuild(&self.arena);
        physics::resolve_collisions(&mut self.arena, &self.spatial_hash, &self.world);

        // Combat
        self.combat_events = combat::resolve_combat(
            &mut self.arena,
            &attack_intents,
            &self.spatial_hash,
            &self.world,
            &mut self.meat,
        );

        // Emit combat particles
        for event in &self.combat_events {
            self.particles.emit_combat(event.target_pos);
        }

        // Meat consumption and decay
        combat::consume_meat(&mut self.arena, &mut self.meat, &self.world);
        combat::decay_meat(&mut self.meat, dt);

        // Energy: metabolism, food consumption, starvation
        energy::deduct_metabolism(&mut self.arena, dt);
        let eaten_positions = energy::consume_food(&mut self.arena, &mut self.food, &self.world);
        for pos in &eaten_positions {
            self.particles.emit_eat(*pos);
        }
        energy::kill_starved(&mut self.arena);

        // Food sharing: entities with high signal and adjacent neighbor share energy
        self.process_food_sharing();

        // Signals and pheromones
        signals::update_signals(
            &self.arena,
            &signal_intensities,
            &mut self.signals,
            &mut self.pheromone_grid,
            dt,
        );

        // Reproduction
        let birth_positions = reproduction::check_and_spawn(
            &mut self.arena,
            &mut self.brains,
            &mut self.genomes,
            &self.world,
            &mut self.rng,
            self.tick_count,
        );
        for pos in &birth_positions {
            self.particles.emit_birth(*pos);
        }

        // Sweep dead entities
        let dead = self.arena.sweep_dead();
        for (idx, pos) in &dead {
            self.brains.deactivate(*idx);
            if *idx < self.genomes.len() {
                self.genomes[*idx] = None;
            }
            self.particles.emit_death(*pos);
        }

        // Environment: terrain, storms, day/night, seasons
        environment::apply_terrain_effects(&mut self.arena, &self.environment.terrain, &self.world, dt);
        if let Some(ref storm) = self.environment.storm {
            let storm_clone = storm.clone();
            environment::apply_storm_effects(
                &mut self.arena,
                &storm_clone,
                &self.world,
                &self.environment.terrain,
                dt,
            );
        }
        self.environment.tick(dt, &self.world, &mut self.rng);

        // Respawn food (modulated by environment)
        let food_rate_mult = self.environment.food_rate_multiplier();
        self.food_spawner.accumulator += config::FOOD_RESPAWN_RATE * food_rate_mult * dt;
        let max_food = config::INITIAL_FOOD_COUNT * 2;
        while self.food_spawner.accumulator >= 1.0 && self.food.len() < max_food {
            let pos = vec2(
                self.rng.gen_range(0.0..self.world.width),
                self.rng.gen_range(0.0..self.world.height),
            );
            // Bias food spawning by terrain
            let terrain = self.environment.terrain.get_at(pos);
            if self.rng.gen::<f32>() < terrain.food_spawn_mult() {
                self.food.push(FoodItem {
                    pos,
                    energy: config::FOOD_ENERGY,
                });
            }
            self.food_spawner.accumulator -= 1.0;
        }

        // Update particles
        self.particles.update(dt);

        self.tick_count += 1;
    }

    /// Food sharing: entities with signal intensity > 0.7 share energy with closest neighbor
    fn process_food_sharing(&mut self) {
        let share_range = config::ATTACK_RANGE * 2.0; // slightly larger than attack range
        let share_amount = 5.0;
        let signal_threshold = 0.7;

        // Collect sharing intents: (giver_idx, receiver_idx)
        let mut shares: Vec<(usize, usize)> = Vec::new();

        for (idx, entity) in self.arena.entities.iter().enumerate() {
            let entity = match entity {
                Some(e) => e,
                None => continue,
            };

            // Check if entity's signal output is high enough to indicate sharing intent
            if idx >= self.signals.len() {
                continue;
            }
            let sig = &self.signals[idx];
            let sig_intensity = sig.intensity;
            if sig_intensity < signal_threshold {
                continue;
            }

            // Must have enough energy to share
            if entity.energy < share_amount * 2.0 {
                continue;
            }

            // Find closest neighbor
            let neighbors = self.spatial_hash.query_radius_excluding(
                entity.pos,
                share_range,
                idx as u32,
                &self.world,
                &self.arena,
            );

            if let Some(&neighbor_idx) = neighbors.first() {
                shares.push((idx, neighbor_idx as usize));
            }
        }

        // Apply shares (two-pass to avoid double mutable borrow)
        for (giver, receiver) in shares {
            let can_give = self.arena.entities.get(giver)
                .and_then(|e| e.as_ref())
                .map(|e| e.energy > share_amount * 2.0)
                .unwrap_or(false);
            if can_give {
                if let Some(Some(giver_e)) = self.arena.entities.get_mut(giver) {
                    giver_e.energy -= share_amount;
                }
                if let Some(Some(receiver_e)) = self.arena.entities.get_mut(receiver) {
                    receiver_e.energy = (receiver_e.energy + share_amount).min(config::MAX_ENTITY_ENERGY);
                }
            }
        }
    }
}
