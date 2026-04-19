//! `atlas_prefab` — prefab definitions, instance override tracking, and spawning.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use atlas_core::{StableId, TransformData, AssetId};
use atlas_scene::ComponentBlob;

// ────────────────────────────────────────────────────────────────────────────
// Prefab file format
// ────────────────────────────────────────────────────────────────────────────

pub const PREFAB_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabFile {
    pub version: u32,
    pub root:    PrefabNode,
}

/// A node in the prefab's entity graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabNode {
    pub id:         StableId,
    pub name:       String,
    pub transform:  TransformData,
    pub components: Vec<ComponentBlob>,
    pub children:   Vec<PrefabNode>,
}

// ────────────────────────────────────────────────────────────────────────────
// Instance overrides
// ────────────────────────────────────────────────────────────────────────────

/// The kind of override applied to a specific node in a prefab instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverrideKind {
    /// A component field was changed from the prefab default.
    ModifiedField {
        component_type: String,
        field_name:     String,
        /// RON-encoded new value.
        value:          String,
    },
    /// A component was added to this instance only.
    AddedComponent(ComponentBlob),
    /// A component that exists in the prefab was removed from this instance.
    RemovedComponent(String),
    /// The transform was changed.
    TransformOverride(TransformData),
}

/// All overrides applied to a single node of a prefab instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOverride {
    pub node_id:   StableId,
    pub overrides: Vec<OverrideKind>,
}

/// Attached to an entity that is a placed prefab instance.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PrefabInstance {
    pub prefab_asset_id: AssetId,
    pub node_overrides:  Vec<NodeOverride>,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct PrefabPlugin;

impl Plugin for PrefabPlugin {
    fn build(&self, _app: &mut App) {
        // Prefab spawning and override application systems will be added here.
    }
}
