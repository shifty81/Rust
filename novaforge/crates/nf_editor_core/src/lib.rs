//! `nf_editor_core` — editor modes, panel contracts, docking layout, shared events.

use bevy::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// Shared entity metadata
// ────────────────────────────────────────────────────────────────────────────

/// Display name shown in the outliner and details panel for an entity.
/// Add this component to any entity that should be visible and named in the editor.
#[derive(Component, Default, Clone)]
pub struct EntityLabel(pub String);

/// Marks the camera used by the editor viewport (not the runtime/game camera).
/// The PIE system deactivates this camera when Play starts and reactivates it
/// when Play stops.
#[derive(Component)]
pub struct EditorCamera;

// ────────────────────────────────────────────────────────────────────────────
// Editor mode state machine
// ────────────────────────────────────────────────────────────────────────────

/// Top-level editor state.  Systems use this to decide whether to run.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EditorMode {
    /// Editor UI is active; scene entities are editable; game systems paused.
    #[default]
    Editing,
    /// Gameplay systems are running; input routed to the runtime world.
    PlayingInEditor,
    /// Gameplay/physics runs but the editor camera stays detached.
    Simulating,
    /// Runtime is frozen; frame stepping is available.
    Paused,
}

// ────────────────────────────────────────────────────────────────────────────
// Panel IDs — used for docking layout and focus routing
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Viewport,
    Outliner,
    Details,
    ContentBrowser,
    OutputLog,
    Scene,
}

// ────────────────────────────────────────────────────────────────────────────
// Shared editor events
// ────────────────────────────────────────────────────────────────────────────

/// Request the editor to enter a different mode.
#[derive(Event, Debug)]
pub struct RequestEditorMode(pub EditorMode);

/// Request a full redraw of all editor panels next frame.
#[derive(Event, Debug)]
pub struct RefreshPanels;

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorCorePlugin;

impl Plugin for EditorCorePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<EditorMode>()
            .add_event::<RequestEditorMode>()
            .add_event::<RefreshPanels>()
            .add_systems(Update, handle_mode_requests);
    }
}

fn handle_mode_requests(
    mut events: EventReader<RequestEditorMode>,
    mut next:   ResMut<NextState<EditorMode>>,
) {
    for ev in events.read() {
        next.set(ev.0);
    }
}
