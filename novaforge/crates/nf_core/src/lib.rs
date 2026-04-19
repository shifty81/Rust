//! `nf_core` — shared core types, stable IDs, math helpers, and tags.
//!
//! Every other NovaForge crate depends on this one.  Keep it lean:
//! no Bevy, no heavy deps — just plain Rust.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ────────────────────────────────────────────────────────────────────────────
// Stable entity identity
// ────────────────────────────────────────────────────────────────────────────

/// A globally-unique, stable entity identifier that survives serialization and
/// Bevy entity reuse.  Every scene entity and prefab node carries one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StableId(pub Uuid);

impl StableId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for StableId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Asset identity
// ────────────────────────────────────────────────────────────────────────────

/// A stable, serialisable asset identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub Uuid);

impl AssetId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Editor-visible tags
// ────────────────────────────────────────────────────────────────────────────

/// A free-form tag that can be applied to any entity for filtering/search.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag(pub String);

impl Tag {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Transform data (serde-friendly, no Bevy dep)
// ────────────────────────────────────────────────────────────────────────────

/// A plain-old-data transform stored in scene and prefab files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformData {
    pub translation: [f32; 3],
    pub rotation:    [f32; 4], // quaternion xyzw
    pub scale:       [f32; 3],
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation:    [0.0, 0.0, 0.0, 1.0],
            scale:       [1.0, 1.0, 1.0],
        }
    }
}
