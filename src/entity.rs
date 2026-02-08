use macroquad::prelude::*;

/// Stable handle to an entity. The generation field invalidates stale references.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EntityId {
    pub index: u32,
    pub generation: u32,
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub velocity: Vec2,
    pub heading: f32,
    pub radius: f32,
    pub color: Color,
    pub energy: f32,
    pub health: f32,
    pub max_health: f32,
    pub age: f32,
    pub alive: bool,
    pub speed_multiplier: f32,
    pub sensor_range: f32,
    pub metabolic_rate: f32,
    pub generation_depth: u32,
    pub parent_id: Option<EntityId>,
    pub offspring_count: u32,
    pub tick_born: u64,
}

impl Entity {
    /// Create an entity from a genome at a given position.
    pub fn new_from_genome(genome: &crate::genome::Genome, pos: Vec2, tick: u64) -> Self {
        let size = genome.body_size();
        let max_health = 80.0 + size * 40.0; // larger = more HP
        Self {
            pos,
            prev_pos: pos,
            velocity: Vec2::ZERO,
            heading: 0.0,
            radius: crate::config::ENTITY_BASE_RADIUS * size,
            color: genome.body_color(),
            energy: crate::config::INITIAL_ENTITY_ENERGY,
            health: max_health,
            max_health,
            age: 0.0,
            alive: true,
            speed_multiplier: genome.max_speed(),
            sensor_range: genome.sensor_range(),
            metabolic_rate: genome.metabolic_rate(),
            generation_depth: 0,
            parent_id: None,
            offspring_count: 0,
            tick_born: tick,
        }
    }

    /// Create with a random heading.
    pub fn new_from_genome_rng(
        genome: &crate::genome::Genome,
        pos: Vec2,
        tick: u64,
        rng: &mut impl ::rand::Rng,
    ) -> Self {
        let mut e = Self::new_from_genome(genome, pos, tick);
        e.heading = rng.gen_range(0.0..std::f32::consts::TAU);
        e
    }
}

/// Arena-based entity storage with generational indices and free list.
pub struct EntityArena {
    pub entities: Vec<Option<Entity>>,
    pub generations: Vec<u32>,
    pub free_list: Vec<u32>,
    pub count: usize,
}

impl EntityArena {
    pub fn new(capacity: usize) -> Self {
        Self {
            entities: vec![None; capacity],
            generations: vec![0; capacity],
            free_list: (0..capacity as u32).rev().collect(),
            count: 0,
        }
    }

    pub fn spawn(&mut self, entity: Entity) -> Option<EntityId> {
        if let Some(index) = self.free_list.pop() {
            let idx = index as usize;
            self.entities[idx] = Some(entity);
            self.count += 1;
            Some(EntityId {
                index,
                generation: self.generations[idx],
            })
        } else {
            // Grow the arena
            let index = self.entities.len() as u32;
            self.entities.push(Some(entity));
            self.generations.push(0);
            self.count += 1;
            Some(EntityId {
                index,
                generation: 0,
            })
        }
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        let idx = id.index as usize;
        if idx < self.entities.len()
            && self.generations[idx] == id.generation
            && self.entities[idx].is_some()
        {
            self.entities[idx] = None;
            self.generations[idx] += 1;
            self.free_list.push(id.index);
            self.count -= 1;
            true
        } else {
            false
        }
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        let idx = id.index as usize;
        if idx < self.entities.len() && self.generations[idx] == id.generation {
            self.entities[idx].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        let idx = id.index as usize;
        if idx < self.entities.len() && self.generations[idx] == id.generation {
            self.entities[idx].as_mut()
        } else {
            None
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Entity> {
        self.entities.get(index).and_then(|e| e.as_ref())
    }

    pub fn get_mut_by_index(&mut self, index: usize) -> Option<&mut Entity> {
        self.entities.get_mut(index).and_then(|e| e.as_mut())
    }

    /// Remove dead entities and reclaim their slots.
    pub fn sweep_dead(&mut self) -> Vec<(usize, Vec2)> {
        let mut dead_positions = Vec::new();
        for (idx, slot) in self.entities.iter_mut().enumerate() {
            if let Some(entity) = slot {
                if !entity.alive {
                    dead_positions.push((idx, entity.pos));
                    *slot = None;
                    self.generations[idx] += 1;
                    self.free_list.push(idx as u32);
                    self.count -= 1;
                }
            }
        }
        dead_positions
    }

    /// Iterate over (index, &Entity) for all alive entities.
    pub fn iter_alive(&self) -> impl Iterator<Item = (usize, &Entity)> {
        self.entities
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|e| (i, e)))
    }

    pub fn capacity(&self) -> usize {
        self.entities.len()
    }
}
