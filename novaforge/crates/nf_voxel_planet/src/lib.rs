//! `nf_voxel_planet` — Rust Voxel Planet Engine, packaged as a reusable library.
//!
//! # Plugin groups
//! * [`VoxelPlanetPlugins`] — solar system, terrain, atmosphere, vegetation.
//!   Safe to add in both the editor and the runtime app.
//! * [`PlayerPlugin`] — first-person controller. Add this in the standalone
//!   runtime; in the editor, PIE handles player spawning manually.

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod atmosphere;
pub mod biome;
pub mod components;
pub mod config;
pub mod planet;
pub mod player;
pub mod solar_system;
pub mod vegetation;

pub use atmosphere::AtmospherePlugin;
pub use biome::{classify_biome, Biome, Voxel};
pub use components::*;
pub use config::*;
pub use planet::{terrain_radius_at, PlanetPlugin};
pub use player::PlayerPlugin;
pub use solar_system::SolarSystemPlugin;
pub use vegetation::VegetationPlugin;

// ─────────────────────────────────────────────────────────────────────────────
// Plugin group
// ─────────────────────────────────────────────────────────────────────────────

/// All world-building plugins: solar system, planet terrain, atmosphere, and
/// vegetation.  Does **not** include [`PlayerPlugin`] — add that separately so
/// the editor can render the world without a first-person controller.
pub struct VoxelPlanetPlugins;

impl PluginGroup for VoxelPlanetPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(solar_system::SolarSystemPlugin)
            .add(planet::PlanetPlugin)
            .add(atmosphere::AtmospherePlugin)
            .add(vegetation::VegetationPlugin)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// World regeneration event
// ─────────────────────────────────────────────────────────────────────────────

/// Sent by the World Settings panel (or any system) to despawn all loaded
/// chunks and restart terrain generation with the current [`NoiseSeed`].
#[derive(Event)]
pub struct RegenerateWorld;
