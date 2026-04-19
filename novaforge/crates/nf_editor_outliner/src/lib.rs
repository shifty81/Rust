//! `nf_editor_outliner` — World Outliner panel with voxel-planet grouping.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::{EditorMode, EntityLabel};
use nf_selection::{FocusedEntity, SelectionChanged};
use nf_voxel_planet::{
    ChunkManager, GrassDecoration, Moon, Planet, Player, Sun, Tree, VoxelChunk, WeatherParticle,
    WeatherState,
};

// ────────────────────────────────────────────────────────────────────────────
// Outliner state
// ────────────────────────────────────────────────────────────────────────────

/// Persistent state for the World Outliner panel.
#[derive(Resource, Default)]
pub struct OutlinerState {
    pub search: String,
}

/// Counts of voxel-category entities, collected each frame before drawing.
#[derive(Resource, Default)]
struct VoxelCounts {
    suns:      usize,
    moons:     usize,
    planets:   Vec<Entity>,
    chunks:    Vec<Entity>,
    trees:     usize,
    grasses:   usize,
    particles: usize,
    players:   Vec<Entity>,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorOutlinerPlugin;

impl Plugin for EditorOutlinerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OutlinerState>()
            .init_resource::<VoxelCounts>()
            .add_systems(
                Update,
                (collect_voxel_counts, draw_outliner_panel).chain(),
            );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Count collection (runs before drawing)
// ────────────────────────────────────────────────────────────────────────────

fn collect_voxel_counts(
    mut counts:  ResMut<VoxelCounts>,
    sun_q:       Query<Entity, With<Sun>>,
    moon_q:      Query<Entity, With<Moon>>,
    planet_q:    Query<Entity, With<Planet>>,
    chunk_q:     Query<Entity, With<VoxelChunk>>,
    tree_q:      Query<Entity, With<Tree>>,
    grass_q:     Query<Entity, With<GrassDecoration>>,
    particle_q:  Query<Entity, With<WeatherParticle>>,
    player_q:    Query<Entity, With<Player>>,
) {
    counts.suns      = sun_q.iter().count();
    counts.moons     = moon_q.iter().count();
    counts.planets   = planet_q.iter().collect();
    counts.chunks    = chunk_q.iter().collect();
    counts.trees     = tree_q.iter().count();
    counts.grasses   = grass_q.iter().count();
    counts.particles = particle_q.iter().count();
    counts.players   = player_q.iter().collect();
}

// ────────────────────────────────────────────────────────────────────────────
// Hierarchy helpers
// ────────────────────────────────────────────────────────────────────────────

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
    mut contexts: EguiContexts,
    mut state:    ResMut<OutlinerState>,
    counts:       Res<VoxelCounts>,
    entities:     Query<(Entity, Option<&EntityLabel>, Option<&Parent>)>,
    children_q:   Query<&Children>,
    mut focused:  ResMut<FocusedEntity>,
    mut changed:  EventWriter<SelectionChanged>,
    mode:         Res<State<EditorMode>>,
    chunk_mgr:    Res<ChunkManager>,
    weather:      Res<WeatherState>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("nf_outliner")
        .default_width(260.0)
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

            egui::ScrollArea::vertical().show(ui, |ui| {
                // ── 🌍 Solar System ──────────────────────────────────────
                egui::CollapsingHeader::new(format!(
                    "🌍 Solar System  (sun:{} moon:{})",
                    counts.suns, counts.moons,
                ))
                .default_open(true)
                .show(ui, |ui| {
                    // Show all root entities that look like solar-system bodies.
                    for (entity, lbl, parent) in &entities {
                        if parent.is_some() { continue; }
                        let name = lbl.map(|l| l.0.as_str()).unwrap_or("");
                        if ["Sun","Moon","Mercury","Venus","Mars","Jupiter",
                            "Saturn","Uranus","Neptune","SunLight"]
                            .iter().any(|p| name.contains(p))
                        {
                            entity_row(ui, entity, name, &mut focused, &mut changed, &filter);
                        }
                    }
                });

                ui.separator();

                // ── 🗺 Planet ────────────────────────────────────────────
                egui::CollapsingHeader::new("🗺  Planet")
                    .default_open(true)
                    .show(ui, |ui| {
                        for &entity in &counts.planets {
                            entity_row(ui, entity, "Planet", &mut focused, &mut changed, &filter);
                        }
                    });

                ui.separator();

                // ── 🧱 Chunks ────────────────────────────────────────────
                egui::CollapsingHeader::new(format!(
                    "🧱  Chunks  ({} loaded, {} pending)",
                    chunk_mgr.loaded.len(), chunk_mgr.pending.len(),
                ))
                .default_open(false)
                .show(ui, |ui| {
                    for &entity in &counts.chunks {
                        entity_row(ui, entity, "Chunk", &mut focused, &mut changed, &filter);
                    }
                });

                ui.separator();

                // ── 🌲 Vegetation ────────────────────────────────────────
                egui::CollapsingHeader::new(format!(
                    "🌲  Vegetation  ({} trees, {} grass)",
                    counts.trees, counts.grasses,
                ))
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(format!(
                        "{} tree entities, {} grass entities",
                        counts.trees, counts.grasses
                    ));
                });

