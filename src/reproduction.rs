use macroquad::prelude::*;
use ::rand::Rng;

use crate::brain::BrainStorage;
use crate::config;
use crate::entity::{Entity, EntityArena, EntityId};
use crate::genome::Genome;
use crate::world::World;

/// Pending birth record (to avoid borrow conflicts during iteration).
struct Birth {
    parent_idx: usize,
    child_pos: Vec2,
    child_genome: Genome,
    parent_generation_depth: u32,
    parent_id: EntityId,
}

/// Check all entities for reproduction eligibility and spawn offspring.
/// Returns positions of newly born entities.
pub fn check_and_spawn(
    arena: &mut EntityArena,
    brains: &mut BrainStorage,
    genomes: &mut Vec<Option<Genome>>,
    world: &World,
    rng: &mut impl Rng,
    tick: u64,
) -> Vec<Vec2> {
    let mut birth_positions = Vec::new();

    if arena.count >= config::MAX_ENTITY_COUNT {
        return birth_positions;
    }

    // Collect birth events
    let mut births: Vec<Birth> = Vec::new();

    for (idx, entity) in arena.entities.iter().enumerate() {
        if let Some(e) = entity {
            if e.energy < config::REPRODUCTION_THRESHOLD {
                continue;
            }
            if arena.count + births.len() >= config::MAX_ENTITY_COUNT {
                break;
            }

            if let Some(ref genome) = genomes[idx] {
                let child_genome = genome.mutate(rng);
                let offset_angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let offset_dist = e.radius * 3.0;
                let child_pos = world.wrap(e.pos + Vec2::from_angle(offset_angle) * offset_dist);

                births.push(Birth {
                    parent_idx: idx,
                    child_pos,
                    child_genome,
                    parent_generation_depth: e.generation_depth,
                    parent_id: EntityId {
                        index: idx as u32,
                        generation: arena.generations[idx],
                    },
                });
            }
        }
    }

    // Deduct energy from parents and spawn children
    for birth in births {
        if let Some(parent) = &mut arena.entities[birth.parent_idx] {
            parent.energy -= config::REPRODUCTION_COST;
            parent.offspring_count += 1;
        }

        let mut child = Entity::new_from_genome_rng(&birth.child_genome, birth.child_pos, tick, rng);
        child.energy = config::INITIAL_ENTITY_ENERGY * config::OFFSPRING_ENERGY_FRACTION;
        child.generation_depth = birth.parent_generation_depth + 1;
        child.parent_id = Some(birth.parent_id);

        if let Some(id) = arena.spawn(child) {
            let slot = id.index as usize;
            brains.init_from_genome(slot, &birth.child_genome);

            // Ensure genomes vec is large enough
            if slot >= genomes.len() {
                genomes.resize(slot + 1, None);
            }
            genomes[slot] = Some(birth.child_genome);
            birth_positions.push(birth.child_pos);
        }
    }

    birth_positions
}
