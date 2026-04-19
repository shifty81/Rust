use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

use crate::biome::Voxel;
use crate::config::*;

// ============================================================
//  MARKER COMPONENTS
// ============================================================

/// The main planet body.
#[derive(Component)]
pub struct Planet;

/// The sea-level ocean mesh that surrounds the planet.
#[derive(Component)]
pub struct Ocean;

/// The star (sun) entity.
#[derive(Component)]
pub struct Sun;

/// The moon entity.
#[derive(Component)]
pub struct Moon;

/// The player character.
#[derive(Component)]
pub struct Player;

/// The camera attached to the player.
#[derive(Component)]
pub struct PlayerCamera;

/// A loaded voxel chunk.
#[derive(Component)]
pub struct VoxelChunk {
    pub position: IVec3,
}

/// Raw voxel data stored on every chunk entity so edits have something to
/// mutate and undo commands can record old/new state.
#[derive(Component)]
pub struct VoxelData(pub Vec<Voxel>);

/// Marks a chunk whose mesh is stale and needs to be rebuilt next frame.
/// Set by the voxel editing tools and cleared by `remesh_dirty_chunks`.
#[derive(Component)]
pub struct ChunkDirty;

/// Marks a chunk whose voxels were manually edited by the user.
/// `handle_regen_world` skips these chunks so hand-crafted edits survive a
/// world regeneration.
#[derive(Component)]
pub struct ManuallyEdited;

/// A tree entity.
#[derive(Component)]
pub struct Tree;

/// A grass decoration entity.
#[derive(Component)]
pub struct GrassDecoration;

/// A weather-particle entity.
#[derive(Component)]
pub struct WeatherParticle;

/// The directional light representing the sun.
#[derive(Component)]
pub struct SunLight;

// ============================================================
//  DATA COMPONENTS
// ============================================================

/// Physics / orientation state for the player.
#[derive(Component)]
pub struct PlayerState {
    /// World-space velocity (m/s).
    pub velocity: Vec3,
    /// Yaw angle (radians) around the local-up axis.
    pub yaw: f32,
    /// Pitch angle (radians) for the camera.
    pub pitch: f32,
    /// True when the player is standing on terrain.
    pub is_grounded: bool,
    /// How long the player has been grounded (for coyote-time, etc.).
    pub grounded_timer: f32,
    /// True while the player is in free-fly / space-flight mode.
    /// Gravity and surface alignment are disabled; movement follows camera look.
    pub is_flying: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            is_grounded: false,
            grounded_timer: 0.0,
            is_flying: false,
        }
    }
}

/// Orbital body – follows a circular orbit around `orbit_center`.
#[derive(Component)]
pub struct OrbitalBody {
    pub orbit_radius: f32,
    pub orbit_period: f32,
    pub orbit_angle:  f32,
    pub orbit_center: Vec3,
    pub orbit_axis:   Vec3,
}

impl OrbitalBody {
    pub fn new(orbit_radius: f32, orbit_period: f32, orbit_center: Vec3) -> Self {
        Self {
            orbit_radius,
            orbit_period,
            orbit_angle: 0.0,
            orbit_center,
            orbit_axis: Vec3::Y,
        }
    }

    pub fn with_axis(mut self, axis: Vec3) -> Self {
        self.orbit_axis = axis;
        self
    }
}

/// Self-rotation around a local axis.
#[derive(Component)]
pub struct SelfRotation {
    pub axis:          Vec3,
    pub angular_speed: f32,
}

/// Statistics stored on each spawned chunk entity for editor inspection.
#[derive(Component)]
pub struct ChunkInfo {
    pub solid_voxel_count: u32,
    pub vertex_count:      u32,
}

// ============================================================
//  RESOURCES
// ============================================================