                ui.separator();

                // ── ⛅ Weather ────────────────────────────────────────────
                let wlabel = match weather.kind {
                    nf_voxel_planet::WeatherKind::Clear  => "Clear",
                    nf_voxel_planet::WeatherKind::Cloudy => "Cloudy",
                    nf_voxel_planet::WeatherKind::Rain   => "Rain",
                    nf_voxel_planet::WeatherKind::Snow   => "Snow",
                    nf_voxel_planet::WeatherKind::Storm  => "Storm",
                };
                egui::CollapsingHeader::new(format!(
                    "⛅  Weather  {} ({} particles)",
                    wlabel, counts.particles,
                ))
                .default_open(false)
                .show(ui, |_ui| {});

                // ── 👤 Player (PIE only) ─────────────────────────────────
                if !counts.players.is_empty() {
                    ui.separator();
                    egui::CollapsingHeader::new("👤  Player (PIE)")
                        .default_open(true)
                        .show(ui, |ui| {
                            for &entity in &counts.players {
                                entity_row(ui, entity, "Player", &mut focused, &mut changed, &filter);
                            }
                        });
                }

                ui.separator();

                // ── Generic entity hierarchy ─────────────────────────────
                // Collect voxel entities to skip them in the "Other" section.
                let voxel_set: std::collections::HashSet<Entity> = counts
                    .planets.iter()
                    .chain(counts.chunks.iter())
                    .chain(counts.players.iter())
                    .copied()
                    .collect();

                egui::CollapsingHeader::new("📋  Other Entities")
                    .default_open(false)
                    .show(ui, |ui| {
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

                        for (entity, depth) in &ordered {
                            let entity = *entity;
                            if voxel_set.contains(&entity) { continue; }

                            // Skip solar-system named entities already shown above.
                            if let Ok((_, Some(lbl), _)) = entities.get(entity) {
                                let name = lbl.0.as_str();
                                if ["Sun","Moon","Mercury","Venus","Mars","Jupiter",
                                    "Saturn","Uranus","Neptune","SunLight"]
                                    .iter().any(|p| name.contains(p))
                                {
                                    continue;
                                }
                            }

                            let lbl = entities
                                .get(entity)
                                .ok()
                                .and_then(|(_, l, _)| l.map(|l| l.0.clone()))
                                .unwrap_or_else(|| format!("Entity({entity:?})"));

                            if !filter.is_empty() && !lbl.to_lowercase().contains(&filter) {
                                continue;
                            }

                            let is_focused = focused.0 == Some(entity);
                            ui.horizontal(|ui| {
                                if *depth > 0 {
                                    ui.add_space(*depth as f32 * 16.0);
                                }
                                let row = ui.selectable_label(is_focused, &lbl);
                                if row.clicked() {
                                    focused.0 = if is_focused { None } else { Some(entity) };
                                    changed.send(SelectionChanged);
                                }
                            });
                        }
                    });
            });
        });
}

/// Helper: render a single entity row and handle focus click.
fn entity_row(
    ui:      &mut egui::Ui,
    entity:  Entity,
    name:    &str,
    focused: &mut FocusedEntity,
    changed: &mut EventWriter<SelectionChanged>,
    filter:  &str,
) {
    let display = if name.is_empty() {
        format!("Entity({entity:?})")
    } else {
        format!("{name} [{entity:?}]")
    };
    if !filter.is_empty() && !display.to_lowercase().contains(filter) {
        return;
    }
    let is_focused = focused.0 == Some(entity);
    let row = ui.selectable_label(is_focused, &display);
    if row.clicked() {
        focused.0 = if is_focused { None } else { Some(entity) };
        changed.send(SelectionChanged);
    }
}
