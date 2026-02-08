use macroquad::prelude::*;
use ::rand::Rng;

use crate::config;
use crate::entity::EntityArena;
use crate::simulation::FoodItem;
use crate::world::World;

/// Accumulator for fractional food spawning.
pub struct FoodSpawner {
    pub accumulator: f32,
}

impl FoodSpawner {
    pub fn new() -> Self {
        Self { accumulator: 0.0 }
    }
}

/// Deduct metabolic costs from all alive entities.
pub fn deduct_metabolism(arena: &mut EntityArena, dt: f32) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            let speed_frac = entity.velocity.length()
                / (config::ENTITY_MAX_SPEED * entity.speed_multiplier).max(1.0);
            let cost = (config::IDLE_METABOLIC_COST
                + config::MOVE_METABOLIC_COST * speed_frac)
                * entity.metabolic_rate;
            entity.energy -= cost * dt;
        }
    }
}

/// Let entities eat nearby food. Returns positions of eaten food items.
pub fn consume_food(arena: &mut EntityArena, food: &mut Vec<FoodItem>, world: &World) -> Vec<Vec2> {
    let pickup_radius = config::ENTITY_BASE_RADIUS * 2.0;
    let pickup_radius_sq = pickup_radius * pickup_radius;
    let mut eaten_positions = Vec::new();

    // For each food item, find the closest entity within range
    food.retain(|item| {
        let mut best_idx: Option<usize> = None;
        let mut best_dist_sq = pickup_radius_sq;

        for (idx, entity) in arena.entities.iter().enumerate() {
            if let Some(e) = entity {
                let dist_sq = world.distance_sq(e.pos, item.pos);
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = best_idx {
            if let Some(e) = &mut arena.entities[idx] {
                e.energy = (e.energy + item.energy).min(config::MAX_ENTITY_ENERGY);
                eaten_positions.push(item.pos);
                return false; // consumed
            }
        }
        true // not eaten
    });

    eaten_positions
}

/// Kill entities with no energy or exceeding max age.
pub fn kill_starved(arena: &mut EntityArena) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            if entity.energy <= 0.0 || entity.age > config::DEATH_AGE {
                entity.alive = false;
            }
        }
    }
}

/// Respawn food up to a maximum amount.
pub fn respawn_food(
    food: &mut Vec<FoodItem>,
    spawner: &mut FoodSpawner,
    world: &World,
    rng: &mut impl Rng,
    dt: f32,
) {
    let max_food = config::INITIAL_FOOD_COUNT * 2;
    spawner.accumulator += config::FOOD_RESPAWN_RATE * dt;

    while spawner.accumulator >= 1.0 && food.len() < max_food {
        food.push(FoodItem {
            pos: vec2(rng.gen_range(0.0..world.width), rng.gen_range(0.0..world.height)),
            energy: config::FOOD_ENERGY,
        });
        spawner.accumulator -= 1.0;
    }
}
