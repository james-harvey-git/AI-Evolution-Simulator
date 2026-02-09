use macroquad::prelude::*;
use std::collections::HashSet;

use crate::config;
use crate::entity::EntityArena;
use crate::spatial_hash::SpatialHash;
use crate::world::World;

/// Meat item dropped when an entity dies from combat.
#[derive(Clone, Debug)]
pub struct MeatItem {
    pub pos: Vec2,
    pub energy: f32,
    pub decay_timer: f32,
}

/// Combat event for rendering effects.
#[derive(Clone, Debug)]
pub struct CombatEvent {
    pub attacker_pos: Vec2,
    pub target_pos: Vec2,
}

/// Resolve combat interactions. Entities with attack intent > threshold attack the nearest entity.
/// Returns list of combat events for visual effects.
pub fn resolve_combat(
    arena: &mut EntityArena,
    attack_intents: &[f32], // indexed by slot, [0,1]
    spatial: &SpatialHash,
    world: &World,
    meat: &mut Vec<MeatItem>,
) -> Vec<CombatEvent> {
    let attack_threshold = config::ATTACK_INTENT_THRESHOLD;
    let mut events = Vec::new();

    // Collect damage to apply (to avoid borrow conflicts).
    let mut damage_list: Vec<(usize, usize, f32, Vec2, Vec2)> = Vec::new();
    // (attacker_idx, target_idx, damage, attacker_pos, target_pos)
    let mut attackers_who_attacked: HashSet<usize> = HashSet::new();

    for (idx, entity) in arena.entities.iter().enumerate() {
        if let Some(e) = entity {
            if !e.alive || idx >= attack_intents.len() {
                continue;
            }
            if attack_intents[idx] < attack_threshold {
                continue;
            }

            // Find nearest entity within attack range
            let neighbors = spatial.query_radius_excluding(
                e.pos,
                config::ATTACK_RANGE + e.radius,
                idx as u32,
                world,
                arena,
            );

            if let Some(target_idx) = nearest_alive_neighbor(e.pos, &neighbors, arena, world) {
                if let Some(target) = arena.get_by_index(target_idx as usize) {
                    let damage = attack_damage(e.radius, e.energy);
                    attackers_who_attacked.insert(idx);
                    damage_list.push((idx, target_idx as usize, damage, e.pos, target.pos));
                }
            }
        }
    }

    // Apply damage. Dead targets are skipped, preventing duplicate meat drops.
    for (_attacker_idx, target_idx, damage, attacker_pos, target_pos) in &damage_list {
        if let Some(target) = arena.get_mut_by_index(*target_idx) {
            if !target.alive {
                continue;
            }

            target.health -= damage;
            target.energy -= damage * 0.5; // damage also drains energy

            events.push(CombatEvent {
                attacker_pos: *attacker_pos,
                target_pos: *target_pos,
            });

            if target.health <= 0.0 || target.energy <= 0.0 {
                target.alive = false;
                meat.push(MeatItem {
                    pos: target.pos,
                    energy: config::MEAT_ENERGY,
                    decay_timer: config::MEAT_DECAY_TIME,
                });
            }
        }
    }

    // Deduct attack energy cost from entities that actually attacked.
    for idx in attackers_who_attacked {
        if let Some(attacker) = arena.get_mut_by_index(idx) {
            if attacker.alive {
                attacker.energy -= config::ATTACK_COST;
            }
        }
    }

    events
}

/// Let entities eat nearby meat items.
pub fn consume_meat(
    arena: &mut EntityArena,
    meat: &mut Vec<MeatItem>,
    world: &World,
    eat_intents: &[f32],
    pickup_intents: &[f32],
) {
    let pickup_radius = config::ENTITY_BASE_RADIUS * 2.5;
    let pickup_sq = pickup_radius * pickup_radius;

    meat.retain(|item| {
        for (idx, slot) in arena.entities.iter_mut().enumerate() {
            if let Some(e) = slot {
                if !e.alive {
                    continue;
                }
                let dist_sq = world.distance_sq(e.pos, item.pos);
                if dist_sq < pickup_sq {
                    let eat_intent = eat_intents.get(idx).copied().unwrap_or(0.0);
                    let pickup_intent = pickup_intents.get(idx).copied().unwrap_or(0.0);

                    if eat_intent >= config::EAT_INTENT_THRESHOLD {
                        e.energy = (e.energy + item.energy).min(config::MAX_ENTITY_ENERGY);
                        return false;
                    }
                    if pickup_intent >= config::PICKUP_INTENT_THRESHOLD {
                        e.carried_energy =
                            (e.carried_energy + item.energy).min(config::MAX_CARRIED_ENERGY);
                        return false;
                    }
                }
            }
        }
        true
    });
}

