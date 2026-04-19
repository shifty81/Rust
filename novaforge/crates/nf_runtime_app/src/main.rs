//! `nf_runtime_app` — shipped game executable.
//!
//! Composes only runtime plugins.  No editor code is linked here.

use bevy::prelude::*;
use nf_game::GamePlugin;
use nf_render::RenderPlugin;
use nf_assets::AssetsPlugin;
use nf_scene::ScenePlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "NovaForge".into(),
                    ..default()
                }),
                ..default()
            }),
            AssetsPlugin,
            ScenePlugin,
            RenderPlugin,
            GamePlugin,
        ))
        .run();
}
