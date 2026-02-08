use macroquad::prelude::*;

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

/// Resolve combat interactions. Entities with attack intent > 0.7 attack the nearest entity.
/// Returns list of combat events for visual effects.
pub fn resolve_combat(
    arena: &mut EntityArena,
    attack_intents: &[f32], // indexed by slot, [0,1]
    spatial: &SpatialHash,
    world: &World,
    meat: &mut Vec<MeatItem>,
) -> Vec<CombatEvent> {
    let attack_threshold = 0.7;
    let mut events = Vec::new();

    // Collect damage to apply (to avoid borrow conflicts)
    let mut damage_list: Vec<(usize, f32, Vec2, Vec2)> = Vec::new(); // (target_idx, damage, attacker_pos, target_pos)

    for (idx, entity) in arena.entities.iter().enumerate() {
        if let Some(e) = entity {
            if idx >= attack_intents.len() {
                continue;
            }
            let intent = attack_intents[idx];
            if intent < attack_threshold {
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

            if let Some(&target_idx) = neighbors.first() {
                if let Some(target) = arena.get_by_index(target_idx as usize) {
                    let damage = config::ATTACK_DAMAGE * (e.radius / config::ENTITY_BASE_RADIUS);
                    damage_list.push((target_idx as usize, damage, e.pos, target.pos));
                }
            }
        }
    }

    // Apply damage and deduct attacker energy cost
    for (target_idx, damage, attacker_pos, target_pos) in &damage_list {
        if let Some(target) = arena.get_mut_by_index(*target_idx) {
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

    // Deduct attack energy cost from attackers
    for (idx, entity) in arena.entities.iter().enumerate() {
        if let Some(_e) = entity {
            if idx < attack_intents.len() && attack_intents[idx] >= attack_threshold {
                // Mark for energy deduction
            }
        }
    }
    // Actually deduct (separate loop for borrow reasons)
    for (idx, entity) in arena.entities.iter_mut().enumerate() {
        if let Some(e) = entity {
            if idx < attack_intents.len() && attack_intents[idx] >= attack_threshold {
                e.energy -= config::ATTACK_COST;
            }
        }
    }

    events
}

/// Let entities eat nearby meat items.
pub fn consume_meat(arena: &mut EntityArena, meat: &mut Vec<MeatItem>, world: &World) {
    let pickup_radius = config::ENTITY_BASE_RADIUS * 2.5;
    let pickup_sq = pickup_radius * pickup_radius;

    meat.retain(|item| {
        for slot in arena.entities.iter_mut() {
            if let Some(e) = slot {
                let dist_sq = world.distance_sq(e.pos, item.pos);
                if dist_sq < pickup_sq {
                    e.energy = (e.energy + item.energy).min(config::MAX_ENTITY_ENERGY);
                    return false;
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
