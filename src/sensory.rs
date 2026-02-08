use macroquad::prelude::*;

use crate::config;
use crate::entity::EntityArena;
use crate::environment::{EnvironmentState, TerrainType};
use crate::spatial_hash::SpatialHash;
use crate::world::World;

/// What a sensor ray hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitType {
    Nothing,
    Entity,
    Food,
    Wall,
}

/// Result of a single raycast.
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub distance_norm: f32, // [0, 1] where 0 = at origin, 1 = max range (nothing hit)
    pub hit_type: HitType,
}

/// Per-entity ray data for visualization.
#[derive(Clone, Debug)]
pub struct EntityRays {
    pub rays: Vec<(Vec2, Vec2, HitType)>, // (start, end, hit_type)
}

/// Compute sensor inputs for all entities.
/// Returns a Vec of sensor arrays, indexed by entity slot.
/// Also returns ray data for visualization if requested.
pub fn compute_all_sensors(
    arena: &EntityArena,
    food_positions: &[Vec2],
    spatial: &SpatialHash,
    world: &World,
    environment: &EnvironmentState,
    collect_rays: bool,
) -> (Vec<[f32; config::BRAIN_SENSOR_NEURONS]>, Vec<Option<EntityRays>>) {
    let capacity = arena.entities.len();
    let mut all_inputs = vec![[0.0f32; config::BRAIN_SENSOR_NEURONS]; capacity];
    let mut all_rays: Vec<Option<EntityRays>> = if collect_rays {
        vec![None; capacity]
    } else {
        Vec::new()
    };

    for (idx, entity) in arena.entities.iter().enumerate() {
        let entity = match entity {
            Some(e) => e,
            None => continue,
        };

        let ray_length = config::SENSOR_RAY_LENGTH * entity.sensor_range;
        let num_rays = config::NUM_SENSOR_RAYS;
        let arc = config::SENSOR_ARC;
        let step_angle = arc / (num_rays - 1).max(1) as f32;
        let start_angle = entity.heading - arc * 0.5;

        // Cast rays and collect hits
        let mut ray_distances = [1.0f32; 8]; // default = nothing hit
        let mut ray_types = [HitType::Nothing; 8];
        let mut ray_data = if collect_rays {
            Vec::with_capacity(num_rays)
        } else {
            Vec::new()
        };

        for ray_i in 0..num_rays.min(8) {
            let angle = start_angle + step_angle * ray_i as f32;
            let dir = Vec2::from_angle(angle);

            let hit = raycast(
                entity.pos,
                dir,
                ray_length,
                idx as u32,
                arena,
                food_positions,
                spatial,
                world,
            );

            ray_distances[ray_i] = hit.distance_norm;
            ray_types[ray_i] = hit.hit_type;

            if collect_rays {
                let end = world.wrap(entity.pos + dir * ray_length * hit.distance_norm);
                ray_data.push((entity.pos, end, hit.hit_type));
            }
        }

        if collect_rays {
            all_rays[idx] = Some(EntityRays { rays: ray_data });
        }

        // Compress 8 rays into 6 brain sensor inputs:
        // [0]: avg proximity left side (rays 0-3), inverted: 1 = close, 0 = far
        // [1]: avg proximity right side (rays 4-7), inverted
        // [2]: food proximity (min distance to food ray, inverted)
        // [3]: entity proximity (min distance to entity ray, inverted)
        // [4]: own energy level normalized [0,1]
        // [5]: environment signal: terrain danger + day/night combined

        let left_prox = 1.0
            - (ray_distances[0] + ray_distances[1] + ray_distances[2] + ray_distances[3]) * 0.25;
        let right_prox = 1.0
            - (ray_distances[4] + ray_distances[5] + ray_distances[6] + ray_distances[7]) * 0.25;

        let mut food_prox = 0.0f32;
        let mut entity_prox = 0.0f32;
        for ray_i in 0..num_rays.min(8) {
            let inv_dist = 1.0 - ray_distances[ray_i];
            match ray_types[ray_i] {
                HitType::Food => food_prox = food_prox.max(inv_dist),
                HitType::Entity => entity_prox = entity_prox.max(inv_dist),
                _ => {}
            }
        }

        let energy_norm = (entity.energy / config::MAX_ENTITY_ENERGY).clamp(0.0, 1.0);

        // Environment signal: combines terrain danger and day/night
        // Terrain: Water=0.8, Toxic=1.0, Desert=0.4, Forest=0.2, Plains=0.0
        // Day/night: adds 0.0 (full day) to 0.3 (full night)
        let terrain = environment.terrain.get_at(entity.pos);
        let terrain_danger = match terrain {
            TerrainType::Plains => 0.0,
            TerrainType::Forest => 0.2,
            TerrainType::Desert => 0.4,
            TerrainType::Water => 0.8,
            TerrainType::Toxic => 1.0,
        };
        let night_signal = 1.0 - environment.day_brightness(); // 0 at day, 0.7 at night
        let env_signal = (terrain_danger * 0.7 + night_signal * 0.3).clamp(0.0, 1.0);

        all_inputs[idx] = [left_prox, right_prox, food_prox, entity_prox, energy_norm, env_signal];
    }

    (all_inputs, all_rays)
}

/// Cast a single ray from `origin` in `direction`, checking for entity and food collisions.
fn raycast(
    origin: Vec2,
    direction: Vec2,
    max_dist: f32,
    exclude_idx: u32,
    arena: &EntityArena,
    food_positions: &[Vec2],
    spatial: &SpatialHash,
    world: &World,
) -> RayHit {
    // March along ray in discrete steps
    let step_size = 4.0;
    let num_steps = (max_dist / step_size) as usize;
    let entity_hit_radius = config::ENTITY_BASE_RADIUS * 1.5;
    let food_hit_radius = 8.0;

    let mut closest_hit = RayHit {
        distance_norm: 1.0,
        hit_type: HitType::Nothing,
    };

    for step in 1..=num_steps {
        let t = step as f32 * step_size;
        let sample_pos = world.wrap(origin + direction * t);

        // Check entities via spatial hash
        let nearby = spatial.query_radius_excluding(
            sample_pos,
            entity_hit_radius,
            exclude_idx,
            world,
            arena,
        );
        if !nearby.is_empty() {
            let norm = t / max_dist;
            if norm < closest_hit.distance_norm {
                closest_hit = RayHit {
                    distance_norm: norm,
                    hit_type: HitType::Entity,
                };
                return closest_hit; // first hit along ray is closest
            }
        }

        // Check food (brute force since food count is moderate)
        for food_pos in food_positions {
            let dist_sq = world.distance_sq(sample_pos, *food_pos);
            if dist_sq < food_hit_radius * food_hit_radius {
                let norm = t / max_dist;
                if norm < closest_hit.distance_norm {
                    closest_hit = RayHit {
                        distance_norm: norm,
                        hit_type: HitType::Food,
                    };
                    return closest_hit;
                }
            }
        }

        // Check world bounds (non-toroidal only)
        if !world.toroidal {
            let raw_pos = origin + direction * t;
            if raw_pos.x < 0.0 || raw_pos.x > world.width || raw_pos.y < 0.0 || raw_pos.y > world.height {
                let norm = t / max_dist;
                if norm < closest_hit.distance_norm {
                    closest_hit = RayHit {
                        distance_norm: norm,
                        hit_type: HitType::Wall,
                    };
                    return closest_hit;
                }
            }
        }
    }

    closest_hit
}
