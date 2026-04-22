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
// Voxel-world on/off toggle
// ─────────────────────────────────────────────────────────────────────────────

/// Controls whether the voxel world (solar system, planet terrain) should be
/// active.
///
/// Default is `true` (normal editor sandbox mode).  Set to `false` before
/// `app.run()` when the editor is linked to a Nova-Forge game repository so
/// that the default planet demo is suppressed in favour of an empty viewport
/// waiting for game content.
#[derive(bevy::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelWorldEnabled(pub bool);

impl Default for VoxelWorldEnabled {
    fn default() -> Self { Self(true) }
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin groups
// ─────────────────────────────────────────────────────────────────────────────

/// Pure world-simulation plugins: solar system, planet terrain, atmosphere,
/// vegetation, wildlife, structures, and world I/O.  **Does not** include any
/// in-game UI / HUD plugins — safe to use in the editor's Editing mode without
/// pulling gameplay overlays (HUD, minimap, hotbar, crafting panel, dialogue)
/// on top of the editor panels.
///
/// This is the plugin group the Atlas editor uses in `atlas_editor_app`.
pub struct WorldPlugins;

impl PluginGroup for WorldPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(solar_system::SolarSystemPlugin)
            .add(planet::PlanetPlugin)
            .add(atmosphere::AtmospherePlugin)
            .add(vegetation::VegetationPlugin)
            .add(wildlife::WildlifePlugin)
            .add(structures::StructuresPlugin)
            .add(world_io::WorldIoPlugin)
    }
}

/// Gameplay UI / HUD / interaction plugins: HUD, minimap, inventory/hotbar,
/// crafting, NPC dialogue, player character model, ambient audio and
/// multiplayer.  These spawn Bevy UI nodes and register update systems that
/// only make sense when a [`Player`] entity exists.
///
/// Runtime (standalone-game) builds should add both [`WorldPlugins`] and
/// [`GameplayPlugins`].
pub struct GameplayPlugins;

impl PluginGroup for GameplayPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(npc::NpcPlugin)
            .add(inventory::InventoryPlugin)
            .add(crafting::CraftingPlugin)
            .add(character::CharacterPlugin)
            .add(multiplayer::MultiplayerPlugin)
            .add(hud::HudPlugin)
            .add(minimap::MinimapPlugin)
            .add(ambient_audio::AmbientAudioPlugin)
    }
}

/// Backwards-compatible union of [`WorldPlugins`] + [`GameplayPlugins`].
/// New code should prefer the specific group it needs.
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
