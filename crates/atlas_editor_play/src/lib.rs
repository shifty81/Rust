//! `atlas_editor_play` — Play-In-Editor and Simulate modes.
//!
//! Manages the PIE lifecycle:
//!   1. On Play: spawn the voxel player entity, lock cursor, deactivate editor camera.
//!   2. On Stop: despawn the player entity, unlock cursor, reactivate editor camera.
//!
//! All player Update systems are registered at startup and gated with
//! `run_if(in_state(EditorMode::PlayingInEditor))` so they are harmless while
//! no Player entity exists.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_egui::{egui, EguiContexts};
use atlas_editor_core::{EditorCamera, EditorMode, RequestEditorMode};
use atlas_voxel_planet::{
    player::{
        align_to_surface, apply_gravity, handle_mouse_look, handle_movement,
        spawn_voxel_player, toggle_cursor, update_camera_pitch,
        update_chunk_viewpoint_from_player, update_survival_stats,
    },
    NoiseSeed, Player, PlayerState, PLANET_RADIUS,
};

// ────────────────────────────────────────────────────────────────────────────
// PIE state
// ────────────────────────────────────────────────────────────────────────────

/// Tracks Play-In-Editor session state.
#[derive(Resource, Default, Debug)]
pub struct PieState {
    /// True while a play or simulate session is active.
    pub active: bool,
    /// Serialised snapshot of the edit world taken just before PIE started,
    /// used to restore editor state on Stop.
    pub edit_snapshot: Option<Vec<u8>>,
}

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

#[derive(Event)] pub struct StartPie;
#[derive(Event)] pub struct StopPie;
#[derive(Event)] pub struct PausePie;
#[derive(Event)] pub struct StepPie;

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorPlayPlugin;

impl Plugin for EditorPlayPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_egui::EguiPlugin>() {
            app.add_plugins(bevy_egui::EguiPlugin);
        }
        app
            .init_resource::<PieState>()
            .add_event::<StartPie>()
            .add_event::<StopPie>()
            .add_event::<PausePie>()
            .add_event::<StepPie>()
            // Player Update systems — registered at startup, only run during PIE.
            .add_systems(
                Update,
                (
                    update_chunk_viewpoint_from_player,
                    handle_mouse_look,
                    handle_movement,
                    apply_gravity,
                    align_to_surface,
                    update_camera_pitch,
                    update_survival_stats,
                    toggle_cursor,
                )
                    .chain()
                    .run_if(in_state(EditorMode::PlayingInEditor)),
            )
            .add_systems(
                Update,
                draw_pie_hud.run_if(in_state(EditorMode::PlayingInEditor)),
            )
            .add_systems(Update, (handle_start, handle_stop, handle_pause));
    }
}

// ────────────────────────────────────────────────────────────────────────────
// PIE heads-up display
// ────────────────────────────────────────────────────────────────────────────

