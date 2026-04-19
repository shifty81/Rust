//! `nf_gizmos` — transform gizmos, editor grid, and picking helpers.
//!
//! # Hotkeys (editor mode only, without RMB held)
//! * **W** — Translate mode
//! * **E** — Rotate mode
//! * **R** — Scale mode
//! * **G** — Toggle editor grid

use bevy::prelude::*;
use nf_editor_core::{EditorCamera, EditorMode};
use nf_selection::FocusedEntity;

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

impl GizmoMode {
    /// Short label shown in the viewport HUD.
    pub fn label(self) -> &'static str {
        match self {
            Self::Translate => "Translate (W)",
            Self::Rotate    => "Rotate (E)",
            Self::Scale     => "Scale (R)",
        }
    }
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
    pub translate_snap:    f32,
    /// Snap angle in degrees.
    pub rotate_snap:       f32,
    pub scale_snap:        f32,
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
// Grid settings
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GridSettings {
    /// Show the editor grid.
    pub visible:    bool,
    /// Distance between grid lines in metres.
    pub cell_size:  f32,
    /// Number of cells on each side of the grid origin.
    pub cell_count: u32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self { visible: true, cell_size: 2.0, cell_count: 20 }
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
            .init_resource::<SnapSettings>()
            .init_resource::<GridSettings>()
            .add_systems(
                Update,
                (
                    handle_gizmo_hotkeys,
                    draw_selection_gizmo,
                    draw_editor_grid,
                )
                    .run_if(in_state(EditorMode::Editing)),
            );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Hotkeys
// ────────────────────────────────────────────────────────────────────────────

fn handle_gizmo_hotkeys(
    keyboard:     Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mode:     ResMut<GizmoMode>,
    mut grid:     ResMut<GridSettings>,
) {
    // W/E/R switch gizmo mode only when the user is not in RMB-fly mode.
    if !mouse_button.pressed(MouseButton::Right) {
        if keyboard.just_pressed(KeyCode::KeyW) { *mode = GizmoMode::Translate; }
        if keyboard.just_pressed(KeyCode::KeyE) { *mode = GizmoMode::Rotate;    }
        if keyboard.just_pressed(KeyCode::KeyR) { *mode = GizmoMode::Scale;     }
    }
    // G toggles the grid in all cases.
    if keyboard.just_pressed(KeyCode::KeyG) { grid.visible = !grid.visible; }
}

// ────────────────────────────────────────────────────────────────────────────
// Selection gizmo — box + axis arrows around the focused entity
// ────────────────────────────────────────────────────────────────────────────

fn draw_selection_gizmo(
    mut gizmos:  Gizmos,
    focused:     Res<FocusedEntity>,
    transforms:  Query<&Transform>,
    mode:        Res<GizmoMode>,
) {
    let Some(entity) = focused.0 else { return };
    let Ok(tf) = transforms.get(entity) else { return };

    // Box colour depends on the active gizmo mode.
    let box_color = match *mode {
        GizmoMode::Translate => Color::srgba(0.9, 0.7, 0.1, 0.85),
        GizmoMode::Rotate    => Color::srgba(0.2, 0.8, 0.2, 0.85),
        GizmoMode::Scale     => Color::srgba(0.9, 0.3, 0.3, 0.85),
    };

    // Draw a 2 m reference box centred on the entity.
    gizmos.cuboid(
        Transform {
            translation: tf.translation,
            rotation:    tf.rotation,
            scale:       Vec3::splat(2.0),
        },
        box_color,
    );

    // Draw RGB axis arrows from the entity origin.
    let len    = 3.5_f32;
    let origin = tf.translation;
    gizmos.line(origin, origin + tf.rotation * Vec3::X * len, Color::srgb(1.0, 0.15, 0.15));
    gizmos.line(origin, origin + tf.rotation * Vec3::Y * len, Color::srgb(0.15, 1.0, 0.15));
    gizmos.line(origin, origin + tf.rotation * Vec3::Z * len, Color::srgb(0.15, 0.15, 1.0));
}

// ────────────────────────────────────────────────────────────────────────────
// Editor grid — drawn tangent to the planet surface at the focused entity or
// editor camera
// ────────────────────────────────────────────────────────────────────────────

fn draw_editor_grid(
    mut gizmos:  Gizmos,
    grid:        Res<GridSettings>,
    focused:     Res<FocusedEntity>,
    transforms:  Query<&Transform>,
    camera_q:    Query<&Transform, With<EditorCamera>>,
) {
    if !grid.visible { return; }

    // Anchor: focused entity > editor camera > origin.
    let anchor: Vec3 = if let Some(entity) = focused.0 {
        transforms.get(entity)
            .map(|tf| tf.translation)
            .unwrap_or_else(|_| camera_anchor(&camera_q))
    } else {
        camera_anchor(&camera_q)
    };

    // Compute a grid plane perpendicular to the surface normal (radial direction).
    let normal = anchor.normalize_or_zero();
    let grid_rotation = if normal.length_squared() > 0.01 {
        Quat::from_rotation_arc(Vec3::Y, normal)
    } else {
        Quat::IDENTITY
    };

    let half  = grid.cell_count as f32 * grid.cell_size * 0.5;
    let count = grid.cell_count;
    let size  = grid.cell_size;
    let color = Color::srgba(0.35, 0.35, 0.40, 0.30);

    // Draw count+1 lines in each direction to form the grid.
    for i in 0..=count {
        let t = i as f32 * size - half;

        let a = anchor + grid_rotation * Vec3::new(-half, 0.0, t);
        let b = anchor + grid_rotation * Vec3::new( half, 0.0, t);
        gizmos.line(a, b, color);

        let c = anchor + grid_rotation * Vec3::new(t, 0.0, -half);
        let d = anchor + grid_rotation * Vec3::new(t, 0.0,  half);
        gizmos.line(c, d, color);
    }
}

fn camera_anchor(camera_q: &Query<&Transform, With<EditorCamera>>) -> Vec3 {
    camera_q.get_single()
        .map(|tf| {
            let pos = tf.translation;
            let dir = pos.normalize_or_zero();
            // Project camera position onto the surface: use the camera height as
            // the grid distance from origin.
            dir * pos.length().max(1.0)
        })
        .unwrap_or(Vec3::ZERO)
}
