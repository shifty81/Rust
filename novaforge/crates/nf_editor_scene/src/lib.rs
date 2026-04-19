//! `nf_editor_scene` — scene panel: open/save/new, dirty-state tracking, recent scenes.

use bevy::prelude::*;
use nf_scene::{SceneDirty, ActiveScenePath};
use nf_editor_core::EditorMode;

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

/// Request to open a scene from the given path.
#[derive(Event)]
pub struct OpenSceneRequest(pub std::path::PathBuf);

/// Request to save the current scene (to its existing path, or prompt if new).
#[derive(Event)]
pub struct SaveSceneRequest;

/// Request to create a blank new scene (prompt to save if dirty first).
#[derive(Event)]
pub struct NewSceneRequest;

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorScenePlugin;

impl Plugin for EditorScenePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<OpenSceneRequest>()
            .add_event::<SaveSceneRequest>()
            .add_event::<NewSceneRequest>()
            .add_systems(
                Update,
                (handle_open, handle_save, handle_new)
                    .run_if(in_state(EditorMode::Editing)),
            );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Systems
// ────────────────────────────────────────────────────────────────────────────

fn handle_open(
    mut events:      EventReader<OpenSceneRequest>,
    mut active_path: ResMut<ActiveScenePath>,
    mut dirty:       ResMut<SceneDirty>,
) {
    for ev in events.read() {
        info!("Opening scene: {}", ev.0.display());
        active_path.0 = Some(ev.0.clone());
        dirty.0 = false;
        // Actual scene deserialization and entity spawning will be added here.
    }
}

fn handle_save(
    mut events:  EventReader<SaveSceneRequest>,
    active_path: Res<ActiveScenePath>,
    mut dirty:   ResMut<SceneDirty>,
) {
    for _ev in events.read() {
        if let Some(path) = &active_path.0 {
            info!("Saving scene: {}", path.display());
            dirty.0 = false;
            // Actual scene serialization will be added here.
        } else {
            warn!("Save requested but no active scene path — prompt user.");
        }
    }
}

fn handle_new(
    mut events: EventReader<NewSceneRequest>,
    mut dirty:  ResMut<SceneDirty>,
    mut path:   ResMut<ActiveScenePath>,
) {
    for _ev in events.read() {
        info!("Creating new scene.");
        path.0 = None;
        dirty.0 = false;
        // Entity despawning and blank world setup will be added here.
    }
}