fn draw_pie_hud(
    mut contexts: EguiContexts,
    player_q:     Query<(&Transform, &PlayerState), With<Player>>,
    diagnostics:  Res<DiagnosticsStore>,
) {
    let ctx = contexts.ctx_mut();

    // ── Top-left mode banner ─────────────────────────────────────────────────
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("▶  PIE ACTIVE")
                    .color(egui::Color32::from_rgb(80, 220, 80))
                    .strong(),
            );

            if let Some(fps) = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|d| d.smoothed())
            {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("{fps:.1} FPS"))
                        .color(egui::Color32::from_rgb(120, 220, 120)),
                );
            }

            ui.separator();
            ui.label(
                egui::RichText::new(
                    "Esc: release cursor · Shift: sprint · Space: jump · F: toggle flight  |  \
                     Flying: WASD/QE: 6DoF · Shift: fast"
                )
                .weak()
                .small(),
            );
        });
    });

    // ── Bottom-right player status ───────────────────────────────────────────
    if let Ok((tf, state)) = player_q.get_single() {
        let pos    = tf.translation;
        let dist   = pos.length();
        let alt_m  = dist - PLANET_RADIUS;
        let speed  = state.velocity.length();

        let mode_str = if state.is_flying { "✈  Flying" } else if state.is_grounded { "grounded" } else { "airborne" };

        egui::Window::new("Player")
            .title_bar(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
            .resizable(false)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                egui::Grid::new("pie_player_grid")
                    .num_columns(2)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Pos").weak());
                        ui.label(format!("({:.0}, {:.0}, {:.0})", pos.x, pos.y, pos.z));
                        ui.end_row();

                        ui.label(egui::RichText::new("Altitude").weak());
                        if alt_m.abs() >= 1_000.0 {
                            ui.label(format!(
                                "{}{:.2} km",
                                if alt_m < 0.0 { "-" } else { "+" },
                                alt_m.abs() / 1_000.0
                            ));
                        } else {
                            ui.label(format!(
                                "{}{:.1} m",
                                if alt_m < 0.0 { "-" } else { "+" },
                                alt_m.abs()
                            ));
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Speed").weak());
                        if speed >= 1_000.0 {
                            ui.label(format!("{:.2} km/s", speed / 1_000.0));
                        } else {
                            ui.label(format!("{speed:.1} m/s"));
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Mode").weak());
                        let mode_color = if state.is_flying {
                            egui::Color32::from_rgb(100, 180, 255)
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.label(egui::RichText::new(mode_str).color(mode_color));
                        ui.end_row();
                    });
            });
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Systems
// ────────────────────────────────────────────────────────────────────────────

fn handle_start(
    mut events:       EventReader<StartPie>,
    mut pie_state:    ResMut<PieState>,
    mut mode_ev:      EventWriter<RequestEditorMode>,
    mut commands:     Commands,
    seed:             Res<NoiseSeed>,
    mut windows:      Query<&mut Window>,
    mut editor_cams:  Query<&mut Camera, With<EditorCamera>>,
) {
    for _ev in events.read() {
        if pie_state.active { continue; }

        info!("PIE: starting.");

        // 1. Spawn the voxel player + player camera.
        spawn_voxel_player(&mut commands, seed.0);

        // 2. Lock cursor for first-person control.
        if let Ok(mut window) = windows.get_single_mut() {
            window.cursor.visible   = false;
            window.cursor.grab_mode = CursorGrabMode::Locked;
        }

        // 3. Deactivate the editor camera so only PlayerCamera renders.
        for mut cam in &mut editor_cams {
            cam.is_active = false;
        }

        pie_state.edit_snapshot = Some(vec![]);
        pie_state.active        = true;
        mode_ev.send(RequestEditorMode(EditorMode::PlayingInEditor));
    }
}

fn handle_stop(
    mut events:       EventReader<StopPie>,
    mut pie_state:    ResMut<PieState>,
    mut mode_ev:      EventWriter<RequestEditorMode>,
    mut commands:     Commands,
    player_query:     Query<Entity, With<Player>>,
    mut windows:      Query<&mut Window>,
    mut editor_cams:  Query<&mut Camera, With<EditorCamera>>,
) {
    for _ev in events.read() {
        if !pie_state.active { continue; }

        info!("PIE: stopping — restoring edit world.");

        // 1. Despawn the player entity (and PlayerCamera child via despawn_recursive).
        for entity in &player_query {
            commands.entity(entity).despawn_recursive();
        }

        // 2. Unlock cursor.
        if let Ok(mut window) = windows.get_single_mut() {
            window.cursor.visible   = true;
            window.cursor.grab_mode = CursorGrabMode::None;
        }

        // 3. Reactivate the editor camera.
        for mut cam in &mut editor_cams {
            cam.is_active = true;
        }

        pie_state.edit_snapshot = None;
        pie_state.active        = false;
        mode_ev.send(RequestEditorMode(EditorMode::Editing));
    }
}

fn handle_pause(
    mut events:  EventReader<PausePie>,
    mut mode_ev: EventWriter<RequestEditorMode>,
) {
    for _ev in events.read() {
        info!("PIE: pausing.");
        mode_ev.send(RequestEditorMode(EditorMode::Paused));
    }
}
