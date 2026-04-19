//! `nf_render` — render setup and game-view configuration.

use bevy::prelude::*;

/// Marker for the editor viewport's render target camera.
#[derive(Component)]
pub struct GameViewCamera;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, _app: &mut App) {
        // Render pipeline configuration and post-process setup will be added here.
    }
}
