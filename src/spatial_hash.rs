use macroquad::prelude::*;

use crate::entity::EntityArena;
use crate::world::World;

pub struct SpatialHash {
    inv_cell_size: f32,
    pub cols: usize,
    pub rows: usize,
    cells: Vec<Vec<u32>>,
}

impl SpatialHash {
    pub fn new(world_w: f32, world_h: f32, cell_size: f32) -> Self {
        let cols = (world_w / cell_size).ceil() as usize;
        let rows = (world_h / cell_size).ceil() as usize;
        let cells = (0..cols * rows).map(|_| Vec::with_capacity(8)).collect();
        Self {
            inv_cell_size: 1.0 / cell_size,
            cols,
            rows,
            cells,
        }
    }

    /// Clear all cells and re-insert all alive entities.
    pub fn rebuild(&mut self, arena: &EntityArena) {
        for cell in &mut self.cells {
            cell.clear();
        }
        for (idx, entity) in arena.entities.iter().enumerate() {
            if let Some(e) = entity {
                if !e.alive {
                    continue;
                }
                let cx = ((e.pos.x * self.inv_cell_size) as usize).min(self.cols - 1);
                let cy = ((e.pos.y * self.inv_cell_size) as usize).min(self.rows - 1);
                self.cells[cy * self.cols + cx].push(idx as u32);
            }
        }
    }

    /// Query all entity indices within `radius` of `pos`.
    pub fn query_radius(
        &self,
        pos: Vec2,
        radius: f32,
        world: &World,
        arena: &EntityArena,
    ) -> Vec<u32> {
        let mut result = Vec::new();
        let radius_sq = radius * radius;

        // Determine cell range to check
        let cells_range = (radius * self.inv_cell_size).ceil() as i32 + 1;

        let cx = (pos.x * self.inv_cell_size) as i32;
        let cy = (pos.y * self.inv_cell_size) as i32;

        for dy in -cells_range..=cells_range {
            for dx in -cells_range..=cells_range {
                let mut gx = cx + dx;
                let mut gy = cy + dy;

                if world.toroidal {
                    gx = gx.rem_euclid(self.cols as i32);
                    gy = gy.rem_euclid(self.rows as i32);
                } else {
                    if gx < 0 || gx >= self.cols as i32 || gy < 0 || gy >= self.rows as i32 {
                        continue;
                    }
                }

                let cell_idx = gy as usize * self.cols + gx as usize;
                for &entity_idx in &self.cells[cell_idx] {
                    if let Some(e) = arena.get_by_index(entity_idx as usize) {
                        if !e.alive {
                            continue;
                        }
                        let dist_sq = world.distance_sq(pos, e.pos);
                        if dist_sq <= radius_sq {
                            result.push(entity_idx);
                        }
                    }
                }
            }
        }

        result
    }

    /// Query all entity indices within `radius` of `pos`, excluding a specific index.
    pub fn query_radius_excluding(
        &self,
        pos: Vec2,
        radius: f32,
        exclude_idx: u32,
        world: &World,
        arena: &EntityArena,
    ) -> Vec<u32> {
        let mut result = self.query_radius(pos, radius, world, arena);
        result.retain(|&idx| idx != exclude_idx);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;

    fn test_entity(pos: Vec2, alive: bool) -> Entity {
        Entity {
            pos,
            prev_pos: pos,
            velocity: Vec2::ZERO,
            heading: 0.0,
            radius: 8.0,
            color: WHITE,
            energy: 100.0,
            health: 100.0,
            max_health: 100.0,
            age: 0.0,
            alive,
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
    fn dead_entities_are_not_returned_from_queries() {
        let world = World::new(200.0, 200.0, false);
        let mut arena = EntityArena::new(2);
        arena.spawn(test_entity(vec2(50.0, 50.0), true)).unwrap();
        arena.spawn(test_entity(vec2(55.0, 50.0), false)).unwrap();

        let mut spatial = SpatialHash::new(world.width, world.height, 32.0);
        spatial.rebuild(&arena);

        let neighbors = spatial.query_radius(vec2(50.0, 50.0), 20.0, &world, &arena);
        assert_eq!(neighbors.len(), 1);
    }
}
