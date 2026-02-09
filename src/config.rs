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
pub const ENTITY_TURN_AT_MAX_SPEED_FACTOR: f32 = 0.42;
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
pub const MAX_CARRIED_ENERGY: f32 = 120.0;
pub const IDLE_METABOLIC_COST: f32 = 0.5;
pub const MOVE_METABOLIC_COST: f32 = 1.5;
pub const REPRODUCTION_THRESHOLD: f32 = 150.0;
pub const REPRODUCTION_COST: f32 = 80.0;
pub const OFFSPRING_ENERGY_FRACTION: f32 = 0.3;
pub const DEATH_AGE: f32 = 600.0;

// Brain (Phase 2+)
pub const BRAIN_SENSOR_NEURONS: usize = 15;
pub const BRAIN_MOTOR_NEURONS: usize = 10;
pub const BRAIN_INTERNEURONS_DEFAULT: usize = 7;
pub const BRAIN_MIN_INTERNEURONS: usize = 3;
pub const BRAIN_MAX_INTERNEURONS: usize = 16;
pub const BRAIN_NEURONS_DEFAULT: usize =
    BRAIN_SENSOR_NEURONS + BRAIN_INTERNEURONS_DEFAULT + BRAIN_MOTOR_NEURONS;
pub const STRUCTURAL_ADD_PROB: f32 = 0.01;
pub const STRUCTURAL_REMOVE_PROB: f32 = 0.01;
pub const REPRODUCTION_INTENT_THRESHOLD: f32 = 0.6;
pub const ATTACK_INTENT_THRESHOLD: f32 = 0.7;
pub const EAT_INTENT_THRESHOLD: f32 = 0.5;
pub const PICKUP_INTENT_THRESHOLD: f32 = 0.55;
pub const SHARE_INTENT_THRESHOLD: f32 = 0.6;

// Sensory (Phase 2+)
pub const NUM_SENSOR_RAYS: usize = 8;
pub const SENSOR_RAY_LENGTH: f32 = 150.0;
pub const SENSOR_ARC: f32 = std::f32::consts::PI * 1.5; // 270 degrees
pub const SENSOR_ADJACENT_RADIUS: f32 = 22.0;

// Combat (Phase 4+)
pub const ATTACK_RANGE: f32 = 15.0;
pub const ATTACK_COST: f32 = 5.0;
pub const ATTACK_DAMAGE: f32 = 25.0;
pub const ATTACK_ENERGY_MIN_MULT: f32 = 0.35;
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
pub const STORM_FOREST_SHELTER_MULT: f32 = 0.3;
pub const STORM_WALL_SHELTER_RANGE: f32 = 140.0;
pub const STORM_WALL_SHELTER_MIN_MULT: f32 = 0.35;
pub const STORM_COMBINED_SHELTER_MIN_MULT: f32 = 0.15;
pub const TOXIC_ZONE_RADIUS: f32 = 90.0;
pub const TOXIC_ZONE_DAMAGE: f32 = 5.0;
pub const TOXIC_ZONE_DURATION: f32 = 25.0;

// Walls / tools
pub const WALL_THICKNESS: f32 = 8.0;
pub const FOOD_CLUSTER_COUNT: usize = 18;
pub const FOOD_CLUSTER_RADIUS: f32 = 35.0;

// Visual FX
pub const VISUAL_STORM_CELL_SIZE: f32 = 28.0;
pub const VISUAL_STORM_LINE_BASE_LENGTH: f32 = 36.0;
pub const VISUAL_STORM_LINE_LENGTH_JITTER: f32 = 24.0;
pub const VISUAL_STORM_LINE_ALPHA: f32 = 0.45;
pub const VISUAL_STORM_GUST_ARC_ALPHA: f32 = 0.24;
pub const VISUAL_STORM_LINES_LOW: usize = 48;
pub const VISUAL_STORM_LINES_MEDIUM: usize = 80;
pub const VISUAL_STORM_LINES_HIGH: usize = 128;
pub const VISUAL_STORM_LINES_ULTRA: usize = 196;
pub const VISUAL_STORM_GUSTS_LOW: usize = 8;
pub const VISUAL_STORM_GUSTS_MEDIUM: usize = 14;
pub const VISUAL_STORM_GUSTS_HIGH: usize = 22;
pub const VISUAL_STORM_GUSTS_ULTRA: usize = 34;
pub const VISUAL_STORM_TICK_QUANTIZATION: u64 = 4;

pub const VISUAL_TRAIL_BASE_LENGTH: f32 = 10.0;
pub const VISUAL_TRAIL_SPEED_SCALE: f32 = 0.08;
pub const VISUAL_TRAIL_MAX_LENGTH: f32 = 42.0;
pub const VISUAL_TRAIL_ALPHA: f32 = 0.28;

pub const VISUAL_ATMOSPHERE_BASE_STRENGTH: f32 = 0.22;
pub const VISUAL_ATMOSPHERE_NOISE_ALPHA: f32 = 0.08;

pub const VISUAL_BLOOM_THRESHOLD_LOW: f32 = 0.72;
pub const VISUAL_BLOOM_THRESHOLD_MEDIUM: f32 = 0.66;
pub const VISUAL_BLOOM_THRESHOLD_HIGH: f32 = 0.60;
pub const VISUAL_BLOOM_THRESHOLD_ULTRA: f32 = 0.54;
pub const VISUAL_BLOOM_INTENSITY_LOW: f32 = 0.25;
pub const VISUAL_BLOOM_INTENSITY_MEDIUM: f32 = 0.33;
pub const VISUAL_BLOOM_INTENSITY_HIGH: f32 = 0.42;
pub const VISUAL_BLOOM_INTENSITY_ULTRA: f32 = 0.52;

pub const VISUAL_VIGNETTE_LOW: f32 = 0.12;
pub const VISUAL_VIGNETTE_MEDIUM: f32 = 0.18;
pub const VISUAL_VIGNETTE_HIGH: f32 = 0.24;
pub const VISUAL_VIGNETTE_ULTRA: f32 = 0.30;
pub const VISUAL_GRADE_STRENGTH_LOW: f32 = 0.08;
pub const VISUAL_GRADE_STRENGTH_MEDIUM: f32 = 0.12;
pub const VISUAL_GRADE_STRENGTH_HIGH: f32 = 0.16;
pub const VISUAL_GRADE_STRENGTH_ULTRA: f32 = 0.22;

pub const VISUAL_SHELTER_HIGHLIGHT_ALPHA: f32 = 0.22;
pub const VISUAL_SHELTER_BANDS_LOW: usize = 2;
pub const VISUAL_SHELTER_BANDS_MEDIUM: usize = 3;
pub const VISUAL_SHELTER_BANDS_HIGH: usize = 4;
pub const VISUAL_SHELTER_BANDS_ULTRA: usize = 5;

// Camera
pub const CAMERA_ZOOM_MIN: f32 = 0.05;
pub const CAMERA_ZOOM_MAX: f32 = 2.0;
pub const CAMERA_PAN_SPEED: f32 = 500.0;
pub const CAMERA_ZOOM_SPEED: f32 = 0.1;
pub const CAMERA_SMOOTH_SPEED: f32 = 8.0;
