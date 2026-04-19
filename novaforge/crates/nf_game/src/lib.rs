//! `nf_game` — gameplay systems and plugins.
//!
//! This crate contains all actual game logic.  It is compiled into both the
//! shipped runtime (`nf_runtime_app`) and the editor's PIE runtime world
//! (`nf_editor_play`).  It must not depend on any `nf_editor_*` crate.

use bevy::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// Runtime-only marker components
// ────────────────────────────────────────────────────────────────────────────

/// Tags an entity as a player-controlled actor.
#[derive(Component, Default)]
pub struct Player;

/// Health points component.
#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: f32,
    pub maximum: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self { current: 100.0, maximum: 100.0 }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

/// Registers all gameplay systems.  Add to both the runtime app and the PIE
/// sub-app inside `nf_editor_play`.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, _app: &mut App) {
        // Gameplay systems (movement, combat, AI, …) will be registered here.
    }
}
