use macroquad::prelude::*;

use crate::config;
use crate::entity::EntityArena;
use crate::spatial_hash::SpatialHash;
use crate::world::World;

/// Apply random wander movement (Phase 1 placeholder — replaced by brain output in Phase 2).
pub fn random_wander(arena: &mut EntityArena, rng: &mut impl ::rand::Rng, dt: f32) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            // Random turn
            entity.heading += rng.gen_range(-1.5..1.5) * dt;

            // Constant forward drive
            let dir = Vec2::from_angle(entity.heading);
            let target_vel = dir * config::ENTITY_MAX_SPEED * 0.5 * entity.speed_multiplier;

            // Smooth velocity toward target (simple friction model)
            entity.velocity +=
                (target_vel - entity.velocity) * (config::ENTITY_FRICTION * dt).min(1.0);
        }
    }
}

/// Apply brain-driven motor outputs to entity movement.
pub fn apply_motor_outputs(
    arena: &mut EntityArena,
    motor_outputs: &[(f32, f32)], // (forward_drive [0,1], turn [-1,1]) indexed by slot
    dt: f32,
) {
    for (idx, slot) in arena.entities.iter_mut().enumerate() {
        if let Some(entity) = slot {
            if idx < motor_outputs.len() {
                let (forward, turn) = motor_outputs[idx];

                // Turn
                entity.heading += turn * config::ENTITY_TURN_RATE * dt;

                // Forward drive
                let dir = Vec2::from_angle(entity.heading);
                let max_speed = config::ENTITY_MAX_SPEED * entity.speed_multiplier;
                let target_vel = dir * forward * max_speed;

                entity.velocity +=
                    (target_vel - entity.velocity) * (config::ENTITY_FRICTION * dt).min(1.0);
            }
        }
    }
}

/// Integrate positions from velocities and wrap to world bounds.
pub fn integrate(arena: &mut EntityArena, world: &World, dt: f32) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            entity.prev_pos = entity.pos;
            entity.pos += entity.velocity * dt;
            entity.pos = world.wrap(entity.pos);
            entity.age += dt;
        }
    }
}

/// Resolve entity-entity overlaps by pushing them apart.
pub fn resolve_collisions(arena: &mut EntityArena, spatial: &SpatialHash, world: &World) {
    let max_radius = config::ENTITY_BASE_RADIUS * 2.0;
    let query_radius = max_radius * 2.5;

    // Collect positions and radii first to avoid borrow conflicts
    let entity_data: Vec<Option<(Vec2, f32)>> = arena
        .entities
        .iter()
        .map(|slot| slot.as_ref().map(|e| (e.pos, e.radius)))
        .collect();

    for (idx_a, slot_a) in entity_data.iter().enumerate() {
        if let Some((pos_a, radius_a)) = slot_a {
            let neighbors =
                spatial.query_radius_excluding(*pos_a, query_radius, idx_a as u32, world, arena);

            for idx_b in neighbors {
                let idx_b = idx_b as usize;
                if idx_b <= idx_a {
                    continue; // avoid double-processing pairs
                }
                if let Some((pos_b, radius_b)) = entity_data[idx_b] {
                    let delta = world.delta(*pos_a, pos_b);
                    let dist_sq = delta.length_squared();
                    let min_dist = radius_a + radius_b;

                    if dist_sq < min_dist * min_dist && dist_sq > 0.001 {
                        let dist = dist_sq.sqrt();
                        let overlap = min_dist - dist;
                        let push = delta / dist * (overlap * 0.5);

                        if let Some(ea) = arena.get_mut_by_index(idx_a) {
                            ea.pos = world.wrap(ea.pos - push);
                        }
                        if let Some(eb) = arena.get_mut_by_index(idx_b) {
                            eb.pos = world.wrap(eb.pos + push);
                        }
                    }
                }
            }
        }
    }
}
