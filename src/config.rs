// All tunable simulation constants in one place.

// World
pub const WORLD_WIDTH: f32 = 2000.0;
pub const WORLD_HEIGHT: f32 = 2000.0;
pub const WORLD_TOROIDAL: bool = true;

// Entities
pub const INITIAL_ENTITY_COUNT: usize = 50;
pub const MAX_ENTITY_COUNT: usize = 300;
pub const ENTITY_BASE_RADIUS: f32 = 8.0;
pub const ENTITY_MAX_SPEED: f32 = 120.0;
pub const ENTITY_TURN_RATE: f32 = 4.0;
pub const ENTITY_FRICTION: f32 = 3.0;

// Simulation
pub const FIXED_DT: f32 = 1.0 / 60.0;

// Spatial hash
pub const SPATIAL_CELL_SIZE: f32 = 64.0;

// Energy (Phase 3+)
pub const INITIAL_FOOD_COUNT: usize = 300;
pub const FOOD_RESPAWN_RATE: f32 = 2.0;
pub const FOOD_ENERGY: f32 = 40.0;
pub const INITIAL_ENTITY_ENERGY: f32 = 100.0;
pub const MAX_ENTITY_ENERGY: f32 = 200.0;
pub const IDLE_METABOLIC_COST: f32 = 0.5;
pub const MOVE_METABOLIC_COST: f32 = 1.5;
pub const REPRODUCTION_THRESHOLD: f32 = 150.0;
pub const REPRODUCTION_COST: f32 = 80.0;
pub const OFFSPRING_ENERGY_FRACTION: f32 = 0.3;
pub const DEATH_AGE: f32 = 600.0;

// Mutation (Phase 3+)
pub const MUTATION_RATE: f32 = 0.05;
pub const MUTATION_SIGMA: f32 = 0.1;

// Brain (Phase 2+)
pub const BRAIN_NEURONS: usize = 12;
pub const BRAIN_SENSOR_NEURONS: usize = 6;
pub const BRAIN_INTERNEURONS: usize = 2;
pub const BRAIN_MOTOR_NEURONS: usize = 4;

// Sensory (Phase 2+)
pub const NUM_SENSOR_RAYS: usize = 8;
pub const SENSOR_RAY_LENGTH: f32 = 150.0;
pub const SENSOR_ARC: f32 = std::f32::consts::PI * 1.5; // 270 degrees

// Combat (Phase 4+)
pub const ATTACK_RANGE: f32 = 15.0;
pub const ATTACK_COST: f32 = 5.0;
pub const ATTACK_DAMAGE: f32 = 25.0;
pub const MEAT_ENERGY: f32 = 60.0;
pub const MEAT_DECAY_TIME: f32 = 30.0;

// Environment (Phase 5+)
pub const DAY_LENGTH: f32 = 120.0;
pub const SEASON_LENGTH: f32 = 300.0;
pub const STORM_DURATION: f32 = 45.0;
pub const STORM_INTERVAL_MIN: f32 = 120.0;
pub const STORM_INTERVAL_MAX: f32 = 300.0;
pub const STORM_RADIUS: f32 = 200.0;
pub const STORM_DAMAGE: f32 = 2.0;

// Camera
pub const CAMERA_ZOOM_MIN: f32 = 0.05;
pub const CAMERA_ZOOM_MAX: f32 = 2.0;
pub const CAMERA_PAN_SPEED: f32 = 500.0;
pub const CAMERA_ZOOM_SPEED: f32 = 0.1;
pub const CAMERA_SMOOTH_SPEED: f32 = 8.0;
