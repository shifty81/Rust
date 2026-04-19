use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

use crate::config::*;

// ============================================================
//  MARKER COMPONENTS
// ============================================================

/// The main planet body.
#[derive(Component)]
pub struct Planet;

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
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            is_grounded: false,
            grounded_timer: 0.0,
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
    pub loaded:  HashMap<IVec3, Entity>,
    /// Queue of chunk positions waiting to be generated.
    pub pending: VecDeque<IVec3>,
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
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            render_distance:      RENDER_DISTANCE,
            max_chunks_per_frame: MAX_CHUNKS_PER_FRAME,
        }
    }
}
