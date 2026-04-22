//! `RecipeAsset` — crafting recipe loaded from `*.recipe.ron`.
//!
//! Recipes are the smallest, leaf-most content type, and so are the first to
//! migrate from hardcoded Rust (`atlas_voxel_planet::crafting::RECIPES`) into
//! project content.  Editing a `.recipe.ron` on disk while the editor is
//! running triggers Bevy's file-watcher, the [`RecipeAsset`] handle updates,
//! and the crafting panel rebuilds.
//!
//! # Schema
//!
//! ```ron
//! (
//!     name: "Compressed Stone",
//!     ingredients: [
//!         (voxel: "Gravel", count: 3),
//!     ],
//!     output: (voxel: "Stone", count: 2),
//! )
//! ```
//!
//! The `voxel` field is a string name matching the [`atlas_voxel_planet::biome::Voxel`]
//! enum variant.  When voxels are themselves migrated to RON (see
//! `assets/Content/Voxels/`), this will change to a stable `StableId` /
//! `AssetPath` reference instead of an enum name.

use bevy::asset::Asset;
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

/// A single ingredient line on a recipe: a voxel name + how many the player
/// must consume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    /// Voxel enum-variant name (e.g. `"Gravel"`).  Must match a variant of
    /// `atlas_voxel_planet::biome::Voxel`.  See module docs on future
    /// migration to stable IDs.
    pub voxel: String,
    pub count: u32,
}

/// The output of a recipe — one voxel type, produced `count` times.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeOutput {
    pub voxel: String,
    pub count: u32,
}

/// A crafting recipe loaded from a `*.recipe.ron` file.
///
/// Implements [`Asset`] so it participates in Bevy's handle / ref-counting /
/// hot-reload pipeline.  Consumers (the crafting panel, the editor's content
/// browser, …) react to `AssetEvent::<RecipeAsset>` events.
#[derive(Debug, Clone, Asset, TypePath, Serialize, Deserialize)]
pub struct RecipeAsset {
    /// Human-readable display name shown in the crafting panel.
    pub name: String,
    /// Ingredients consumed by the recipe.  All must be satisfied.
    pub ingredients: Vec<RecipeIngredient>,
    /// What the recipe produces.
    pub output: RecipeOutput,
}
