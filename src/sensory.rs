use macroquad::prelude::*;

use crate::config;
use crate::entity::EntityArena;
use crate::environment::{closest_point_on_segment, point_near_any_wall, EnvironmentState};
use crate::signals::{PheromoneGrid, SignalState};
use crate::spatial_hash::SpatialHash;
use crate::world::World;

/// What a sensor ray hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitType {
    Nothing,
    Entity,
    Food,
    Wall,
    Hazard,
}

/// Result of a single raycast.
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub distance_norm: f32, // [0, 1] where 0 = at origin, 1 = max range (nothing hit)
    pub hit_type: HitType,
    pub entity_idx: Option<usize>,
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
    meat_positions: &[Vec2],
    signals: &[SignalState],
    pheromones: &PheromoneGrid,
    spatial: &SpatialHash,
    world: &World,
    environment: &EnvironmentState,
    collect_rays: bool,
) -> (
    Vec<[f32; config::BRAIN_SENSOR_NEURONS]>,
    Vec<Option<EntityRays>>,
) {
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
        if !entity.alive {
            continue;
        }

        let ray_length = config::SENSOR_RAY_LENGTH * entity.sensor_range;
        let num_rays = config::NUM_SENSOR_RAYS;
        let arc = config::SENSOR_ARC;
        let step_angle = arc / (num_rays.saturating_sub(1).max(1)) as f32;
        let start_angle = entity.heading - arc * 0.5;

        let mut ray_distances = vec![1.0f32; num_rays];
        let mut ray_types = vec![HitType::Nothing; num_rays];
        let mut ray_entities: Vec<Option<usize>> = vec![None; num_rays];
        let mut ray_data = if collect_rays {
            Vec::with_capacity(num_rays)
        } else {
            Vec::new()
        };

        for ray_i in 0..num_rays {
            let angle = start_angle + step_angle * ray_i as f32;
            let dir = Vec2::from_angle(angle);

            let hit = raycast(
                entity.pos,
                dir,
                ray_length,
                idx as u32,
                arena,
                food_positions,
                meat_positions,
                spatial,
                world,
                environment,
            );

            ray_distances[ray_i] = hit.distance_norm;
            ray_types[ray_i] = hit.hit_type;
            ray_entities[ray_i] = hit.entity_idx;

            if collect_rays {
                let end = world.wrap(entity.pos + dir * ray_length * hit.distance_norm);
                ray_data.push((entity.pos, end, hit.hit_type));
            }
        }

        if collect_rays {
            all_rays[idx] = Some(EntityRays { rays: ray_data });
        }

        let split = num_rays / 2;
        let left_avg = if split > 0 {
            ray_distances[..split].iter().sum::<f32>() / split as f32
        } else {
            1.0
        };
        let right_count = num_rays.saturating_sub(split).max(1);
        let right_avg = ray_distances[split..].iter().sum::<f32>() / right_count as f32;

        let left_prox = 1.0 - left_avg;
        let right_prox = 1.0 - right_avg;

        let mut food_prox = 0.0f32;
        let mut entity_prox = 0.0f32;
        let mut obstacle_prox = 0.0f32;
        let mut sensed_signal = [0.0f32; 3];

        for ray_i in 0..num_rays {
            let inv_dist = 1.0 - ray_distances[ray_i];
            match ray_types[ray_i] {
                HitType::Food => food_prox = food_prox.max(inv_dist),
                HitType::Entity => {
                    entity_prox = entity_prox.max(inv_dist);
                    if let Some(e_idx) = ray_entities[ray_i] {
                        if let Some(signal) = signals.get(e_idx) {
                            sensed_signal[0] = sensed_signal[0].max(signal.color.r * inv_dist);
                            sensed_signal[1] = sensed_signal[1].max(signal.color.g * inv_dist);
                            sensed_signal[2] = sensed_signal[2].max(signal.color.b * inv_dist);
                        }
                    }
                }
                HitType::Wall | HitType::Hazard => obstacle_prox = obstacle_prox.max(inv_dist),
                HitType::Nothing => {}
            }
        }

        let energy_norm = (entity.energy / config::MAX_ENTITY_ENERGY).clamp(0.0, 1.0);
        let health_norm = (entity.health / entity.max_health.max(1.0)).clamp(0.0, 1.0);
        let age_norm = (entity.age / config::DEATH_AGE).clamp(0.0, 1.0);
        let speed_norm = (entity.velocity.length()
            / (config::ENTITY_MAX_SPEED * entity.speed_multiplier).max(1.0))
        .clamp(0.0, 1.0);
        let carried_norm = (entity.carried_energy / config::MAX_CARRIED_ENERGY).clamp(0.0, 1.0);

        let adjacent = adjacent_contact(
            idx as u32,
            entity.pos,
            arena,
            food_positions,
            meat_positions,
            spatial,
            world,
            environment,
        );

        // Pheromone gradient projected onto current heading.
        let grad = pheromones.gradient(entity.pos);
        let grad_strength = grad.length().clamp(0.0, 1.0);
        let heading = Vec2::from_angle(entity.heading);
        let pheromone_alignment = if grad_strength > 0.0001 {
            grad.normalize().dot(heading).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let pheromone_sensor = ((pheromone_alignment * grad_strength) + 1.0) * 0.5;

        all_inputs[idx] = [
            left_prox,
            right_prox,
            food_prox,
            entity_prox,
            obstacle_prox,
            pheromone_sensor,
            sensed_signal[0],
            sensed_signal[1],
            sensed_signal[2],
            energy_norm,
            health_norm,
            age_norm,
            speed_norm,
            carried_norm,
            if adjacent { 1.0 } else { 0.0 },
        ];
    }

    (all_inputs, all_rays)
}

