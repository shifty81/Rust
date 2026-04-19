//! `nf_editor_details` — Details / Inspector panel.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::EditorMode;
use nf_selection::SelectionState;

// ────────────────────────────────────────────────────────────────────────────
// Component descriptor (reflection bridge)
// ────────────────────────────────────────────────────────────────────────────

/// Describes a single inspectable field inside a component.
pub enum FieldDescriptor {
    Float { label: &'static str, min: f32, max: f32 },
    Bool  { label: &'static str },
    Color { label: &'static str },
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
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorDetailsPlugin;

impl Plugin for EditorDetailsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DetailsRegistry>()
            .add_systems(Update, draw_details_panel);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Panel
// ────────────────────────────────────────────────────────────────────────────

fn draw_details_panel(
    mut contexts: EguiContexts,
    selection:    Res<SelectionState>,
    registry:     Res<DetailsRegistry>,
    mode:         Res<State<EditorMode>>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("nf_details")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Details");
            ui.separator();

            if selection.selected_entities.is_empty() {
                ui.label("Nothing selected.");
                return;
            }

            let count = selection.selected_entities.len();
            if count > 1 {
                ui.label(format!("{count} entities selected."));
                return;
            }

            // Single-entity view
            let id = &selection.selected_entities[0];
            ui.label(format!("Entity: {id}"));
            ui.separator();

            // Render descriptor-driven component fields
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
            if ui.button("+ Add Component").clicked() {
                // Component-picker popup will be added in Phase 2.
            }
        });
}
