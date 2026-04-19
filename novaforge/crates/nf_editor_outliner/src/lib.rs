//! `nf_editor_outliner` — World Outliner panel.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::{EditorMode, EntityLabel};
use nf_selection::{SelectionChanged, FocusedEntity};

// ────────────────────────────────────────────────────────────────────────────
// Local outliner state (search string, persists across frames)
// ────────────────────────────────────────────────────────────────────────────

/// Persistent state for the World Outliner panel.
///
/// The hierarchy ordered list and entity set are rebuilt each frame for
/// simplicity.  Phase 3 will cache these and only rebuild on structural
/// changes (entity added/removed/reparented).
#[derive(Resource, Default)]
pub struct OutlinerState {
    pub search: String,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorOutlinerPlugin;

impl Plugin for EditorOutlinerPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<OutlinerState>()
            .add_systems(Update, draw_outliner_panel);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Hierarchy helpers
// ────────────────────────────────────────────────────────────────────────────

/// Recursively collect (entity, depth) pairs in hierarchy order.
fn collect_subtree(
    entity: Entity,
    depth:  usize,
    children_q: &Query<&Children>,
    out: &mut Vec<(Entity, usize)>,
) {
    out.push((entity, depth));
    if let Ok(children) = children_q.get(entity) {
        for &child in children.iter() {
            collect_subtree(child, depth + 1, children_q, out);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Panel
// ────────────────────────────────────────────────────────────────────────────

fn draw_outliner_panel(
    mut contexts:  EguiContexts,
    mut state:     ResMut<OutlinerState>,
    entities:      Query<(Entity, Option<&EntityLabel>, Option<&Parent>)>,
    children_q:    Query<&Children>,
    mut focused:   ResMut<FocusedEntity>,
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

            // ── Search bar ───────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut state.search);
                if !state.search.is_empty() && ui.small_button("✕").clicked() {
                    state.search.clear();
                }
            });
            ui.separator();

            let filter = state.search.to_lowercase();

            // ── Build hierarchy ordered list ─────────────────────────────
            // Collect root entities (no parent, or parent not in our query set).
            let entity_set: std::collections::HashSet<Entity> =
                entities.iter().map(|(e, _, _)| e).collect();

            let roots: Vec<Entity> = entities
                .iter()
                .filter_map(|(e, _, parent)| {
                    if parent.map_or(true, |p| !entity_set.contains(&p.get())) {
                        Some(e)
                    } else {
                        None
                    }
                })
                .collect();

            let mut ordered: Vec<(Entity, usize)> = Vec::new();
            for root in roots {
                collect_subtree(root, 0, &children_q, &mut ordered);
            }

            // ── Entity rows ──────────────────────────────────────────────
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (entity, depth) in &ordered {
                    let entity = *entity;
                    let depth  = *depth;

                    let label_str = entities
                        .get(entity)
                        .ok()
                        .and_then(|(_, lbl, _)| lbl.map(|l| l.0.clone()))
                        .unwrap_or_else(|| format!("Entity({:?})", entity));

                    // Apply search filter
                    if !filter.is_empty()
                        && !label_str.to_lowercase().contains(&filter)
                    {
                        continue;
                    }

                    let is_focused = focused.0 == Some(entity);

                    ui.horizontal(|ui| {
                        // Indent by depth
                        if depth > 0 {
                            ui.add_space(depth as f32 * 16.0);
                        }

                        let row = ui.selectable_label(is_focused, &label_str);
                        if row.clicked() {
                            focused.0 = if is_focused { None } else { Some(entity) };
                            changed.send(SelectionChanged);
                        }
                    });
                }
            });
        });
}