fn adjacent_contact(
    exclude_idx: u32,
    pos: Vec2,
    arena: &EntityArena,
    food_positions: &[Vec2],
    meat_positions: &[Vec2],
    spatial: &SpatialHash,
    world: &World,
    environment: &EnvironmentState,
) -> bool {
    let radius = config::SENSOR_ADJACENT_RADIUS;
    let radius_sq = radius * radius;

    let neighbors = spatial.query_radius_excluding(pos, radius, exclude_idx, world, arena);
    if !neighbors.is_empty() {
        return true;
    }

    if food_positions
        .iter()
        .any(|p| world.distance_sq(*p, pos) <= radius_sq)
    {
        return true;
    }

    if meat_positions
        .iter()
        .any(|p| world.distance_sq(*p, pos) <= radius_sq)
    {
        return true;
    }

    if point_near_any_wall(pos, &environment.walls, world, radius) {
        return true;
    }

    environment
        .toxic_zones
        .iter()
        .any(|zone| world.distance_sq(zone.center, pos) <= zone.radius * zone.radius)
}

/// Cast a single ray from `origin` in `direction`, checking for entity, food, walls, and hazards.
fn raycast(
    origin: Vec2,
    direction: Vec2,
    max_dist: f32,
    exclude_idx: u32,
    arena: &EntityArena,
    food_positions: &[Vec2],
    meat_positions: &[Vec2],
    spatial: &SpatialHash,
    world: &World,
    environment: &EnvironmentState,
) -> RayHit {
    let step_size = 4.0;
    let num_steps = (max_dist / step_size) as usize;
    let entity_hit_radius = config::ENTITY_BASE_RADIUS * 1.5;
    let food_hit_radius = 8.0;
    let wall_hit_radius = config::WALL_THICKNESS * 0.8;
    let food_hit_sq = food_hit_radius * food_hit_radius;
    let wall_hit_sq = wall_hit_radius * wall_hit_radius;

    let mut closest_hit = RayHit {
        distance_norm: 1.0,
        hit_type: HitType::Nothing,
        entity_idx: None,
    };

    for step in 1..=num_steps {
        let t = step as f32 * step_size;
        let sample_pos = world.wrap(origin + direction * t);

        let nearby = spatial.query_radius_excluding(
            sample_pos,
            entity_hit_radius,
            exclude_idx,
            world,
            arena,
        );
        if !nearby.is_empty() {
            let mut nearest: Option<(usize, f32)> = None;
            for idx in nearby {
                if let Some(candidate) = arena.get_by_index(idx as usize) {
                    let dist_sq = world.distance_sq(sample_pos, candidate.pos);
                    match nearest {
                        Some((_, best_dist)) if dist_sq >= best_dist => {}
                        _ => nearest = Some((idx as usize, dist_sq)),
                    }
                }
            }

            if let Some((entity_idx, _)) = nearest {
                let norm = t / max_dist;
                return RayHit {
                    distance_norm: norm,
                    hit_type: HitType::Entity,
                    entity_idx: Some(entity_idx),
                };
            }
        }

        if food_positions
            .iter()
            .chain(meat_positions.iter())
            .any(|food_pos| world.distance_sq(sample_pos, *food_pos) < food_hit_sq)
        {
            let norm = t / max_dist;
            return RayHit {
                distance_norm: norm,
                hit_type: HitType::Food,
                entity_idx: None,
            };
        }

        for wall in &environment.walls {
            let cp = closest_point_on_segment(wall.start, wall.end, sample_pos);
            if world.distance_sq(sample_pos, cp) <= wall_hit_sq {
                let norm = t / max_dist;
                return RayHit {
                    distance_norm: norm,
                    hit_type: HitType::Wall,
                    entity_idx: None,
                };
            }
        }

        if environment
            .toxic_zones
            .iter()
            .any(|zone| world.distance_sq(sample_pos, zone.center) <= zone.radius * zone.radius)
        {
            let norm = t / max_dist;
            return RayHit {
                distance_norm: norm,
                hit_type: HitType::Hazard,
                entity_idx: None,
            };
        }

        if !world.toroidal {
            let raw_pos = origin + direction * t;
            if raw_pos.x < 0.0
                || raw_pos.x > world.width
                || raw_pos.y < 0.0
                || raw_pos.y > world.height
            {
                let norm = t / max_dist;
                if norm < closest_hit.distance_norm {
                    closest_hit = RayHit {
                        distance_norm: norm,
                        hit_type: HitType::Wall,
                        entity_idx: None,
                    };
                    return closest_hit;
                }
            }
        }
    }

    closest_hit
}
