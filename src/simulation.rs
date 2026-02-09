use ::rand::{Rng, SeedableRng};
use macroquad::prelude::*;
use rand_chacha::ChaCha8Rng;

use crate::brain::{BrainStorage, MotorOutputs};
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
    pub births_last_tick: u32,
    pub deaths_last_tick: u32,
    pub(crate) motors_scratch: Vec<MotorOutputs>,
    pub(crate) attack_intents_scratch: Vec<f32>,
    pub(crate) eat_intents_scratch: Vec<f32>,
    pub(crate) pickup_intents_scratch: Vec<f32>,
    pub(crate) share_intents_scratch: Vec<f32>,
    pub(crate) reproduce_intents_scratch: Vec<f32>,
    pub(crate) signal_colors_scratch: Vec<[f32; 3]>,
    pub(crate) food_positions_scratch: Vec<Vec2>,
    pub(crate) meat_positions_scratch: Vec<Vec2>,
    pub cached_avg_energy: f32,
    pub cached_avg_age: f32,
    pub cached_avg_size: f32,
    pub cached_avg_generation: f32,
    pub cached_species_estimate: usize,
    pub(crate) cached_species_tick: u64,
}

impl SimState {
    pub fn new(entity_count: usize, seed: u64) -> Self {
        let world = World::new(
            config::WORLD_WIDTH,
            config::WORLD_HEIGHT,
            config::WORLD_TOROIDAL,
        );
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
                pos: vec2(
                    rng.gen_range(0.0..world.width),
                    rng.gen_range(0.0..world.height),
                ),
                energy: config::FOOD_ENERGY,
            });
        }

        let spatial_hash = SpatialHash::new(
            config::WORLD_WIDTH,
            config::WORLD_HEIGHT,
            config::SPATIAL_CELL_SIZE,
        );
        let pheromone_grid = PheromoneGrid::new(config::WORLD_WIDTH, config::WORLD_HEIGHT, 32.0);

        let mut sim = Self {
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
            environment: EnvironmentState::new(
                config::WORLD_WIDTH,
                config::WORLD_HEIGHT,
                seed as u32,
            ),
            rng,
            tick_count: 0,
            paused: false,
            speed_multiplier: 1.0,
            show_rays: false,
            last_rays: Vec::new(),
            births_last_tick: 0,
            deaths_last_tick: 0,
            motors_scratch: Vec::with_capacity(config::MAX_ENTITY_COUNT),
            attack_intents_scratch: Vec::with_capacity(config::MAX_ENTITY_COUNT),
            eat_intents_scratch: Vec::with_capacity(config::MAX_ENTITY_COUNT),
            pickup_intents_scratch: Vec::with_capacity(config::MAX_ENTITY_COUNT),
            share_intents_scratch: Vec::with_capacity(config::MAX_ENTITY_COUNT),
            reproduce_intents_scratch: Vec::with_capacity(config::MAX_ENTITY_COUNT),
            signal_colors_scratch: Vec::with_capacity(config::MAX_ENTITY_COUNT),
            food_positions_scratch: Vec::with_capacity(config::INITIAL_FOOD_COUNT * 2),
            meat_positions_scratch: Vec::new(),
            cached_avg_energy: 0.0,
            cached_avg_age: 0.0,
            cached_avg_size: 0.0,
            cached_avg_generation: 0.0,
            cached_species_estimate: 0,
            cached_species_tick: 0,
        };
        sim.refresh_population_cache(true);
        sim
    }

    pub fn spawn_food_cluster(&mut self, center: Vec2, count: usize) {
        for _ in 0..count {
            let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let dist = self.rng.gen_range(0.0..config::FOOD_CLUSTER_RADIUS);
            let pos = self.world.wrap(center + Vec2::from_angle(angle) * dist);
            self.food.push(FoodItem {
                pos,
                energy: config::FOOD_ENERGY,
            });
        }
    }

    pub fn spawn_toxic_zone(&mut self, center: Vec2) {
        self.environment.add_toxic_zone(
            center,
            config::TOXIC_ZONE_RADIUS,
            config::TOXIC_ZONE_DURATION,
        );
    }

    pub fn add_wall(&mut self, start: Vec2, end: Vec2) {
        self.environment.add_wall(start, end);
    }

    pub fn tick(&mut self) {
        let dt = config::FIXED_DT;
        self.births_last_tick = 0;
        self.deaths_last_tick = 0;

        // Rebuild spatial hash
        self.spatial_hash.rebuild(&self.arena);

        // Sensory + Brain
        self.refresh_position_caches();
        let (sensor_inputs, rays) = sensory::compute_all_sensors(
            &self.arena,
            &self.food_positions_scratch,
            &self.meat_positions_scratch,
            &self.signals,
            &self.pheromone_grid,
            &self.spatial_hash,
            &self.world,
            &self.environment,
            self.show_rays,
        );
        self.last_rays = rays;
        self.brains.step_all(&sensor_inputs, dt);

        // Extract all motor outputs at once
        let entity_count = self.arena.entities.len();
        self.motors_scratch.clear();
        self.motors_scratch
            .resize(entity_count, MotorOutputs::default());

        for slot in 0..entity_count {
            if self.brains.active.get(slot).copied().unwrap_or(false) {
                self.motors_scratch[slot] = self.brains.motor_outputs(slot);
            }
        }

        self.attack_intents_scratch.clear();
        self.eat_intents_scratch.clear();
        self.pickup_intents_scratch.clear();
        self.share_intents_scratch.clear();
        self.reproduce_intents_scratch.clear();
        self.signal_colors_scratch.clear();
        self.attack_intents_scratch.reserve(entity_count);
        self.eat_intents_scratch.reserve(entity_count);
        self.pickup_intents_scratch.reserve(entity_count);
        self.share_intents_scratch.reserve(entity_count);
        self.reproduce_intents_scratch.reserve(entity_count);
        self.signal_colors_scratch.reserve(entity_count);
        for m in &self.motors_scratch {
            self.attack_intents_scratch.push(m.attack);
            self.eat_intents_scratch.push(m.eat);
            self.pickup_intents_scratch.push(m.pickup);
            self.share_intents_scratch.push(m.share);
            self.reproduce_intents_scratch.push(m.reproduce);
            self.signal_colors_scratch.push(m.signal_rgb);
        }

        // Physics
        physics::apply_motor_outputs(
            &mut self.arena,
            &self.motors_scratch,
            &self.environment.terrain,
            dt,
        );
        physics::integrate(&mut self.arena, &self.world, dt);
        self.spatial_hash.rebuild(&self.arena);
        physics::resolve_collisions(&mut self.arena, &self.spatial_hash, &self.world);
        physics::resolve_wall_collisions(&mut self.arena, &self.environment.walls, &self.world);

        // Combat
        self.combat_events = combat::resolve_combat(
            &mut self.arena,
            &self.attack_intents_scratch,
            &self.spatial_hash,
            &self.world,
            &mut self.meat,
        );

        // Emit combat particles
        for event in &self.combat_events {
            self.particles.emit_combat(event.target_pos);
        }

        // Meat consumption and decay
        combat::consume_meat(
            &mut self.arena,
            &mut self.meat,
            &self.world,
            &self.eat_intents_scratch,
            &self.pickup_intents_scratch,
        );
        combat::decay_meat(&mut self.meat, dt);

        // Energy: metabolism, food consumption, starvation
        energy::deduct_metabolism(&mut self.arena, dt);
        energy::digest_carried_energy(&mut self.arena, &self.eat_intents_scratch, dt);
        let (eaten_positions, picked_positions) = energy::consume_food(
            &mut self.arena,
            &mut self.food,
            &self.world,
            &self.eat_intents_scratch,
            &self.pickup_intents_scratch,
        );
        for pos in &eaten_positions {
            self.particles.emit_eat(*pos);
        }
        for pos in &picked_positions {
            self.particles.emit_eat(*pos);
        }
        energy::kill_starved(&mut self.arena);

        // Food sharing: entities with high signal and adjacent neighbor share energy
        self.process_food_sharing();

        // Signals and pheromones
        signals::update_signals(
            &self.arena,
            &self.signal_colors_scratch,
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
            &self.reproduce_intents_scratch,
        );
        for pos in &birth_positions {
            self.particles.emit_birth(*pos);
        }
        self.births_last_tick = birth_positions.len() as u32;

        // Sweep dead entities
        let dead = self.arena.sweep_dead();
        for (idx, pos) in &dead {
            self.brains.deactivate(*idx);
            if *idx < self.genomes.len() {
                self.genomes[*idx] = None;
            }
            self.particles.emit_death(*pos);
        }
        self.deaths_last_tick = dead.len() as u32;

        // Environment: terrain, storms, day/night, seasons
        environment::apply_terrain_effects(
            &mut self.arena,
            &self.environment.terrain,
            &self.world,
            dt,
        );
        environment::apply_toxic_zone_effects(
            &mut self.arena,
            &self.environment.toxic_zones,
            &self.world,
            dt,
        );
        if let Some(ref storm) = self.environment.storm {
            let storm_clone = storm.clone();
            environment::apply_storm_effects(
                &mut self.arena,
                &storm_clone,
                &self.world,
                &self.environment.terrain,
                &self.environment.walls,
                dt,
            );
        }
        self.environment.tick(dt, &self.world, &mut self.rng);
        energy::kill_starved(&mut self.arena);

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
        self.refresh_population_cache(false);
    }

    /// Food sharing: entities with high share intent transfer resources to nearby entities.
    fn process_food_sharing(&mut self) {
        let share_range = config::ATTACK_RANGE * 2.0; // slightly larger than attack range
        let share_amount = 5.0;

        // Collect sharing intents: (giver_idx, receiver_idx)
        let mut shares: Vec<(usize, usize)> = Vec::new();

        for (idx, entity) in self.arena.entities.iter().enumerate() {
            let entity = match entity {
                Some(e) => e,
                None => continue,
            };
            if !entity.alive {
                continue;
            }

            if idx >= self.share_intents_scratch.len() {
                continue;
            }
            if self.share_intents_scratch[idx] < config::SHARE_INTENT_THRESHOLD {
                continue;
            }

            // Must have enough energy to share
            if entity.energy + entity.carried_energy < share_amount * 2.0 {
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

            if let Some(neighbor_idx) =
                Self::nearest_alive_neighbor(entity.pos, &neighbors, &self.arena, &self.world)
            {
                shares.push((idx, neighbor_idx as usize));
            }
        }

        // Apply shares (two-pass to avoid double mutable borrow)
        for (giver, receiver) in shares {
            let can_give = self
                .arena
                .entities
                .get(giver)
                .and_then(|e| e.as_ref())
                .map(|e| e.alive && (e.energy + e.carried_energy) > share_amount * 2.0)
                .unwrap_or(false);
            if can_give {
                if let Some(Some(giver_e)) = self.arena.entities.get_mut(giver) {
                    let from_carried = giver_e.carried_energy.min(share_amount);
                    giver_e.carried_energy -= from_carried;
                    let remaining = share_amount - from_carried;
                    if remaining > 0.0 {
                        giver_e.energy -= remaining;
                    }
                }
                if let Some(Some(receiver_e)) = self.arena.entities.get_mut(receiver) {
                    if receiver_e.alive {
                        receiver_e.energy =
                            (receiver_e.energy + share_amount).min(config::MAX_ENTITY_ENERGY);
                    }
                }
            }
        }
    }

    fn nearest_alive_neighbor(
        pos: Vec2,
        neighbors: &[u32],
        arena: &EntityArena,
        world: &World,
    ) -> Option<u32> {
        let mut best: Option<(u32, f32)> = None;
        for &idx in neighbors {
            if let Some(candidate) = arena.get_by_index(idx as usize) {
                if !candidate.alive {
                    continue;
                }
                let dist_sq = world.distance_sq(pos, candidate.pos);
                match best {
                    Some((_, best_dist_sq)) if dist_sq >= best_dist_sq => {}
                    _ => best = Some((idx, dist_sq)),
                }
            }
        }
        best.map(|(idx, _)| idx)
    }

    fn refresh_position_caches(&mut self) {
        self.food_positions_scratch.clear();
        self.meat_positions_scratch.clear();
        self.food_positions_scratch.extend(self.food.iter().map(|f| f.pos));
        self.meat_positions_scratch.extend(self.meat.iter().map(|m| m.pos));
    }

    pub(crate) fn refresh_population_cache(&mut self, force_species_recalc: bool) {
        let mut total_energy = 0.0f32;
        let mut total_age = 0.0f32;
        let mut total_size = 0.0f32;
        let mut total_gen = 0.0f32;
        let mut count = 0u32;

        for (_idx, e) in self.arena.iter_alive() {
            total_energy += e.energy;
            total_age += e.age;
            total_size += e.radius / config::ENTITY_BASE_RADIUS;
            total_gen += e.generation_depth as f32;
            count += 1;
        }

        if count > 0 {
            let inv = 1.0 / count as f32;
            self.cached_avg_energy = total_energy * inv;
            self.cached_avg_age = total_age * inv;
            self.cached_avg_size = total_size * inv;
            self.cached_avg_generation = total_gen * inv;
        } else {
            self.cached_avg_energy = 0.0;
            self.cached_avg_age = 0.0;
            self.cached_avg_size = 0.0;
            self.cached_avg_generation = 0.0;
            self.cached_species_estimate = 0;
        }

        let should_recalc_species = force_species_recalc
            || self.tick_count.saturating_sub(self.cached_species_tick) >= 15
            || self.cached_species_estimate == 0;
        if should_recalc_species {
            self.cached_species_estimate = self.estimate_species_count();
            self.cached_species_tick = self.tick_count;
        }
    }

    fn estimate_species_count(&self) -> usize {
        let mut representatives: Vec<&[f32]> = Vec::new();
        let threshold = 0.17f32;

        for (idx, _e) in self.arena.iter_alive() {
            let genome = match self.genomes.get(idx).and_then(|g| g.as_ref()) {
                Some(g) => g,
                None => continue,
            };

            let is_known = representatives
                .iter()
                .any(|rep| genome_distance(rep, &genome.genes) < threshold);

            if !is_known {
                representatives.push(&genome.genes);
            }
        }

        representatives.len()
    }
}

