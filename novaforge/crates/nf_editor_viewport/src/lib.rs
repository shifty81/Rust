//! `nf_editor_viewport` — viewport panel: editor camera, overlays, picking, drop targets.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::EditorMode;

// ────────────────────────────────────────────────────────────────────────────
// Editor camera
// ────────────────────────────────────────────────────────────────────────────

/// Marks the camera used in the editor viewport (not the runtime/game camera).
#[derive(Component)]
pub struct EditorCamera;

/// Flight-camera state for RMB-look + WASD in the viewport.
#[derive(Component)]
pub struct EditorCameraState {
    pub yaw:   f32,
    pub pitch: f32,
    pub speed: f32,
}

impl Default for EditorCameraState {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: 0.0, speed: 10.0 }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorViewportPlugin;

impl Plugin for EditorViewportPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_editor_camera)
            .add_systems(Update, draw_viewport_panel);
    }
}

fn spawn_editor_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle::default(),
        EditorCamera,
        EditorCameraState::default(),
    ));
}

fn draw_viewport_panel(
    mut contexts: EguiContexts,
    mode:         Res<State<EditorMode>>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }
    let ctx = contexts.ctx_mut();
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("[ Viewport — render target will be embedded here ]");
        // TODO: integrate Bevy render-to-texture and blit to egui image.
    });
}
