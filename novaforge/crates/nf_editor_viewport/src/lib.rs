//! `nf_editor_viewport` — viewport panel: planet-aware editor camera, overlays,
//! picking, and drop targets.
//!
//! The editor camera starts above the voxel planet's north pole and supports
//! right-mouse-button look + WASD fly (RMB required) + scroll-wheel speed.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::{EditorCamera, EditorMode};
use nf_gizmos::GizmoMode;
use nf_voxel_planet::{ChunkViewpoint, PLANET_RADIUS};

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
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.add_systems(Startup, spawn_editor_camera)
            .add_systems(
                Update,
                (
                    update_chunk_viewpoint_from_editor_camera,
                    update_editor_camera,
                    draw_viewport_panel,
                )
                    .chain()
                    .run_if(in_state(EditorMode::Editing)),
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
// Keep ChunkViewpoint in sync with the editor camera
// ────────────────────────────────────────────────────────────────────────────

fn update_chunk_viewpoint_from_editor_camera(
    cam_q:         Query<&Transform, With<EditorCamera>>,
    mut viewpoint: ResMut<ChunkViewpoint>,
) {
    if let Ok(tf) = cam_q.get_single() {
        viewpoint.0 = tf.translation;
    }
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
) {
    let Ok((mut transform, mut state)) = cam_q.get_single_mut() else { return };
    let rmb = mouse_button.pressed(MouseButton::Right);

    // ── Scroll wheel → adjust speed ─────────────────────────────────────────
    for ev in scroll_events.read() {
        state.speed = (state.speed + ev.y * 50.0).clamp(5.0, 10_000.0);
    }

    // ── RMB held → look ─────────────────────────────────────────────────────
    if rmb {
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

    // ── WASD + Q/E → fly (only while RMB is held) ───────────────────────────
    if rmb {
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
}

// ────────────────────────────────────────────────────────────────────────────
// Viewport panel (egui overlay)
// ────────────────────────────────────────────────────────────────────────────

fn draw_viewport_panel(
    mut contexts: EguiContexts,
    cam_q:        Query<(&Transform, &EditorCameraState), With<EditorCamera>>,
    diagnostics:  Res<DiagnosticsStore>,
    gizmo_mode:   Res<GizmoMode>,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        // ── FPS / performance readout ─────────────────────────────────────
        let fps = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.smoothed())
            .unwrap_or(0.0);
        let frame_ms = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
            .and_then(|d| d.smoothed())
            .unwrap_or(0.0)
            * 1000.0;

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{fps:.1} FPS  ({frame_ms:.2} ms)"))
                    .color(egui::Color32::from_rgb(120, 220, 120)),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(gizmo_mode.label())
                    .color(egui::Color32::from_rgb(220, 180, 80)),
            );
            ui.separator();
            ui.label(egui::RichText::new("RMB+drag: look · WASD/QE: fly · scroll: speed")
                .weak());
        });

        // ── Camera position ───────────────────────────────────────────────
        if let Ok((tf, state)) = cam_q.get_single() {
            let p = tf.translation;
            ui.label(format!(
                "Camera  ({:.0}, {:.0}, {:.0})  speed {:.0} m/s",
                p.x, p.y, p.z, state.speed
            ));
        }

        ui.label(egui::RichText::new("W/E/R: translate/rotate/scale · G: toggle grid")
            .weak().small());
    });
}