/// Decay meat timers and remove expired meat.
pub fn decay_meat(meat: &mut Vec<MeatItem>, dt: f32) {
    for item in meat.iter_mut() {
        item.decay_timer -= dt;
    }
    meat.retain(|item| item.decay_timer > 0.0);
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

fn attack_damage(radius: f32, energy: f32) -> f32 {
    let size_mult = radius / config::ENTITY_BASE_RADIUS;
    let energy_frac = (energy / config::MAX_ENTITY_ENERGY).clamp(0.0, 1.0);
    let energy_mult =
        config::ATTACK_ENERGY_MIN_MULT + (1.0 - config::ATTACK_ENERGY_MIN_MULT) * energy_frac;
    config::ATTACK_DAMAGE * size_mult * energy_mult
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityArena};

    fn test_entity(pos: Vec2) -> Entity {
        Entity {
            pos,
            prev_pos: pos,
            velocity: Vec2::ZERO,
            heading: 0.0,
            radius: config::ENTITY_BASE_RADIUS,
            color: WHITE,
            energy: 100.0,
            health: 100.0,
            max_health: 100.0,
            age: 0.0,
            alive: true,
            speed_multiplier: 1.0,
            sensor_range: 1.0,
            metabolic_rate: 1.0,
            carried_energy: 0.0,
            generation_depth: 0,
            parent_id: None,
            offspring_count: 0,
            tick_born: 0,
        }
    }

    #[test]
    fn resolves_nearest_target_not_first_neighbor() {
        let world = World::new(500.0, 500.0, false);
        let mut arena = EntityArena::new(3);

        let attacker = arena.spawn(test_entity(vec2(100.0, 100.0))).unwrap();
        let far = arena.spawn(test_entity(vec2(114.0, 100.0))).unwrap();
        let near = arena.spawn(test_entity(vec2(106.0, 100.0))).unwrap();

        let mut spatial = SpatialHash::new(world.width, world.height, 64.0);
        spatial.rebuild(&arena);

        let mut attack_intents = vec![0.0; arena.entities.len()];
        attack_intents[attacker.index as usize] = 1.0;

        let mut meat = Vec::new();
        let _ = resolve_combat(&mut arena, &attack_intents, &spatial, &world, &mut meat);

        let near_health = arena.get_by_index(near.index as usize).unwrap().health;
        let far_health = arena.get_by_index(far.index as usize).unwrap().health;
        assert!(
            near_health < far_health,
            "nearest target should receive damage"
        );
    }

    #[test]
    fn target_drops_meat_once_when_focus_fired() {
        let world = World::new(500.0, 500.0, false);
        let mut arena = EntityArena::new(3);

        let a1 = arena.spawn(test_entity(vec2(100.0, 100.0))).unwrap();
        let a2 = arena.spawn(test_entity(vec2(140.0, 100.0))).unwrap();
        let victim = arena.spawn(test_entity(vec2(120.0, 100.0))).unwrap();

        {
            let v = arena.get_mut(victim).unwrap();
            v.health = 10.0;
            v.energy = 10.0;
        }

        let mut spatial = SpatialHash::new(world.width, world.height, 64.0);
        spatial.rebuild(&arena);

        let mut attack_intents = vec![0.0; arena.entities.len()];
        attack_intents[a1.index as usize] = 1.0;
        attack_intents[a2.index as usize] = 1.0;

        let mut meat = Vec::new();
        let _ = resolve_combat(&mut arena, &attack_intents, &spatial, &world, &mut meat);

        assert_eq!(
            meat.len(),
            1,
            "victim should generate exactly one meat drop"
        );
    }

    #[test]
    fn higher_energy_attackers_deal_more_damage() {
        let world = World::new(300.0, 300.0, false);

        let damage_for_energy = |attacker_energy: f32| -> f32 {
            let mut arena = EntityArena::new(2);
            let attacker = arena.spawn(test_entity(vec2(100.0, 100.0))).unwrap();
            let victim = arena.spawn(test_entity(vec2(110.0, 100.0))).unwrap();
            arena.get_mut(attacker).unwrap().energy = attacker_energy;

            let mut spatial = SpatialHash::new(world.width, world.height, 64.0);
            spatial.rebuild(&arena);

            let mut intents = vec![0.0; arena.entities.len()];
            intents[attacker.index as usize] = 1.0;

            let victim_health_before = arena.get(victim).unwrap().health;
            let mut meat = Vec::new();
            let _ = resolve_combat(&mut arena, &intents, &spatial, &world, &mut meat);
            let victim_health_after = arena.get(victim).unwrap().health;

            victim_health_before - victim_health_after
        };

        let low = damage_for_energy(10.0);
        let high = damage_for_energy(config::MAX_ENTITY_ENERGY);
        assert!(
            high > low,
            "attacker with higher energy should deal more damage"
        );
    }
}
