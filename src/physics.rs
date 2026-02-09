use macroquad::prelude::*;

use crate::brain::MotorOutputs;
use crate::config;
use crate::entity::EntityArena;
use crate::environment::{closest_point_on_segment, TerrainGrid, WallSegment};
use crate::spatial_hash::SpatialHash;
use crate::world::World;

#[inline]
fn mass_from_radius(radius: f32) -> f32 {
    (radius * radius).max(1.0)
}

#[inline]
fn wrap_angle(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

#[inline]
fn turn_agility(speed: f32, max_speed: f32) -> f32 {
    if max_speed <= 0.001 {
        return 1.0;
    }
    let speed_frac = (speed / max_speed).clamp(0.0, 1.0);
    1.0 - (1.0 - config::ENTITY_TURN_AT_MAX_SPEED_FACTOR) * speed_frac
}

/// Apply random wander movement (Phase 1 placeholder — replaced by brain output in Phase 2).
pub fn random_wander(arena: &mut EntityArena, rng: &mut impl ::rand::Rng, dt: f32) {
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            if !entity.alive {
                continue;
            }
            // Random turn
            entity.heading = wrap_angle(entity.heading + rng.gen_range(-1.5..1.5) * dt);

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
    motor_outputs: &[MotorOutputs],
    terrain: &TerrainGrid,
    dt: f32,
) {
    for (idx, slot) in arena.entities.iter_mut().enumerate() {
        if let Some(entity) = slot {
            if !entity.alive {
                continue;
            }
            if idx < motor_outputs.len() {
                let outputs = motor_outputs[idx];

                // Turn
                let max_speed = config::ENTITY_MAX_SPEED * entity.speed_multiplier;
                let agility = turn_agility(entity.velocity.length(), max_speed);
                let turn = outputs.turn.clamp(-1.0, 1.0) * config::ENTITY_TURN_RATE * agility;
                entity.heading = wrap_angle(entity.heading + turn * dt);

                // Forward drive
                let dir = Vec2::from_angle(entity.heading);
                let terrain_mult = terrain.get_at(entity.pos).friction_mult();
                let target_vel = dir * outputs.forward * max_speed * terrain_mult;

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
            if !entity.alive {
                continue;
            }
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

    // Collect positions, radii, and masses first to avoid borrow conflicts.
    let entity_data: Vec<Option<(Vec2, f32, f32)>> = arena
        .entities
        .iter()
        .map(|slot| {
            slot.as_ref().and_then(|e| {
                if e.alive {
                    Some((e.pos, e.radius, mass_from_radius(e.radius)))
                } else {
                    None
                }
            })
        })
        .collect();

    for (idx_a, slot_a) in entity_data.iter().enumerate() {
        if let Some((pos_a, radius_a, mass_a)) = slot_a {
            let neighbors =
                spatial.query_radius_excluding(*pos_a, query_radius, idx_a as u32, world, arena);

            for idx_b in neighbors {
                let idx_b = idx_b as usize;
                if idx_b <= idx_a {
                    continue; // avoid double-processing pairs
                }
                if let Some((pos_b, radius_b, mass_b)) = entity_data[idx_b] {
                    let delta = world.delta(*pos_a, pos_b);
                    let dist_sq = delta.length_squared();
                    let min_dist = radius_a + radius_b;

                    if dist_sq < min_dist * min_dist && dist_sq > 0.001 {
                        let dist = dist_sq.sqrt();
                        let overlap = min_dist - dist;
                        let push_dir = delta / dist;
                        let total_mass = mass_a + mass_b;
                        let move_a = overlap * (mass_b / total_mass);
                        let move_b = overlap * (mass_a / total_mass);

                        if let Some(ea) = arena.get_mut_by_index(idx_a) {
                            ea.pos = world.wrap(ea.pos - push_dir * move_a);
                        }
                        if let Some(eb) = arena.get_mut_by_index(idx_b) {
                            eb.pos = world.wrap(eb.pos + push_dir * move_b);
                        }
                    }
                }
            }
        }
    }
}

pub fn resolve_wall_collisions(arena: &mut EntityArena, walls: &[WallSegment], world: &World) {
    if walls.is_empty() {
        return;
    }

    let wall_thickness = config::WALL_THICKNESS * 0.5;
    for slot in arena.entities.iter_mut() {
        if let Some(entity) = slot {
            if !entity.alive {
                continue;
            }

            for wall in walls {
                let cp = closest_point_on_segment(wall.start, wall.end, entity.pos);
                let delta = world.delta(cp, entity.pos);
                let dist_sq = delta.length_squared();
                let min_dist = entity.radius + wall_thickness;

                if dist_sq < min_dist * min_dist {
                    let dist = dist_sq.sqrt().max(0.0001);
                    let push_dir = delta / dist;
                    let overlap = min_dist - dist;
                    entity.pos = world.wrap(entity.pos + push_dir * overlap);

                    let vn = entity.velocity.dot(push_dir);
                    if vn < 0.0 {
                        entity.velocity -= push_dir * vn * 1.5;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;
    use crate::environment::{TerrainGrid, TerrainType};
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

    fn plains_terrain(width: f32, height: f32) -> TerrainGrid {
        let mut terrain = TerrainGrid::generate(width, height, width.max(height), 7);
        terrain.cells.fill(TerrainType::Plains);
        terrain
    }

    #[test]
    fn random_wander_updates_velocity() {
        let mut arena = EntityArena::new(1);
        arena.spawn(test_entity(vec2(30.0, 30.0))).unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(21);

        random_wander(&mut arena, &mut rng, 0.5);
        let entity = arena.get_by_index(0).unwrap();

        assert!(entity.velocity.length_squared() > 0.0);
    }

    #[test]
    fn wrap_angle_keeps_heading_in_stable_range() {
        let wrapped = wrap_angle(27.0 * std::f32::consts::PI);
        assert!(wrapped >= -std::f32::consts::PI);
        assert!(wrapped <= std::f32::consts::PI);
    }

    #[test]
    fn turn_agility_reduces_with_speed() {
        let max_speed = config::ENTITY_MAX_SPEED;
        let slow = turn_agility(max_speed * 0.1, max_speed);
        let fast = turn_agility(max_speed, max_speed);
        assert!(fast < slow);
        assert!((fast - config::ENTITY_TURN_AT_MAX_SPEED_FACTOR).abs() < 1e-6);
    }

    #[test]
    fn integrate_wraps_large_displacements_back_into_world() {
        let world = World::new(100.0, 100.0, true);
        let mut arena = EntityArena::new(1);
        let mut entity = test_entity(vec2(98.0, 2.0));
        entity.velocity = vec2(500.0, -420.0);
        arena.spawn(entity).unwrap();

        integrate(&mut arena, &world, 0.5);
        let wrapped = arena.get_by_index(0).unwrap();
        assert!((0.0..=world.width).contains(&wrapped.pos.x));
        assert!((0.0..=world.height).contains(&wrapped.pos.y));
    }

    #[test]
    fn full_forward_plus_turn_does_not_collapse_to_tiny_spin_orbit() {
        let world = World::new(500.0, 500.0, true);
        let terrain = plains_terrain(world.width, world.height);
        let mut arena = EntityArena::new(1);
        let start = vec2(250.0, 250.0);
        arena.spawn(test_entity(start)).unwrap();
        let motors = vec![MotorOutputs {
            forward: 1.0,
            turn: 1.0,
            ..Default::default()
        }];

        for _ in 0..240 {
            apply_motor_outputs(&mut arena, &motors, &terrain, config::FIXED_DT);
            integrate(&mut arena, &world, config::FIXED_DT);
        }

        let entity = arena.get_by_index(0).unwrap();
        let displacement = world.distance(start, entity.pos);
        assert!(
            displacement > 20.0,
            "entity remained in a tight spin orbit (displacement={displacement})"
        );
        assert!((0.0..=world.width).contains(&entity.pos.x));
        assert!((0.0..=world.height).contains(&entity.pos.y));
    }

    #[test]
    fn collision_pushes_smaller_entity_more_than_larger_entity() {
        let world = World::new(300.0, 300.0, false);
        let mut arena = EntityArena::new(2);

        let mut small = test_entity(vec2(100.0, 100.0));
        small.radius = 6.0;
        let mut large = test_entity(vec2(110.0, 100.0));
        large.radius = 14.0;

        let small_id = arena.spawn(small).unwrap();
        let large_id = arena.spawn(large).unwrap();
        let small_before = arena.get(small_id).unwrap().pos;
        let large_before = arena.get(large_id).unwrap().pos;

        let mut spatial = SpatialHash::new(world.width, world.height, 64.0);
        spatial.rebuild(&arena);
        resolve_collisions(&mut arena, &spatial, &world);

        let small_after = arena.get(small_id).unwrap().pos;
        let large_after = arena.get(large_id).unwrap().pos;

        let small_move = world.distance_sq(small_before, small_after).sqrt();
        let large_move = world.distance_sq(large_before, large_after).sqrt();
        assert!(small_move > large_move);
    }
}
