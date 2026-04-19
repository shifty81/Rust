//! `nf_gizmos` — transform gizmos, grid, and picking helpers.

use bevy::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// Gizmo mode
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Resource)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

/// Whether gizmo axes are in local or world space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Resource)]
pub enum GizmoSpace {
    #[default]
    World,
    Local,
}

// ────────────────────────────────────────────────────────────────────────────
// Snap settings
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct SnapSettings {
    pub translate_snap: f32,
    pub rotate_snap:    f32, // degrees
    pub scale_snap:     f32,
    pub translate_enabled: bool,
    pub rotate_enabled:    bool,
    pub scale_enabled:     bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            translate_snap:    0.25,
            rotate_snap:       15.0,
            scale_snap:        0.1,
            translate_enabled: false,
            rotate_enabled:    false,
            scale_enabled:     false,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct GizmosPlugin;

impl Plugin for GizmosPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<GizmoMode>()
            .init_resource::<GizmoSpace>()
            .init_resource::<SnapSettings>();
    }
}
