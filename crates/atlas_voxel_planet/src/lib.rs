//! `atlas_voxel_planet` — Rust Voxel Planet Engine, packaged as a reusable library.
//!
//! # Plugin groups
//! * [`VoxelPlanetPlugins`] — solar system, terrain, atmosphere, vegetation.
//!   Safe to add in both the editor and the runtime app.
//! * [`PlayerPlugin`] — first-person controller. In the editor, PIE handles
//!   player spawning manually via [`EditorPlayPlugin`]; `PlayerPlugin` is kept
//!   for testing and standalone contexts.

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod ambient_audio;
pub mod atmosphere;
pub mod biome;
pub mod character;
pub mod components;
pub mod config;
pub mod crafting;
pub mod hud;
pub mod inventory;
pub mod minimap;
pub mod multiplayer;
pub mod npc;
pub mod planet;
pub mod player;
pub mod solar_system;
pub mod structures;
pub mod vegetation;
pub mod wildlife;
pub mod world_io;

pub use ambient_audio::{AmbientAudioPlugin, AmbientAudioState, AmbientTrack};
pub use atmosphere::AtmospherePlugin;
pub use biome::{classify_biome, Biome, Voxel};
pub use character::{CameraMode, CharacterPlugin};
pub use components::*;
pub use config::*;
pub use crafting::{CraftingPlugin, CraftingUiState, Recipe, RECIPES};
pub use hud::{GroundHudText, HudPlugin, SpaceHudText};
pub use inventory::{Inventory, InventoryPlugin, VoxelRaycastResult};
pub use minimap::{MinimapPlugin, MinimapResource};
pub use multiplayer::{MultiplayerPlugin, NetworkConfig, NetworkRole};
pub use npc::{DialogueState, NpcPlugin, QuestLog, QuestStatus};
pub use planet::{terrain_radius_at, chunk_voxel_index, build_chunk_mesh, NoiseCache, PlanetPlugin};
pub use player::{update_chunk_viewpoint_from_player, update_survival_stats, PlayerPlugin};
pub use solar_system::SolarSystemPlugin;
pub use structures::StructuresPlugin;
pub use vegetation::VegetationPlugin;
pub use wildlife::WildlifePlugin;
pub use world_io::{SaveWorldRequest, LoadWorldRequest, WorldIoPlugin};

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
            .add(wildlife::WildlifePlugin)
            .add(structures::StructuresPlugin)
            .add(npc::NpcPlugin)
            .add(inventory::InventoryPlugin)
            .add(crafting::CraftingPlugin)
            .add(character::CharacterPlugin)
            .add(multiplayer::MultiplayerPlugin)
            .add(hud::HudPlugin)
            .add(minimap::MinimapPlugin)
            .add(ambient_audio::AmbientAudioPlugin)
            .add(world_io::WorldIoPlugin)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// World regeneration event
// ─────────────────────────────────────────────────────────────────────────────

/// Sent by the World Settings panel (or any system) to despawn all loaded
/// chunks and restart terrain generation with the current [`NoiseSeed`].
#[derive(Event)]
pub struct RegenerateWorld;
