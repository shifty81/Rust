//! `nf_editor_viewport` — viewport panel: planet-aware editor camera, overlays,
//! picking, and drop targets.
//!
//! The editor camera starts above the voxel planet's north pole and supports
//! right-mouse-button look + WASD fly + scroll-wheel speed adjustment.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::{EditorCamera, EditorMode};
use nf_voxel_planet::PLANET_RADIUS;

// ────────────────────────────────────────────────────────────────────────────
// Editor camera state
// ────────────────────────────────────────────────────────────────────────────

/// Per-frame flight state for the editor free-fly camera.
#[derive(Component)]
pub struct EditorCameraState {
    pub yaw:   f32,
    pub pitch: f32,
    /// Flight speed in m/s (scroll wheel adjusts this).
    pub speed: f32,
}

impl Default for EditorCameraState {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: -0.3, speed: 500.0 }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorViewportPlugin;

impl Plugin for EditorViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_editor_camera)
            .add_systems(
                Update,
                (update_editor_camera, draw_viewport_panel).chain(),
            );
    }
}

fn spawn_editor_camera(mut commands: Commands) {
    // Start above the north pole, tilted slightly to look at the planet surface.
    let start_pos = Vec3::new(0.0, PLANET_RADIUS + 500.0, PLANET_RADIUS * 0.3);
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(start_pos)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        EditorCamera,
        EditorCameraState::default(),
    ));
}

// ────────────────────────────────────────────────────────────────────────────
// Free-fly camera controls
// ────────────────────────────────────────────────────────────────────────────

fn update_editor_camera(
    time:              Res<Time>,
    keyboard:          Res<ButtonInput<KeyCode>>,
    mouse_button:      Res<ButtonInput<MouseButton>>,
    mut motion_events: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    mut cam_q:         Query<(&mut Transform, &mut EditorCameraState), With<EditorCamera>>,
    mode:              Res<State<EditorMode>>,
) {
    if *mode.get() != EditorMode::Editing {
        motion_events.clear();
        scroll_events.clear();
        return;
    }

    let Ok((mut transform, mut state)) = cam_q.get_single_mut() else { return };

    // ── Scroll wheel → adjust speed ─────────────────────────────────────────
    for ev in scroll_events.read() {
        state.speed = (state.speed + ev.y * 50.0).clamp(5.0, 10_000.0);
    }

    // ── RMB held → look ─────────────────────────────────────────────────────
    if mouse_button.pressed(MouseButton::Right) {
        for ev in motion_events.read() {
            state.yaw   -= ev.delta.x * 0.003;
            state.pitch -= ev.delta.y * 0.003;
            state.pitch  = state.pitch.clamp(-1.55, 1.55);
        }
    } else {
        motion_events.clear();
    }

    // Apply accumulated yaw + pitch to rotation.
    transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);

    // ── WASD + Q/E → fly ────────────────────────────────────────────────────
    let fwd   = transform.rotation * Vec3::NEG_Z;
    let right = transform.rotation * Vec3::X;
    let up    = Vec3::Y;

    let mut move_dir = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { move_dir += fwd;   }
    if keyboard.pressed(KeyCode::KeyS) { move_dir -= fwd;   }
    if keyboard.pressed(KeyCode::KeyA) { move_dir -= right; }
    if keyboard.pressed(KeyCode::KeyD) { move_dir += right; }
    if keyboard.pressed(KeyCode::KeyE) { move_dir += up;    }
    if keyboard.pressed(KeyCode::KeyQ) { move_dir -= up;    }

    if move_dir.length_squared() > 0.0 {
        transform.translation +=
            move_dir.normalize() * state.speed * time.delta_seconds();
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Viewport panel (egui overlay)
// ────────────────────────────────────────────────────────────────────────────

fn draw_viewport_panel(
    mut contexts: EguiContexts,
    mode:         Res<State<EditorMode>>,
    cam_q:        Query<(&Transform, &EditorCameraState), With<EditorCamera>>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("[ Viewport — render target will be embedded here ]");

        if let Ok((tf, state)) = cam_q.get_single() {
            let p = tf.translation;
            ui.label(format!(
                "Camera  pos ({:.0}, {:.0}, {:.0})  speed {:.0} m/s  \
                 RMB+drag to look · WASD/QE to fly · scroll to adjust speed",
                p.x, p.y, p.z, state.speed
            ));
        }
    });
}
