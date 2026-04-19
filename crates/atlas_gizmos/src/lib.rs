//! `atlas_gizmos` — transform gizmos, editor grid, and picking helpers.
//!
//! # Hotkeys (editor mode only, without RMB held)
//! * **W** — Translate mode
//! * **E** — Rotate mode
//! * **R** — Scale mode
//! * **G** — Toggle editor grid
//!
//! # Interactive drag
//! Left-click and drag within ~18 px of a gizmo axis arrow to translate the
//! focused entity along that axis.  A [`TransformMovedEvent`] is emitted on
//! drag-end so the command history can record the move for undo/redo.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use atlas_commands::TransformMovedEvent;
use atlas_editor_core::{EditorCamera, EditorMode};
use atlas_selection::FocusedEntity;

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
// Gizmo interaction state (for drag)
// ────────────────────────────────────────────────────────────────────────────

/// Tracks an in-progress gizmo axis drag.
#[derive(Resource, Default)]
pub struct GizmoInteraction {
    /// True while a drag is in progress.
    pub active: bool,
    /// Which axis is being dragged (0=X, 1=Y, 2=Z).
    pub axis: usize,
    /// Entity being dragged.
    pub entity: Option<Entity>,
    /// Transform of the entity at drag start (for undo).
    pub start_transform: Option<Transform>,
    /// Cursor position in logical pixels at drag start.
    pub drag_start_pos: Vec2,
    /// Screen-space direction of the grabbed axis (normalised pixels).
    pub screen_axis_dir: Vec2,
    /// Pixels per world-unit along the axis (for scaling delta).
    pub pixels_per_unit: f32,
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
            .init_resource::<GizmoInteraction>()
            .add_systems(
                Update,
                (
                    handle_gizmo_hotkeys,
                    handle_gizmo_drag,
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
// Interactive drag — translate along world-space axes
// ────────────────────────────────────────────────────────────────────────────

const GIZMO_AXIS_LEN: f32 = 3.5;
const GIZMO_HIT_PIXELS: f32 = 18.0;

fn handle_gizmo_drag(
    buttons:       Res<ButtonInput<MouseButton>>,
    windows:       Query<&Window, With<PrimaryWindow>>,
    camera_q:      Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    focused:       Res<FocusedEntity>,
    mut transforms: Query<&mut Transform>,
    mut interaction: ResMut<GizmoInteraction>,
    snap:          Res<SnapSettings>,
    gizmo_mode:    Res<GizmoMode>,
    mut moved_ev:  EventWriter<TransformMovedEvent>,
) {
    let Ok(window) = windows.get_single() else { return };
    let Ok((camera, cam_tf)) = camera_q.get_single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // Only drag in Translate mode.
    if *gizmo_mode != GizmoMode::Translate { return; }

    // ── End drag ─────────────────────────────────────────────────────────────
    if interaction.active && buttons.just_released(MouseButton::Left) {
        if let (Some(entity), Some(start_tf)) = (interaction.entity, interaction.start_transform) {
            if let Ok(current_tf) = transforms.get(entity) {
                if *current_tf != start_tf {
                    moved_ev.send(TransformMovedEvent {
                        entity,
                        before: start_tf,
                        after: *current_tf,
                    });
                }
            }
        }
        *interaction = GizmoInteraction::default();
        return;
    }

    // ── Continue drag ─────────────────────────────────────────────────────────
    if interaction.active && buttons.pressed(MouseButton::Left) {
        if let Some(entity) = interaction.entity {
            if let Ok(mut tf) = transforms.get_mut(entity) {
                let cursor_delta = cursor - interaction.drag_start_pos;
                let mag = cursor_delta.dot(interaction.screen_axis_dir);
                if interaction.pixels_per_unit.abs() > 0.001 {
                    let world_delta = mag / interaction.pixels_per_unit;
                    let mut snapped = world_delta;
                    if snap.translate_enabled {
                        let s = snap.translate_snap;
                        snapped = (world_delta / s).round() * s;
                    }
                    let start = interaction.start_transform.unwrap();
                    let axis = [Vec3::X, Vec3::Y, Vec3::Z][interaction.axis];
                    tf.translation = start.translation + axis * snapped;
                }
            }
        }
        return;
    }

    // ── Begin drag on LMB press ───────────────────────────────────────────────
    if !buttons.just_pressed(MouseButton::Left) { return; }
    let Some(entity) = focused.0 else { return };
    let Ok(tf) = transforms.get(entity) else { return };

    let origin = tf.translation;
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];

    for (i, &axis) in axes.iter().enumerate() {
        let tip = origin + axis * GIZMO_AXIS_LEN;
        let Some(screen_origin) = camera.world_to_viewport(cam_tf, origin) else { continue };
        let Some(screen_tip)    = camera.world_to_viewport(cam_tf, tip)    else { continue };

        let seg = screen_tip - screen_origin;
        let seg_len = seg.length();
        if seg_len < 1.0 { continue; }

        // Distance from cursor to the axis line segment.
        let t = ((cursor - screen_origin).dot(seg) / (seg_len * seg_len)).clamp(0.0, 1.0);
        let closest = screen_origin + seg * t;
        let dist = (cursor - closest).length();

        if dist <= GIZMO_HIT_PIXELS {
            let screen_dir = seg / seg_len;
            *interaction = GizmoInteraction {
                active:          true,
                axis:            i,
                entity:          Some(entity),
                start_transform: Some(*tf),
                drag_start_pos:  cursor,
                screen_axis_dir: screen_dir,
                pixels_per_unit: seg_len / GIZMO_AXIS_LEN,
            };
            break;
        }
    }
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
    // In Local mode the axes follow the entity's rotation; in World mode they
    // always point along the world X/Y/Z axes.
    let len    = GIZMO_AXIS_LEN;
    let origin = tf.translation;
    let (ax, ay, az) = match *mode {
        GizmoMode::Translate => (
            tf.rotation * Vec3::X,
            tf.rotation * Vec3::Y,
            tf.rotation * Vec3::Z,
        ),
        // Rotate / Scale always show world-space axes so the interaction
        // arrows are easy to read regardless of entity orientation.
        _ => (Vec3::X, Vec3::Y, Vec3::Z),
    };
    gizmos.line(origin, origin + ax * len, Color::srgb(1.0, 0.15, 0.15));
    gizmos.line(origin, origin + ay * len, Color::srgb(0.15, 1.0, 0.15));
    gizmos.line(origin, origin + az * len, Color::srgb(0.15, 0.15, 1.0));
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
