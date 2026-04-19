//! `nf_assets` — asset registry, handles, metadata, and import rules.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use nf_core::AssetId;

// ────────────────────────────────────────────────────────────────────────────
// Asset type enum
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Scene,
    Prefab,
    Mesh,
    Material,
    Texture,
    Audio,
    DataTable,
    Unknown,
}

// ────────────────────────────────────────────────────────────────────────────
// Asset metadata record
// ────────────────────────────────────────────────────────────────────────────

/// Editor-level metadata attached to every registered asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id:            AssetId,
    /// Project-relative path, e.g. `"Content/Meshes/rock.glb"`.
    pub path:          String,
    pub kind:          AssetKind,
    /// Hash of the source file used to detect stale imports.
    pub import_hash:   u64,
    pub source_path:   Option<String>,
    pub derived_files: Vec<String>,
    pub tags:          Vec<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Registry resource
// ────────────────────────────────────────────────────────────────────────────

/// Global asset registry, available as a Bevy [`Resource`].
#[derive(Resource, Default)]
pub struct AssetRegistry {
    records: Vec<AssetRecord>,
}

impl AssetRegistry {
    pub fn register(&mut self, record: AssetRecord) {
        self.records.push(record);
    }

    pub fn find_by_id(&self, id: AssetId) -> Option<&AssetRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    pub fn find_by_path(&self, path: &str) -> Option<&AssetRecord> {
        self.records.iter().find(|r| r.path == path)
    }

    pub fn all(&self) -> &[AssetRecord] {
        &self.records
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetRegistry>();
    }
}