fn genome_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let len = a.len().min(b.len());
    let step = (len / 64).max(1);
    let mut sum_sq = 0.0f32;
    let mut n = 0usize;

    let mut i = 0usize;
    while i < len {
        let d = a[i] - b[i];
        sum_sq += d * d;
        n += 1;
        i += step;
    }

    let shared_gene_rms = (sum_sq / n.max(1) as f32).sqrt();
    let normalized_length_diff =
        (a.len() as f32 - b.len() as f32).abs() / a.len().max(b.len()) as f32;

    shared_gene_rms + 0.25 * normalized_length_diff
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap_angle(angle: f32) -> f32 {
        (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
    }

    #[test]
    fn same_seed_produces_same_initial_entities() {
        let sim_a = SimState::new(5, 42);
        let sim_b = SimState::new(5, 42);

        let a_entities: Vec<(Vec2, f32)> = sim_a
            .arena
            .iter_alive()
            .map(|(_, e)| (e.pos, e.heading))
            .collect();
        let b_entities: Vec<(Vec2, f32)> = sim_b
            .arena
            .iter_alive()
            .map(|(_, e)| (e.pos, e.heading))
            .collect();

        assert_eq!(a_entities, b_entities);
        assert_eq!(sim_a.food.len(), sim_b.food.len());
    }

    #[test]
    fn different_seed_changes_initial_state() {
        let sim_a = SimState::new(5, 42);
        let sim_b = SimState::new(5, 43);

        let a_first = sim_a.arena.iter_alive().next().unwrap().1.pos;
        let b_first = sim_b.arena.iter_alive().next().unwrap().1.pos;

        assert_ne!(a_first, b_first);
    }

    #[test]
    fn tick_reuses_intent_scratch_buffers() {
        let mut sim = SimState::new(8, 11);
        sim.tick();
        let cap_after_first = sim.attack_intents_scratch.capacity();
        sim.tick();

        assert_eq!(sim.motors_scratch.len(), sim.arena.entities.len());
        assert_eq!(sim.attack_intents_scratch.len(), sim.arena.entities.len());
        assert_eq!(sim.signal_colors_scratch.len(), sim.arena.entities.len());
        assert!(sim.attack_intents_scratch.capacity() >= cap_after_first);
    }

    #[test]
    fn population_cache_tracks_alive_state() {
        let mut sim = SimState::new(5, 17);
        assert!(sim.cached_species_estimate > 0);
        for slot in sim.arena.entities.iter_mut() {
            if let Some(e) = slot {
                e.alive = false;
            }
        }
        sim.tick();
        assert_eq!(sim.arena.count, 0);
        assert_eq!(sim.cached_avg_energy, 0.0);
        assert_eq!(sim.cached_species_estimate, 0);
    }

    #[test]
    fn long_run_keeps_entity_positions_bounded_and_finite() {
        for seed in [5u64, 23u64] {
            let mut sim = SimState::new(70, seed);
            for _ in 0..180 {
                sim.tick();
                for (_idx, e) in sim.arena.iter_alive() {
                    assert!(e.pos.x.is_finite() && e.pos.y.is_finite());
                    assert!(e.pos.x >= 0.0 && e.pos.x <= sim.world.width);
                    assert!(e.pos.y >= 0.0 && e.pos.y <= sim.world.height);
                }
            }
        }
    }

    #[test]
    fn behavior_sweep_limits_rapid_turning_at_high_speed() {
        let mut sim = SimState::new(60, 44);
        let mut prev_headings: Vec<Option<f32>> = vec![None; sim.arena.entities.len()];
        let mut movement_samples = 0u64;
        let mut rapid_turn_samples = 0u64;

        for _ in 0..140 {
            sim.tick();
            if prev_headings.len() != sim.arena.entities.len() {
                prev_headings.resize(sim.arena.entities.len(), None);
            }
            for (idx, slot) in sim.arena.entities.iter().enumerate() {
                let entity = match slot {
                    Some(e) if e.alive => e,
                    _ => {
                        prev_headings[idx] = None;
                        continue;
                    }
                };

                if let Some(prev) = prev_headings[idx] {
                    let delta = wrap_angle(entity.heading - prev).abs();
                    let turn_rate = delta / config::FIXED_DT.max(1e-6);
                    let speed = entity.velocity.length();
                    let max_speed = config::ENTITY_MAX_SPEED * entity.speed_multiplier;
                    if speed > max_speed * 0.4 {
                        movement_samples += 1;
                        if turn_rate > config::ENTITY_TURN_RATE * 0.92 {
                            rapid_turn_samples += 1;
                        }
                    }
                }
                prev_headings[idx] = Some(entity.heading);
            }
        }

        let ratio = if movement_samples > 0 {
            rapid_turn_samples as f32 / movement_samples as f32
        } else {
            0.0
        };
        assert!(
            ratio <= 0.55,
            "rapid turning ratio too high: ratio={ratio:.3}, rapid={rapid_turn_samples}, samples={movement_samples}"
        );
    }
}
