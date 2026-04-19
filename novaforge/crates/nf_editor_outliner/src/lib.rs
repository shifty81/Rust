//! `nf_editor_outliner` — World Outliner panel.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::EditorMode;
use nf_selection::{SelectionState, SelectionChanged};

// ────────────────────────────────────────────────────────────────────────────
// Outliner-specific entity metadata
// ────────────────────────────────────────────────────────────────────────────

/// Display name shown in the outliner for an entity.
#[derive(Component, Default)]
pub struct EntityLabel(pub String);

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorOutlinerPlugin;

impl Plugin for EditorOutlinerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_outliner_panel);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Panel
// ────────────────────────────────────────────────────────────────────────────

fn draw_outliner_panel(
    mut contexts: EguiContexts,
    entities:     Query<(Entity, Option<&EntityLabel>, Option<&Parent>)>,
    mut selection: ResMut<SelectionState>,
    mut changed:   EventWriter<SelectionChanged>,
    mode:          Res<State<EditorMode>>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("nf_outliner")
        .default_width(240.0)
        .show(ctx, |ui| {
            ui.heading("World Outliner");
            ui.separator();

            // Search bar (state stored in a local — expanded in Phase 2)
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut String::new());
            });
            ui.separator();

            // Entity rows (flat list; hierarchy in Phase 2)
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (entity, label, _parent) in entities.iter() {
                    let name = label
                        .map(|l| l.0.as_str())
                        .unwrap_or("(unnamed)");

                    let row = ui.selectable_label(false, name);
                    if row.clicked() {
                        // Selection integration will use StableId in Phase 2.
                        // For now just clear so the panel responds visually.
                        selection.clear();
                        let _ = entity; // will be wired to StableId lookup
                        changed.send(SelectionChanged);
                    }
                }
            });
        });
}
