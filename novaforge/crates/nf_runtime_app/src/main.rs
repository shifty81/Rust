//! `nf_runtime_app` — shipped game executable.
//!
//! Composes the voxel planet engine and player controller.
//! No editor code is linked here.

use bevy::prelude::*;
use nf_assets::AssetsPlugin;
use nf_render::RenderPlugin;
use nf_scene::ScenePlugin;
use nf_voxel_planet::{PlayerPlugin, VoxelPlanetPlugins};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Voxel Planet".into(),
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
