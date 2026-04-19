//! `atlas_runtime_app` — shipped game executable.
//!
//! Composes the voxel planet engine and player controller.
//! No editor code is linked here.

use bevy::prelude::*;
use atlas_assets::AssetsPlugin;
use atlas_render::RenderPlugin;
use atlas_scene::ScenePlugin;
use atlas_voxel_planet::{PlayerPlugin, VoxelPlanetPlugins};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Atlas Engine — Voxel Planet".into(),
                    ..default()
                }),
                ..default()
            }),
            AssetsPlugin,
            ScenePlugin,
            RenderPlugin,
            // World: solar system + planet terrain + atmosphere + vegetation.
            VoxelPlanetPlugins,
            // First-person player controller.
            PlayerPlugin,
        ))
        .run();
}
