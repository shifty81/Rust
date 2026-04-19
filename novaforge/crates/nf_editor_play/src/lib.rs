//! `nf_editor_play` — Play-In-Editor and Simulate modes.
//!
//! Manages the PIE lifecycle:
//!   1. On Play: spawn the voxel player entity, lock cursor, deactivate editor camera.
//!   2. On Stop: despawn the player entity, unlock cursor, reactivate editor camera.
//!
//! All player Update systems are registered at startup and gated with
//! `run_if(in_state(EditorMode::PlayingInEditor))` so they are harmless while
//! no Player entity exists.

use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use nf_editor_core::{EditorCamera, EditorMode, RequestEditorMode};
use nf_voxel_planet::{
    player::{
        align_to_surface, apply_gravity, handle_mouse_look, handle_movement,
        spawn_voxel_player, toggle_cursor, update_camera_pitch,
    },
    NoiseSeed, Player,
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
                    handle_mouse_look,
                    handle_movement,
                    apply_gravity,
                    align_to_surface,
                    update_camera_pitch,
                    toggle_cursor,
                )
                    .chain()
                    .run_if(in_state(EditorMode::PlayingInEditor)),
            )
            .add_systems(Update, (handle_start, handle_stop, handle_pause));
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
