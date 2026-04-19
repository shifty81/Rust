//! `atlas_editor_details` — Details / Inspector panel.
//!
//! * Shows [`EntityLabel`] (editable name), [`Transform`] (editable with undo),
//!   and voxel-specific info for [`VoxelChunk`] entities.
//! * Transform changes are tracked: on drag-release a [`TransformMovedEvent`]
//!   is emitted so `atlas_commands` can push a `MoveTransformCommand` onto the
//!   undo stack.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use atlas_commands::TransformMovedEvent;
use atlas_editor_core::{
    DeleteEntityRequest, DuplicateEntityRequest, EditorMode, EntityLabel,
};
use atlas_selection::FocusedEntity;
use atlas_voxel_planet::{ChunkInfo, ChunkManager, NoiseSeed, VoxelChunk};

// ────────────────────────────────────────────────────────────────────────────
// Component descriptor (reflection bridge)
// ────────────────────────────────────────────────────────────────────────────

/// Describes a single inspectable field inside a component.
pub enum FieldDescriptor {
    Float  { label: &'static str, min: f32, max: f32 },
    Bool   { label: &'static str },
    Color  { label: &'static str },
    String { label: &'static str },
}

/// Registers a component type with the details panel so its fields are editable.
pub struct EditableComponentDescriptor {
    pub type_name: &'static str,
    pub category:  &'static str,
    pub fields:    Vec<FieldDescriptor>,
}

/// Registry of descriptors, populated by each crate via [`DetailsRegistry::register`].
#[derive(Resource, Default)]
pub struct DetailsRegistry {
    pub descriptors: Vec<EditableComponentDescriptor>,
}

impl DetailsRegistry {
    pub fn register(&mut self, desc: EditableComponentDescriptor) {
        self.descriptors.push(desc);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Per-entity transform drag state (tracks "before" for undo)
// ────────────────────────────────────────────────────────────────────────────

/// Stores the transform value captured at the start of a drag so we can
/// emit [`TransformMovedEvent`] when the user releases the drag handle.
#[derive(Resource, Default)]
struct TransformDragState {
    /// The entity whose transform is being dragged.
    entity: Option<Entity>,
    /// The transform value when the drag started.
    before: Transform,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorDetailsPlugin;

impl Plugin for EditorDetailsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DetailsRegistry>()
            .init_resource::<TransformDragState>()
            .add_systems(Update, draw_details_panel);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Panel system
// ────────────────────────────────────────────────────────────────────────────

fn draw_details_panel(
    mut contexts:     EguiContexts,
    focused:          Res<FocusedEntity>,
    registry:         Res<DetailsRegistry>,
    mut labels:       Query<&mut EntityLabel>,
    mut transforms:   Query<&mut Transform>,
    // Voxel-specific queries.
    chunks:           Query<(&VoxelChunk, Option<&ChunkInfo>)>,
    chunk_mgr:        Res<ChunkManager>,
    seed:             Res<NoiseSeed>,
    mode:             Res<State<EditorMode>>,
    mut delete_ev:    EventWriter<DeleteEntityRequest>,
    mut dup_ev:       EventWriter<DuplicateEntityRequest>,
    mut moved_ev:     EventWriter<TransformMovedEvent>,
    mut drag_state:   ResMut<TransformDragState>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("atlas_details")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Details");
            ui.separator();

            let Some(entity) = focused.0 else {
                ui.label("Nothing selected.");
                return;
            };

            // ── Entity name ──────────────────────────────────────────────
            ui.label(format!("Entity: {entity:?}"));
            if let Ok(mut lbl) = labels.get_mut(entity) {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut lbl.0);
                });
            }
            ui.separator();

            // ── Transform (with undo support) ────────────────────────────
            if let Ok(mut tf) = transforms.get_mut(entity) {
                egui::CollapsingHeader::new("Transform")
                    .default_open(true)
                    .show(ui, |ui| {
                        let before = *tf;

                        egui::Grid::new("tf_grid")
                            .num_columns(4)
                            .spacing([4.0, 4.0])
                            .show(ui, |ui| {
                                // ── Translation ──────────────────────────
                                ui.label("Translation");
                                let tx = ui.add(
                                    egui::DragValue::new(&mut tf.translation.x)
                                        .speed(0.1).prefix("X: "),
                                );
                                let ty = ui.add(
                                    egui::DragValue::new(&mut tf.translation.y)
                                        .speed(0.1).prefix("Y: "),
                                );
                                let tz = ui.add(
                                    egui::DragValue::new(&mut tf.translation.z)
                                        .speed(0.1).prefix("Z: "),
                                );
                                ui.end_row();

                                // ── Rotation (Euler YXZ) ─────────────────
                                let (mut yaw, mut pitch, mut roll) =
                                    tf.rotation.to_euler(EulerRot::YXZ);
                                yaw   = yaw.to_degrees();
                                pitch = pitch.to_degrees();
                                roll  = roll.to_degrees();
                                let prev_rot = (yaw, pitch, roll);

                                ui.label("Rotation");
                                let ry = ui.add(
                                    egui::DragValue::new(&mut yaw)
                                        .speed(1.0).suffix("°").prefix("Y: "),
                                );
                                let rx = ui.add(
                                    egui::DragValue::new(&mut pitch)
                                        .speed(1.0).suffix("°").prefix("X: "),
                                );
                                let rz = ui.add(
                                    egui::DragValue::new(&mut roll)
                                        .speed(1.0).suffix("°").prefix("Z: "),
                                );
                                ui.end_row();

                                if (yaw, pitch, roll) != prev_rot {
                                    tf.rotation = Quat::from_euler(
                                        EulerRot::YXZ,
                                        yaw.to_radians(),
                                        pitch.to_radians(),
                                        roll.to_radians(),
                                    );
                                }

                                // ── Scale ────────────────────────────────
                                ui.label("Scale");
                                let sx = ui.add(
                                    egui::DragValue::new(&mut tf.scale.x)
                                        .speed(0.01).prefix("X: "),
                                );
                                let sy = ui.add(
                                    egui::DragValue::new(&mut tf.scale.y)
                                        .speed(0.01).prefix("Y: "),
                                );
                                let sz = ui.add(
                                    egui::DragValue::new(&mut tf.scale.z)
                                        .speed(0.01).prefix("Z: "),
                                );
                                ui.end_row();

                                // ── Undo tracking ────────────────────────
                                // Any drag started: record "before".
                                let any_started = [&tx, &ty, &tz, &ry, &rx, &rz, &sx, &sy, &sz]
                                    .iter()
                                    .any(|r| r.drag_started());
                                if any_started {
                                    // New drag on this (or a new) entity.
                                    if drag_state.entity != Some(entity) || any_started {
                                        drag_state.entity = Some(entity);
                                        drag_state.before = before;
                                    }
                                }

                                // Drag released: emit event so CommandHistory picks it up.
                                let any_released = [&tx, &ty, &tz, &ry, &rx, &rz, &sx, &sy, &sz]
                                    .iter()
                                    .any(|r| r.drag_stopped());
                                if any_released {
                                    if drag_state.entity == Some(entity) {
                                        let after = *tf;
                                        if after != drag_state.before {
                                            moved_ev.send(TransformMovedEvent {
                                                entity,
                                                before: drag_state.before,
                                                after,
                                            });
                                        }
                                        drag_state.entity = None;
                                    }
                                }
                            });
                    });
                ui.separator();
            }

            // ── VoxelChunk info ──────────────────────────────────────────
            if let Ok((chunk, info)) = chunks.get(entity) {
                egui::CollapsingHeader::new("🧱 Voxel Chunk")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("chunk_info_grid")
                            .num_columns(2)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Grid Position");
                                ui.label(format!(
                                    "({}, {}, {})",
                                    chunk.position.x, chunk.position.y, chunk.position.z
                                ));
                                ui.end_row();

                                if let Some(info) = info {
                                    ui.label("Solid Voxels");
                                    ui.label(format!("{}", info.solid_voxel_count));
                                    ui.end_row();

                                    ui.label("Vertices");
                                    ui.label(format!("{}", info.vertex_count));
                                    ui.end_row();
                                }

                                ui.label("Total Loaded");
                                ui.label(format!("{}", chunk_mgr.loaded.len()));
                                ui.end_row();

                                ui.label("World Seed");
                                ui.label(format!("{}", seed.0));
                                ui.end_row();
                            });
                    });
                ui.separator();
            }

            // ── Registered component descriptors ─────────────────────────
            for desc in &registry.descriptors {
                egui::CollapsingHeader::new(desc.type_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        for field in &desc.fields {
                            match field {
                                FieldDescriptor::Float { label, .. } => {
                                    ui.label(format!("{label}: (f32)"));
                                }
                                FieldDescriptor::Bool { label } => {
                                    ui.label(format!("{label}: (bool)"));
                                }
                                FieldDescriptor::Color { label } => {
                                    ui.label(format!("{label}: (color)"));
                                }
                                FieldDescriptor::String { label } => {
                                    ui.label(format!("{label}: (string)"));
                                }
                            }
                        }
                    });
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("⧉ Duplicate  Ctrl+D").clicked() {
                    dup_ev.send(DuplicateEntityRequest(entity));
                }
                if ui.button("🗑 Delete  Del").clicked() {
                    delete_ev.send(DeleteEntityRequest(entity));
                }
            });
            if ui.button("+ Add Component").clicked() {
                // Component-picker popup will be added in a future task.
            }
        });
}
