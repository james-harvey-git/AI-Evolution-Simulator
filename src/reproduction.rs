use ::rand::Rng;
use macroquad::prelude::*;

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
    reproduce_intents: &[f32],
) -> Vec<Vec2> {
    let mut birth_positions = Vec::new();

    if arena.count >= config::MAX_ENTITY_COUNT {
        return birth_positions;
    }

    // Collect birth events
    let mut births: Vec<Birth> = Vec::new();

    for (idx, entity) in arena.entities.iter().enumerate() {
        if let Some(e) = entity {
            if !e.alive {
                continue;
            }
            if reproduce_intents.get(idx).copied().unwrap_or(0.0)
                < config::REPRODUCTION_INTENT_THRESHOLD
            {
                continue;
            }
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

        let mut child =
            Entity::new_from_genome_rng(&birth.child_genome, birth.child_pos, tick, rng);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::BrainStorage;
    use crate::world::World;
    use ::rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn reproduction_requires_intent_threshold() {
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let world = World::new(500.0, 500.0, true);
        let mut arena = EntityArena::new(2);
        let mut brains = BrainStorage::new(2);
        let mut genomes: Vec<Option<Genome>> = vec![None; 2];

        let genome = Genome::random(&mut rng);
        let mut parent = Entity::new_from_genome_rng(&genome, vec2(100.0, 100.0), 0, &mut rng);
        parent.energy = config::REPRODUCTION_THRESHOLD + 20.0;

        let parent_id = arena.spawn(parent).unwrap();
        let parent_idx = parent_id.index as usize;
        brains.init_from_genome(parent_idx, &genome);
        genomes[parent_idx] = Some(genome);

        let intents = vec![config::REPRODUCTION_INTENT_THRESHOLD - 0.01, 0.0];
        let births = check_and_spawn(
            &mut arena,
            &mut brains,
            &mut genomes,
            &world,
            &mut rng,
            1,
            &intents,
        );

        assert!(births.is_empty());
        assert_eq!(arena.count, 1);
    }

    #[test]
    fn reproduction_spawns_child_and_deducts_parent_energy() {
        let mut rng = ChaCha8Rng::seed_from_u64(9);
        let world = World::new(500.0, 500.0, true);
        let mut arena = EntityArena::new(4);
        let mut brains = BrainStorage::new(4);
        let mut genomes: Vec<Option<Genome>> = vec![None; 4];

        let genome = Genome::random(&mut rng);
        let mut parent = Entity::new_from_genome_rng(&genome, vec2(120.0, 220.0), 0, &mut rng);
        parent.energy = config::REPRODUCTION_THRESHOLD + 30.0;

        let parent_id = arena.spawn(parent).unwrap();
        let parent_idx = parent_id.index as usize;
        brains.init_from_genome(parent_idx, &genome);
        genomes[parent_idx] = Some(genome);

        let parent_energy_before = arena.get(parent_id).unwrap().energy;
        let intents = vec![1.0; arena.entities.len()];
        let births = check_and_spawn(
            &mut arena,
            &mut brains,
            &mut genomes,
            &world,
            &mut rng,
            42,
            &intents,
        );

        assert_eq!(births.len(), 1);
        assert_eq!(arena.count, 2);

        let parent_after = arena.get(parent_id).unwrap();
        assert!(
            (parent_after.energy - (parent_energy_before - config::REPRODUCTION_COST)).abs() < 1e-5
        );
        assert_eq!(parent_after.offspring_count, 1);

        let child = arena
            .iter_alive()
            .find(|(idx, _)| *idx != parent_idx)
            .map(|(_, e)| e)
            .unwrap();
        assert_eq!(child.generation_depth, parent_after.generation_depth + 1);
        assert_eq!(child.parent_id, Some(parent_id));
    }
}