/// Tracks simulation time (day/night cycle, seasons, etc.).
#[derive(Resource)]
pub struct WorldTime {
    /// Fraction of the current day elapsed (0.0 = midnight, 0.25 = dawn,
    /// 0.5 = noon, 0.75 = dusk).
    pub day_fraction: f32,
    /// Total elapsed days since simulation start.
    pub total_days: f32,
}

impl Default for WorldTime {
    fn default() -> Self {
        Self {
            day_fraction: 0.25,
            total_days: 0.0,
        }
    }
}

/// Manages which voxel chunks are currently loaded.
#[derive(Resource, Default)]
pub struct ChunkManager {
    /// Map from chunk grid position to the corresponding entity.
    pub loaded:    HashMap<IVec3, Entity>,
    /// Queue of chunk positions waiting to be generated.
    pub pending:   VecDeque<IVec3>,
    /// Positions of chunks whose generation tasks are currently in flight.
    /// Prevents duplicate task spawning.
    pub in_flight: std::collections::HashSet<IVec3>,
}

/// Current weather state.
#[derive(Resource)]
pub struct WeatherState {
    pub kind:         WeatherKind,
    pub intensity:    f32,
    /// Seconds until the next weather change.
    pub change_timer: f32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum WeatherKind {
    Clear,
    Cloudy,
    Rain,
    Snow,
    Storm,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            kind:         WeatherKind::Clear,
            intensity:    0.0,
            change_timer: 60.0,
        }
    }
}

/// The world-space position used as the centre for chunk-loading decisions.
///
/// Updated every frame from either the player controller (PIE / runtime) or the
/// editor camera (Editing mode).  Chunk queuing and unloading both read this
/// instead of querying the `Player` transform directly, so the terrain generates
/// correctly even when there is no player entity in the world.
#[derive(Resource, Default)]
pub struct ChunkViewpoint(pub Vec3);

/// Stores the noise seed for reproducible world generation.
#[derive(Resource)]
pub struct NoiseSeed(pub u32);

impl Default for NoiseSeed {
    fn default() -> Self {
        Self(12345)
    }
}

/// Runtime-mutable world configuration.  The World Settings panel reads and
/// writes this resource; chunk-generation systems use it instead of the
/// compile-time constants so changes take effect immediately.
#[derive(Resource)]
pub struct WorldSettings {
    pub render_distance:       i32,
    pub max_chunks_per_frame:  usize,

    // ── Terrain noise (changing these triggers a full regen) ────────────────
    /// Noise frequency for the main terrain height map.
    pub terrain_noise_scale:  f64,
    /// Noise frequency for the moisture / biome map.
    pub moisture_noise_scale: f64,
    /// Maximum mountain height above sea level (metres).
    pub max_terrain_height:   f32,
    /// FBM octave count for the height map.
    pub noise_octaves:        usize,
    /// Lacunarity for both FBM generators.
    pub noise_lacunarity:     f64,
    /// Persistence for both FBM generators.
    pub noise_persistence:    f64,

    // ── Vegetation ──────────────────────────────────────────────────────────
    /// Radius around the player in which vegetation is spawned/despawned (m).
    pub vegetation_radius:  f32,
    /// Base probability that a suitable voxel becomes a tree.
    pub tree_spawn_chance:  f32,
    /// Probability that a suitable voxel gets a grass blade.
    pub grass_spawn_chance: f32,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            render_distance:      RENDER_DISTANCE,
            max_chunks_per_frame: MAX_CHUNKS_PER_FRAME,
            terrain_noise_scale:  TERRAIN_NOISE_SCALE,
            moisture_noise_scale: MOISTURE_NOISE_SCALE,
            max_terrain_height:   MAX_TERRAIN_HEIGHT,
            noise_octaves:        8,
            noise_lacunarity:     2.0,
            noise_persistence:    0.5,
            vegetation_radius:    VEGETATION_RADIUS,
            tree_spawn_chance:    TREE_SPAWN_CHANCE,
            grass_spawn_chance:   GRASS_SPAWN_CHANCE,
        }
    }
}
