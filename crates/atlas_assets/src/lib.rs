//! `atlas_assets` — asset registry, metadata, and RON content loaders.
//!
//! This crate is the bridge between authored **content** on disk
//! (`assets/Content/<Category>/*.ron`) and the Bevy asset system that serves
//! it to gameplay code.  It owns:
//!
//! * The generic [`RonAssetLoader`] — one implementation, reused for every
//!   content type.
//! * One typed asset per content category ([`RecipeAsset`] today; biomes,
//!   voxels, prefabs, … in follow-up PRs).
//! * [`AssetsPlugin`] — registers the Bevy `Asset`s and their loaders so the
//!   editor and any future runtime binary can consume the same content pack.
//!
//! An editor-level [`AssetRegistry`] also lives here, tracking project-level
//! metadata (import hashes, tags, derived files).  That's separate from
//! Bevy's asset server and is used by the content-browser panel.

pub mod recipe;
pub mod ron_loader;

pub use recipe::{RecipeAsset, RecipeIngredient, RecipeOutput};
pub use ron_loader::{RonAssetLoader, RonLoaderError};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use atlas_core::AssetId;

// ────────────────────────────────────────────────────────────────────────────
// Editor-side asset type enum (used by AssetRegistry)
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
    Recipe,
    Biome,
    Voxel,
    Creature,
    Character,
    Quest,
    Structure,
    Planet,
    Unknown,
}

// ────────────────────────────────────────────────────────────────────────────
// Asset metadata record
// ────────────────────────────────────────────────────────────────────────────

/// Editor-level metadata attached to every registered asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id:            AssetId,
    /// Project-relative path, e.g. `"Content/Recipes/compressed_stone.recipe.ron"`.
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
        app.init_resource::<AssetRegistry>()
            // Register every typed content asset + its loader here.  Adding
            // a new content category means: define the `Asset` struct in a
            // sibling module, then register one more pair of lines below.
            .init_asset::<RecipeAsset>()
            .register_asset_loader(RonAssetLoader::<RecipeAsset>::new(&["recipe.ron"]));
    }
}
