//! `nf_selection` — centralised selection state shared by all editor panels.

use bevy::prelude::*;
use nf_core::{StableId, AssetId};

// ────────────────────────────────────────────────────────────────────────────
// Marker components (editor-only, never shipped)
// ────────────────────────────────────────────────────────────────────────────

/// Marks an entity as pick-able by the viewport.
#[derive(Component, Default)]
pub struct Selectable;

/// Marks an entity as locked — cannot be selected or moved in the editor.
#[derive(Component, Default)]
pub struct Locked;

/// Marks an entity as hidden in the editor viewport (independent of runtime visibility).
#[derive(Component, Default)]
pub struct HiddenInEditor;

/// Marks an entity as editor-only — stripped before PIE / shipping.
#[derive(Component, Default)]
pub struct EditorOnly;

// ────────────────────────────────────────────────────────────────────────────
// Selection resource
// ────────────────────────────────────────────────────────────────────────────

/// The global selection state.  All panels read from and write to this single
/// resource; never maintain per-panel selection lists.
#[derive(Resource, Default, Debug)]
pub struct SelectionState {
    /// Entities currently selected in the outliner / viewport.
    pub selected_entities: Vec<StableId>,
    /// An asset selected in the content browser (mutually exclusive with entity selection).
    pub selected_asset: Option<AssetId>,
    /// Entity the mouse is currently hovering over in the viewport.
    pub hovered_entity: Option<StableId>,
}

impl SelectionState {
    pub fn select_single(&mut self, id: StableId) {
        self.selected_entities = vec![id];
        self.selected_asset = None;
    }

    pub fn toggle(&mut self, id: StableId) {
        if let Some(pos) = self.selected_entities.iter().position(|e| *e == id) {
            self.selected_entities.remove(pos);
        } else {
            self.selected_entities.push(id);
        }
        self.selected_asset = None;
    }

    pub fn clear(&mut self) {
        self.selected_entities.clear();
        self.selected_asset = None;
        self.hovered_entity = None;
    }

    pub fn is_selected(&self, id: StableId) -> bool {
        self.selected_entities.contains(&id)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

/// Fired when the selection changes so panels can redraw.
#[derive(Event, Debug)]
pub struct SelectionChanged;

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SelectionState>()
            .add_event::<SelectionChanged>();
    }
}
