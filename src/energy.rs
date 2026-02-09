use ::rand::Rng;
use macroquad::prelude::*;

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
            if !entity.alive {
                continue;
            }
            let speed_frac = entity.velocity.length()
                / (config::ENTITY_MAX_SPEED * entity.speed_multiplier).max(1.0);
            let cost = (config::IDLE_METABOLIC_COST + config::MOVE_METABOLIC_COST * speed_frac)
                * entity.metabolic_rate;
            entity.energy -= cost * dt;
        }
    }
}

/// Convert carried reserves into metabolically usable energy.
pub fn digest_carried_energy(arena: &mut EntityArena, eat_intents: &[f32], dt: f32) {
    let digest_rate = 25.0;
    for (idx, slot) in arena.entities.iter_mut().enumerate() {
        if let Some(entity) = slot {
            if !entity.alive || idx >= eat_intents.len() {
                continue;
            }
            if eat_intents[idx] < config::EAT_INTENT_THRESHOLD {
                continue;
            }
            if entity.carried_energy <= 0.0 {
                continue;
            }

            let amount = (digest_rate * dt).min(entity.carried_energy);
            entity.carried_energy -= amount;
            entity.energy = (entity.energy + amount).min(config::MAX_ENTITY_ENERGY);
        }
    }
}

/// Let entities eat or pick up nearby food.
/// Returns (eaten_positions, picked_positions).
pub fn consume_food(
    arena: &mut EntityArena,
    food: &mut Vec<FoodItem>,
    world: &World,
    eat_intents: &[f32],
    pickup_intents: &[f32],
) -> (Vec<Vec2>, Vec<Vec2>) {
    let pickup_radius = config::ENTITY_BASE_RADIUS * 2.0;
    let pickup_radius_sq = pickup_radius * pickup_radius;
    let mut eaten_positions = Vec::new();
    let mut picked_positions = Vec::new();

    // For each food item, find the closest entity within range
    food.retain(|item| {
        let mut best_idx: Option<usize> = None;
        let mut best_dist_sq = pickup_radius_sq;

        for (idx, entity) in arena.entities.iter().enumerate() {
            if let Some(e) = entity {
                if !e.alive {
                    continue;
                }
                let dist_sq = world.distance_sq(e.pos, item.pos);
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = best_idx {
            if let Some(e) = &mut arena.entities[idx] {
                if !e.alive {
                    return true;
                }
                let eat_intent = eat_intents.get(idx).copied().unwrap_or(0.0);
                let pickup_intent = pickup_intents.get(idx).copied().unwrap_or(0.0);

                if eat_intent >= config::EAT_INTENT_THRESHOLD {
                    e.energy = (e.energy + item.energy).min(config::MAX_ENTITY_ENERGY);
                    eaten_positions.push(item.pos);
                    return false;
                }

                if pickup_intent >= config::PICKUP_INTENT_THRESHOLD {
                    e.carried_energy =
                        (e.carried_energy + item.energy).min(config::MAX_CARRIED_ENERGY);
                    picked_positions.push(item.pos);
                    return false;
                }
            }
        }
        true // not eaten
    });

    (eaten_positions, picked_positions)
}

/// Kill entities with no energy or exceeding max age.
pub fn kill_starved(arena: &mut EntityArena) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            if !entity.alive {
                continue;
            }
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
            pos: vec2(
                rng.gen_range(0.0..world.width),
                rng.gen_range(0.0..world.height),
            ),
            energy: config::FOOD_ENERGY,
        });
        spawner.accumulator -= 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityArena};
    use ::rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn test_entity(pos: Vec2) -> Entity {
        Entity {
            pos,
            prev_pos: pos,
            velocity: Vec2::ZERO,
            heading: 0.0,
            radius: config::ENTITY_BASE_RADIUS,
            color: WHITE,
            energy: 80.0,
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
    fn consume_food_prefers_eat_action_when_both_intents_high() {
        let world = World::new(200.0, 200.0, false);
        let mut arena = EntityArena::new(1);
        arena.spawn(test_entity(vec2(50.0, 50.0))).unwrap();

        let mut food = vec![FoodItem {
            pos: vec2(52.0, 50.0),
            energy: config::FOOD_ENERGY,
        }];
        let eat_intents = vec![1.0];
        let pickup_intents = vec![1.0];

        let (eaten, picked) =
            consume_food(&mut arena, &mut food, &world, &eat_intents, &pickup_intents);

        assert_eq!(food.len(), 0);
        assert_eq!(eaten.len(), 1);
        assert_eq!(picked.len(), 0);

        let entity = arena.get_by_index(0).unwrap();
        assert!(entity.energy > 80.0);
        assert_eq!(entity.carried_energy, 0.0);
    }

    #[test]
    fn consume_food_pickup_stores_energy_when_eat_not_triggered() {
        let world = World::new(200.0, 200.0, false);
        let mut arena = EntityArena::new(1);
        arena.spawn(test_entity(vec2(70.0, 70.0))).unwrap();

        let mut food = vec![FoodItem {
            pos: vec2(72.0, 70.0),
            energy: config::FOOD_ENERGY,
        }];
        let eat_intents = vec![0.0];
        let pickup_intents = vec![1.0];

        let (eaten, picked) =
            consume_food(&mut arena, &mut food, &world, &eat_intents, &pickup_intents);

        assert_eq!(food.len(), 0);
        assert_eq!(eaten.len(), 0);
        assert_eq!(picked.len(), 1);

        let entity = arena.get_by_index(0).unwrap();
        assert!((entity.carried_energy - config::FOOD_ENERGY).abs() < 1e-5);
    }

    #[test]
    fn digest_carried_energy_requires_eat_intent() {
        let mut arena = EntityArena::new(1);
        let id = arena.spawn(test_entity(vec2(40.0, 40.0))).unwrap();
        {
            let e = arena.get_mut(id).unwrap();
            e.energy = 30.0;
            e.carried_energy = 20.0;
        }

        digest_carried_energy(&mut arena, &[0.0], 0.5);
        {
            let e = arena.get(id).unwrap();
            assert!((e.energy - 30.0).abs() < 1e-5);
            assert!((e.carried_energy - 20.0).abs() < 1e-5);
        }

        digest_carried_energy(&mut arena, &[1.0], 0.5);
        let e = arena.get(id).unwrap();
        assert!(e.energy > 30.0);
        assert!(e.carried_energy < 20.0);
    }

    #[test]
    fn respawn_food_adds_items_over_time() {
        let world = World::new(300.0, 300.0, false);
        let mut rng = ChaCha8Rng::seed_from_u64(12);
        let mut food = Vec::new();
        let mut spawner = FoodSpawner::new();

        respawn_food(&mut food, &mut spawner, &world, &mut rng, 2.0);

        assert!(!food.is_empty());
        assert!(food.iter().all(|item| item.pos.x >= 0.0
            && item.pos.x <= world.width
            && item.pos.y >= 0.0
            && item.pos.y <= world.height));
    }
}
