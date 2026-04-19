// ============================================================
//  PLANET CONFIGURATION
// ============================================================

/// Planet radius in metres.  1/128th of Earth (≈ 6 371 000 / 128 ≈ 49 773 m).
pub const PLANET_RADIUS: f32 = 49_773.0;

/// Sea-level is at exactly the planet radius.
pub const SEA_LEVEL: f32 = PLANET_RADIUS;

/// Maximum mountain height above sea level (metres).
pub const MAX_TERRAIN_HEIGHT: f32 = 900.0;

/// Noise frequency for the main terrain shape.
pub const TERRAIN_NOISE_SCALE: f64 = 2.2;

/// Noise frequency for the moisture map.
pub const MOISTURE_NOISE_SCALE: f64 = 1.8;

/// Latitude segments for the planet overview mesh.
pub const PLANET_LAT_SEGS: u32 = 180;

/// Longitude segments for the planet overview mesh.
pub const PLANET_LON_SEGS: u32 = 360;

// ============================================================
//  SOLAR-SYSTEM CONFIGURATION
// ============================================================

/// Distance from planet origin to sun (metres).
pub const SUN_DISTANCE: f32 = 1_500_000.0;

/// Visual radius of the sun sphere.
pub const SUN_RADIUS: f32 = 60_000.0;

/// Duration of one full day (seconds of real-time).
pub const DAY_LENGTH_SECONDS: f32 = 600.0;

/// Moon orbit radius (metres from planet centre).
pub const MOON_DISTANCE: f32 = 180_000.0;

/// Visual radius of the moon.
pub const MOON_RADIUS: f32 = 13_000.0;

/// Moon orbital period (seconds).
pub const MOON_ORBIT_PERIOD: f32 = 900.0;

/// Axial tilt of the planet's orbit plane (radians) – gives seasons.
pub const AXIAL_TILT: f32 = 0.41; // ≈ 23.5°

// Other planets relative distances from the sun (visual, not to scale).
pub const P2_ORBIT: f32 = 700_000.0;
pub const P3_ORBIT: f32 = 1_900_000.0;
pub const P4_ORBIT: f32 = 3_200_000.0;
pub const P5_ORBIT: f32 = 6_000_000.0;
pub const P6_ORBIT: f32 = 10_000_000.0;
pub const P7_ORBIT: f32 = 17_000_000.0;
pub const P8_ORBIT: f32 = 28_000_000.0;

// ============================================================
//  VOXEL / CHUNK CONFIGURATION
// ============================================================

/// Voxels per side in one cubic chunk.
pub const CHUNK_SIZE: usize = 16;

/// Size of a single voxel (metres).
pub const VOXEL_SIZE: f32 = 1.0;

/// How many chunks in each axis direction to keep loaded around the player.
pub const RENDER_DISTANCE: i32 = 7;

/// Maximum number of new chunks generated per frame (to avoid hitching).
pub const MAX_CHUNKS_PER_FRAME: usize = 3;

// ============================================================
//  PLAYER CONFIGURATION
// ============================================================

/// Eye height above the ground (metres).
pub const PLAYER_EYE_HEIGHT: f32 = 1.7;

/// Walking speed (m/s).
pub const PLAYER_WALK_SPEED: f32 = 5.5;

/// Sprint speed (m/s).
pub const PLAYER_RUN_SPEED: f32 = 14.0;

/// Jump initial velocity (m/s).
pub const PLAYER_JUMP_SPEED: f32 = 7.5;

/// Gravitational acceleration (m/s²).
pub const GRAVITY_STRENGTH: f32 = 9.81;

/// Mouse-look sensitivity (radians per pixel).
pub const MOUSE_SENSITIVITY: f32 = 0.0018;

/// Height above the terrain surface at which the player spawns (metres).
pub const SPAWN_HEIGHT: f32 = 5.0;

/// Maximum pitch angle (radians) to prevent gimbal extremes.
pub const MAX_PITCH: f32 = 1.5;

/// Small clearance (metres) between the player's feet and the terrain surface.
pub const PLAYER_FOOT_CLEARANCE: f32 = 0.05;

/// Flight speed in free-fly mode (m/s).
pub const PLAYER_FLY_SPEED: f32 = 400.0;

/// Sprint flight speed in free-fly mode (m/s).
pub const PLAYER_FLY_RUN_SPEED: f32 = 4_000.0;

// ============================================================
//  ATMOSPHERE / WEATHER
// ============================================================

/// Fog start distance (metres).
pub const FOG_START: f32 = 250.0;

/// Fog end distance (metres).
pub const FOG_END: f32 = 900.0;

/// How often (seconds) the weather randomly changes.
pub const WEATHER_CHANGE_INTERVAL: f32 = 90.0;

/// Maximum number of precipitation particles active at once.
pub const MAX_WEATHER_PARTICLES: usize = 600;

/// Altitude above sea level (metres) at which the atmosphere begins to fade.
pub const ATMOSPHERE_FADE_START: f32 = 8_000.0;

/// Altitude above sea level (metres) at which the atmosphere is fully gone.
pub const ATMOSPHERE_HEIGHT: f32 = 20_000.0;

// ============================================================
//  VEGETATION
// ============================================================

/// Probability (0–1) that a suitable surface voxel becomes a tree base.
pub const TREE_SPAWN_CHANCE: f32 = 0.012;

/// Probability (0–1) that a suitable surface voxel gets grass decoration.
pub const GRASS_SPAWN_CHANCE: f32 = 0.04;

/// Radius around player in which vegetation is checked / spawned (metres).
pub const VEGETATION_RADIUS: f32 = 80.0;

/// Pre-computed per-biome tree-spawn probabilities (TREE_SPAWN_CHANCE × density factor).
pub const TREE_PROB_FOREST: f32 = TREE_SPAWN_CHANCE * 80.0;
pub const TREE_PROB_PLAINS: f32 = TREE_SPAWN_CHANCE * 30.0;
pub const TREE_PROB_DESERT: f32 = TREE_SPAWN_CHANCE * 10.0;
pub const TREE_PROB_TUNDRA: f32 = TREE_SPAWN_CHANCE *  8.0;
