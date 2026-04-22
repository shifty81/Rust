//! Generic RON-backed [`bevy::asset::Asset`] loader.
//!
//! Nova-Forge's content library stores all tunable data — recipes, biomes,
//! voxels, prefabs, creatures, … — as typed RON files under
//! `assets/Content/<Category>/*.<ext>.ron`.  They are loaded by Bevy's asset
//! system, which gives us:
//!
//! * **Hot-reload** — edits to any RON file fire `AssetEvent::Modified` so
//!   gameplay systems can live-update their runtime tables without a
//!   25-minute engine rebuild.
//! * **Handles & ref-counting** — the same content identity can be shared by
//!   every system that needs it.
//! * **Error reporting** — RON parse errors surface through Bevy's standard
//!   asset error pipeline and appear in the editor Output Log.
//!
//! # Registering a new content type
//!
//! ```ignore
//! use atlas_assets::RonAssetLoader;
//! use bevy::prelude::*;
//!
//! #[derive(bevy::asset::Asset, bevy::reflect::TypePath, serde::Deserialize)]
//! pub struct MyThingAsset { /* ... */ }
//!
//! app.init_asset::<MyThingAsset>()
//!     .register_asset_loader(RonAssetLoader::<MyThingAsset>::new(&["thing.ron"]));
//! ```

use std::marker::PhantomData;

use bevy::asset::{io::Reader, Asset, AssetLoader, AsyncReadExt, LoadContext};
use serde::Deserialize;
use thiserror::Error;

// ────────────────────────────────────────────────────────────────────────────
// Error type
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RonLoaderError {
    #[error("could not read RON asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse RON asset: {0}")]
    Ron(#[from] ron::error::SpannedError),
}

// ────────────────────────────────────────────────────────────────────────────
// Generic loader
// ────────────────────────────────────────────────────────────────────────────

/// Loads a typed [`Asset`] from a RON source file.
///
/// `EXTENSIONS` is the list of file extensions this loader should claim.
/// By convention Nova-Forge uses compound extensions like `recipe.ron`,
/// `biome.ron`, `voxel.ron` so each content kind is unambiguous on disk.
pub struct RonAssetLoader<T: Asset + for<'de> Deserialize<'de>> {
    extensions: &'static [&'static str],
    _phantom: PhantomData<fn() -> T>,
}

impl<T: Asset + for<'de> Deserialize<'de>> RonAssetLoader<T> {
    pub const fn new(extensions: &'static [&'static str]) -> Self {
        Self {
            extensions,
            _phantom: PhantomData,
        }
    }
}

impl<T: Asset + for<'de> Deserialize<'de>> AssetLoader for RonAssetLoader<T> {
    type Asset = T;
    type Settings = ();
    type Error = RonLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<T, RonLoaderError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let value: T = ron::de::from_bytes(&bytes)?;
        Ok(value)
    }

    fn extensions(&self) -> &[&str] {
        self.extensions
    }
}
