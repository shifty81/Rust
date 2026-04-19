//! `nf_editor_play` — Play-In-Editor and Simulate modes.
//!
//! Manages the PIE lifecycle:
//!   1. Serialise the edit world to a temp scene snapshot.
//!   2. Spawn a runtime sub-world with [`GamePlugin`] active.
//!   3. On Stop, destroy the runtime world and restore the edit state.

use bevy::prelude::*;
use nf_editor_core::{EditorMode, RequestEditorMode};

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
            .add_systems(Update, (handle_start, handle_stop, handle_pause));
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Systems
// ────────────────────────────────────────────────────────────────────────────

fn handle_start(
    mut events:    EventReader<StartPie>,
    mut pie_state: ResMut<PieState>,
    mut mode_ev:   EventWriter<RequestEditorMode>,
) {
    for _ev in events.read() {
        if pie_state.active { continue; }
        info!("PIE: starting.");
        // 1. Snapshot edit world (serialization added in Phase 4).
        pie_state.edit_snapshot = Some(vec![]);
        pie_state.active = true;
        mode_ev.send(RequestEditorMode(EditorMode::PlayingInEditor));
    }
}

fn handle_stop(
    mut events:    EventReader<StopPie>,
    mut pie_state: ResMut<PieState>,
    mut mode_ev:   EventWriter<RequestEditorMode>,
) {
    for _ev in events.read() {
        if !pie_state.active { continue; }
        info!("PIE: stopping — restoring edit world.");
        // Restore edit world from snapshot (Phase 4).
        pie_state.edit_snapshot = None;
        pie_state.active = false;
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
