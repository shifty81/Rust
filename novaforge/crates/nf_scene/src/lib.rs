//! `nf_scene` — versioned scene format, stable entity IDs, and scene loading.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use nf_core::{StableId, TransformData, AssetId};

// ────────────────────────────────────────────────────────────────────────────
// Scene file format
// ────────────────────────────────────────────────────────────────────────────

/// Bump this when the on-disk format changes and write a migration.
pub const SCENE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFile {
    pub version:       u32,
    pub name:          String,
    pub world_settings: WorldSettings,
    pub entities:      Vec<SceneEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldSettings {
    pub ambient_color:     [f32; 3],
    pub ambient_intensity: f32,
    pub gravity:           [f32; 3],
}

/// A single entity record inside a [`SceneFile`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id:             StableId,
    pub name:           String,
    pub parent:         Option<StableId>,
    pub transform:      TransformData,
    /// Opaque blobs, one per component type.  Deserialized at load time.
    pub components:     Vec<ComponentBlob>,
    pub prefab_instance: Option<PrefabInstanceRef>,
    pub editor_meta:    EditorMetadata,
}

/// An opaque, versioned component blob stored in a scene file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentBlob {
    /// Fully-qualified Rust type name used to look up the deserializer.
    pub type_name: String,
    /// RON-encoded component data.
    pub data:      String,
}

/// Reference back to the prefab asset this instance was spawned from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabInstanceRef {
    pub prefab_asset_id: AssetId,
}

/// Editor-only metadata that is NOT loaded at runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorMetadata {
    pub hidden_in_editor: bool,
    pub locked:           bool,
    pub folder_path:      Option<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Dirty-tracking resource
// ────────────────────────────────────────────────────────────────────────────

/// Tracks whether the currently open scene has unsaved changes.
#[derive(Resource, Default)]
pub struct SceneDirty(pub bool);

/// The path of the currently open scene file, if any.
#[derive(Resource, Default)]
pub struct ActiveScenePath(pub Option<std::path::PathBuf>);

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SceneDirty>()
            .init_resource::<ActiveScenePath>();
    }
}
