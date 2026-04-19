//! `atlas_editor_voxel_tools` — voxel painting, sculpting, and structure tools.
//!
//! # Overview
//! This crate wires four closely related features into the Atlas Engine editor:
//!
//! 1. **Voxel Brush** — a [`VoxelBrush`] resource that records the active
//!    paint mode, selected voxel type, brush shape, and radius.
//! 2. **Palette Panel** — an egui side panel listing every [`Voxel`] variant
//!    so the user can click to select the active type.
//! 3. **Ray-cast edit** — on LMB press/hold in editor mode the system casts a
//!    DDA ray from the editor camera, finds the hit voxel, and mutates the
//!    [`VoxelData`] on the chunk, then marks it [`ChunkDirty`] so
//!    `remesh_dirty_chunks` rebuilds its mesh automatically.
//! 4. **Undo/redo** — each brush stroke produces a [`BrushStrokeCommand`] that
//!    stores (entity, local-index, old-voxel, new-voxel) tuples so the full
//!    stroke can be reversed in a single undo step.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts};
use atlas_commands::{CommandHistory, EditorCommand, EditorCommandContext};
use atlas_editor_core::{EditorCamera, EditorMode};
use atlas_voxel_planet::{
    biome::Voxel, ChunkDirty, ChunkManager, ManuallyEdited, VoxelChunk, VoxelData,
    CHUNK_SIZE, VOXEL_SIZE,
};

// ────────────────────────────────────────────────────────────────────────────
// Voxel brush resource
// ────────────────────────────────────────────────────────────────────────────

/// Paint mode selected in the voxel tools panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushMode {
    /// Place the selected voxel type (left-click).
    #[default]
    Place,
    /// Remove voxels (replace with Air).
    Remove,
    /// Replace any solid voxel with the selected type (paint over).
    Paint,
}

impl BrushMode {
    fn label(self) -> &'static str {
        match self {
            Self::Place  => "Place",
            Self::Remove => "Remove",
            Self::Paint  => "Paint",
        }
    }
}

/// Brush shape for the sculpt tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushShape {
    /// Cubic (box) brush.
    #[default]
    Box,
    /// Spherical brush.
    Sphere,
}

impl BrushShape {
    fn label(self) -> &'static str {
        match self {
            Self::Box    => "Box",
            Self::Sphere => "Sphere",
        }
    }
}

/// Active voxel editing tool configuration, shared across all systems.
#[derive(Resource)]
pub struct VoxelBrush {
    /// Brush behaviour (place / remove / paint).
    pub mode:           BrushMode,
    /// Voxel type painted by Place / Paint modes.
    pub selected_voxel: Voxel,
    /// Brush shape.
    pub shape:          BrushShape,
    /// Radius in voxels (1 = single voxel).
    pub radius:         u32,
    /// Whether the voxel editing tool is actively enabled.
    pub enabled:        bool,
}

impl Default for VoxelBrush {
    fn default() -> Self {
        Self {
            mode:           BrushMode::Place,
            selected_voxel: Voxel::Grass,
            shape:          BrushShape::Box,
            radius:         1,
            enabled:        false,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Brush stroke undo command
// ────────────────────────────────────────────────────────────────────────────

/// A single voxel edit within a brush stroke.
#[derive(Clone)]
struct VoxelEdit {
    chunk_entity: Entity,
    local_index:  usize,
    old_voxel:    Voxel,
    new_voxel:    Voxel,
}

/// Undo command for a complete brush stroke (may contain many individual
/// voxel edits spanning multiple chunks).
pub struct BrushStrokeCommand {
    edits: Vec<VoxelEdit>,
    label: String,
}

impl EditorCommand for BrushStrokeCommand {
    fn apply(&mut self, ctx: &mut EditorCommandContext) {
        for edit in &self.edits {
            apply_voxel_edit(ctx.world, edit.chunk_entity, edit.local_index, edit.new_voxel);
        }
    }
    fn undo(&mut self, ctx: &mut EditorCommandContext) {
        for edit in &self.edits {
            apply_voxel_edit(ctx.world, edit.chunk_entity, edit.local_index, edit.old_voxel);
        }
    }
    fn label(&self) -> &str { &self.label }
}

/// Directly mutate a voxel in world and mark the chunk dirty.
fn apply_voxel_edit(world: &mut World, entity: Entity, idx: usize, voxel: Voxel) {
    if let Some(mut vd) = world.get_mut::<VoxelData>(entity) {
        if idx < vd.0.len() {
            vd.0[idx] = voxel;
        }
    }
    world.entity_mut(entity).insert(ChunkDirty).insert(ManuallyEdited);
}

// ────────────────────────────────────────────────────────────────────────────
// Active stroke accumulator (tracks current press-drag stroke before commit)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct ActiveStroke {
    edits:    Vec<VoxelEdit>,
    /// True while LMB is held, false once released.
    pressing: bool,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct VoxelToolsPlugin;

impl Plugin for VoxelToolsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_egui::EguiPlugin>() {
            app.add_plugins(bevy_egui::EguiPlugin);
        }
        app.init_resource::<VoxelBrush>()
            .init_resource::<ActiveStroke>()
            .add_systems(
                Update,
                (
                    draw_voxel_palette_panel,
                    voxel_edit_system,
                )
                    .run_if(in_state(EditorMode::Editing)),
            );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Palette panel
// ────────────────────────────────────────────────────────────────────────────

/// All voxel types available in the palette (in display order).
const ALL_VOXELS: &[Voxel] = &[
    Voxel::Grass,
    Voxel::Dirt,
    Voxel::Stone,
    Voxel::Sand,
    Voxel::Sandstone,
    Voxel::Gravel,
    Voxel::Rock,
    Voxel::Snow,
    Voxel::Ice,
    Voxel::Water,
    Voxel::Air,
];

fn voxel_display_name(v: Voxel) -> &'static str {
    match v {
        Voxel::Grass      => "Grass",
        Voxel::Dirt       => "Dirt",
        Voxel::Stone      => "Stone",
        Voxel::Sand       => "Sand",
        Voxel::Sandstone  => "Sandstone",
        Voxel::Gravel     => "Gravel",
        Voxel::Rock       => "Rock",
        Voxel::Snow       => "Snow",
        Voxel::Ice        => "Ice",
        Voxel::Water      => "Water",
        Voxel::Air        => "Air (erase)",
    }
}

/// Approximate sRGB color for the voxel type (matches `biome.rs` color()).
fn voxel_egui_color(v: Voxel) -> egui::Color32 {
    match v {
        Voxel::Grass      => egui::Color32::from_rgb(80,  140, 60),
        Voxel::Dirt       => egui::Color32::from_rgb(120, 80,  50),
        Voxel::Stone      => egui::Color32::from_rgb(130, 130, 130),
        Voxel::Sand       => egui::Color32::from_rgb(210, 195, 140),
        Voxel::Sandstone  => egui::Color32::from_rgb(195, 170, 110),
        Voxel::Gravel     => egui::Color32::from_rgb(110, 110, 105),
        Voxel::Rock       => egui::Color32::from_rgb(90,  90,  85),
        Voxel::Snow       => egui::Color32::from_rgb(240, 240, 250),
        Voxel::Ice        => egui::Color32::from_rgb(160, 210, 230),
        Voxel::Water      => egui::Color32::from_rgb(50,  120, 200),
        Voxel::Air        => egui::Color32::from_rgb(40,  40,  40),
    }
}

fn draw_voxel_palette_panel(
    mut contexts: EguiContexts,
    mut brush:    ResMut<VoxelBrush>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("atlas_voxel_tools")
        .default_width(180.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("🧱 Voxel Tools");
            ui.separator();

            // ── Enable toggle ─────────────────────────────────────────────
            ui.horizontal(|ui| {
                let label = if brush.enabled { "🔴 Active" } else { "⚪ Off" };
                ui.toggle_value(&mut brush.enabled, label);
                if brush.enabled {
                    ui.label(egui::RichText::new("LMB to paint").small().weak());
                }
            });

            ui.separator();

            // ── Mode ─────────────────────────────────────────────────────
            ui.label("Mode");
            ui.horizontal_wrapped(|ui| {
                for mode in [BrushMode::Place, BrushMode::Remove, BrushMode::Paint] {
                    if ui.selectable_label(brush.mode == mode, mode.label()).clicked() {
                        brush.mode = mode;
                    }
                }
            });

            ui.separator();

            // ── Brush ─────────────────────────────────────────────────────
            ui.label("Brush Shape");
            ui.horizontal(|ui| {
                for shape in [BrushShape::Box, BrushShape::Sphere] {
                    if ui.selectable_label(brush.shape == shape, shape.label()).clicked() {
                        brush.shape = shape;
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Radius");
                ui.add(egui::DragValue::new(&mut brush.radius).speed(1.0).range(1..=8));
            });

            ui.separator();

            // ── Palette ───────────────────────────────────────────────────
            ui.label("Voxel Type");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for &voxel in ALL_VOXELS {
                    let is_selected = brush.selected_voxel == voxel;
                    let color_sq = egui::ColorImage::new([16, 16], egui::Color32::BLACK);
                    let _ = color_sq; // colour patch drawn via styled button

                    let response = ui.add_sized(
                        [ui.available_width(), 22.0],
                        egui::SelectableLabel::new(
                            is_selected,
                            egui::RichText::new(format!(
                                "{}  {}",
                                color_swatch_char(voxel),
                                voxel_display_name(voxel)
                            ))
                            .color(if is_selected {
                                egui::Color32::WHITE
                            } else {
                                voxel_egui_color(voxel)
                            }),
                        ),
                    );
                    if response.clicked() {
                        brush.selected_voxel = voxel;
                        // Selecting Air automatically switches to Remove mode;
                        // leaving Air switches back to Place.
                        if voxel == Voxel::Air {
                            brush.mode = BrushMode::Remove;
                        } else {
                            let was_remove = brush.mode == BrushMode::Remove;
                            if was_remove {
                                brush.mode = BrushMode::Place;
                            }
                        }
                    }
                }
            });
        });
}

fn color_swatch_char(v: Voxel) -> &'static str {
    match v {
        Voxel::Water => "■",
        Voxel::Air   => "□",
        _            => "■",
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Voxel edit system — ray-cast + brush + commit to undo history
// ────────────────────────────────────────────────────────────────────────────

fn voxel_edit_system(
    buttons:     Res<ButtonInput<MouseButton>>,
    windows:     Query<&Window, With<PrimaryWindow>>,
    camera_q:    Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    brush:       Res<VoxelBrush>,
    chunk_mgr:   Res<ChunkManager>,
    mut chunk_q: Query<(&VoxelChunk, &mut VoxelData)>,
    mut commands: Commands,
    mut stroke:   ResMut<ActiveStroke>,
    mut history:  ResMut<CommandHistory>,
) {
    if !brush.enabled { return; }

    let pressing = buttons.pressed(MouseButton::Left);
    let just_released = buttons.just_released(MouseButton::Left);

    // ── Commit stroke to history on release ──────────────────────────────────
    if just_released && stroke.pressing && !stroke.edits.is_empty() {
        let edits = std::mem::take(&mut stroke.edits);
        let label = format!(
            "{} ({} voxels)",
            brush.mode.label(),
            edits.len()
        );
        // Push command into history (the voxels were already mutated live;
        // apply() in command is idempotent because it sets to the same value).
        let cmd = BrushStrokeCommand { edits, label };
        // Push the command to the undo stack without re-applying (voxels are
        // already mutated live during the drag).
        history.push_without_apply(Box::new(cmd));
    }
    stroke.pressing = pressing;

    if !pressing { return; }

    // ── Ray-cast from camera ──────────────────────────────────────────────────
    let Ok(window) = windows.get_single() else { return };
    let Ok((camera, cam_tf)) = camera_q.get_single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Some(ray) = camera.viewport_to_world(cam_tf, cursor) else { return };

    let ray_origin = ray.origin;
    let ray_dir    = (*ray.direction).normalize();
    let cs         = (CHUNK_SIZE as f32) * VOXEL_SIZE;

    // DDA-style ray march.
    let step_size  = VOXEL_SIZE * 0.5;
    let max_dist   = 80.0_f32;
    let mut t      = 0.5_f32; // start slightly ahead of origin
    let mut prev_pos = ray_origin;
    let mut hit: Option<(Entity, IVec3, IVec3)> = None; // (entity, chunk_coord, local_coord)

    'march: while t < max_dist {
        let pos = ray_origin + ray_dir * t;

        let chunk_coord = IVec3::new(
            pos.x.div_euclid(cs).floor() as i32,
            pos.y.div_euclid(cs).floor() as i32,
            pos.z.div_euclid(cs).floor() as i32,
        );

        if let Some(&entity) = chunk_mgr.loaded.get(&chunk_coord) {
            if entity != Entity::PLACEHOLDER {
                let chunk_origin = Vec3::new(
                    chunk_coord.x as f32 * cs,
                    chunk_coord.y as f32 * cs,
                    chunk_coord.z as f32 * cs,
                );
                let local_f = pos - chunk_origin;
                let lx = (local_f.x / VOXEL_SIZE).floor() as i32;
                let ly = (local_f.y / VOXEL_SIZE).floor() as i32;
                let lz = (local_f.z / VOXEL_SIZE).floor() as i32;

                if lx >= 0 && ly >= 0 && lz >= 0
                    && (lx as usize) < CHUNK_SIZE
                    && (ly as usize) < CHUNK_SIZE
                    && (lz as usize) < CHUNK_SIZE
                {
                    let idx = voxel_index(lx as usize, ly as usize, lz as usize);
                    // Read-only check via get() on mutable query.
                    if let Ok((_, vd)) = chunk_q.get(entity) {
                        if vd.0[idx].is_solid() {
                            hit = Some((entity, chunk_coord, IVec3::new(lx, ly, lz)));
                            break 'march;
                        }
                    }
                }
            }
        }

        prev_pos = pos;
        t += step_size;
    }

    let Some((hit_entity, hit_chunk_coord, hit_local)) = hit else { return };

    // Collect voxels to edit based on brush shape and radius.
    let edits_this_frame = brush_voxels(
        hit_local,
        hit_chunk_coord,
        hit_entity,
        prev_pos,
        &brush,
        &chunk_mgr,
        &mut chunk_q,
        &mut commands,
    );

    stroke.edits.extend(edits_this_frame);
}

/// Returns the voxel index in a flat chunk array.
/// Delegates to the canonical implementation in `atlas_voxel_planet`.
#[inline]
fn voxel_index(x: usize, y: usize, z: usize) -> usize {
    atlas_voxel_planet::chunk_voxel_index(x, y, z)
}

/// Apply the brush to voxels around the hit location and return the edits made.
fn brush_voxels(
    hit_local:       IVec3,
    hit_chunk_coord: IVec3,
    _hit_entity:     Entity,
    prev_pos:        Vec3,
    brush:           &VoxelBrush,
    chunk_mgr:       &ChunkManager,
    chunk_q:         &mut Query<(&VoxelChunk, &mut VoxelData)>,
    commands:        &mut Commands,
) -> Vec<VoxelEdit> {
    let mut edits = Vec::new();
    let r = brush.radius as i32;
    let cs = CHUNK_SIZE as i32;

    // Determine the center for the brush.
    let center_local = match brush.mode {
        BrushMode::Remove | BrushMode::Paint => hit_local,
        BrushMode::Place => {
            // Place into the air voxel just before the hit surface.
            let cs_f = (CHUNK_SIZE as f32) * VOXEL_SIZE;
            let chunk_origin = Vec3::new(
                hit_chunk_coord.x as f32 * cs_f,
                hit_chunk_coord.y as f32 * cs_f,
                hit_chunk_coord.z as f32 * cs_f,
            );
            let lf = prev_pos - chunk_origin;
            let lx = (lf.x / VOXEL_SIZE).floor() as i32;
            let ly = (lf.y / VOXEL_SIZE).floor() as i32;
            let lz = (lf.z / VOXEL_SIZE).floor() as i32;
            IVec3::new(lx, ly, lz)
        }
    };

    for dz in -r..=r {
        for dy in -r..=r {
            for dx in -r..=r {
                if brush.shape == BrushShape::Sphere {
                    let dist_sq = dx * dx + dy * dy + dz * dz;
                    if dist_sq > r * r { continue; }
                }

                let local = center_local + IVec3::new(dx, dy, dz);

                // Compute final chunk + local coords (handle cross-chunk).
                let global_vox = IVec3::new(
                    hit_chunk_coord.x * cs + local.x,
                    hit_chunk_coord.y * cs + local.y,
                    hit_chunk_coord.z * cs + local.z,
                );

                let target_chunk = IVec3::new(
                    global_vox.x.div_euclid(cs),
                    global_vox.y.div_euclid(cs),
                    global_vox.z.div_euclid(cs),
                );
                let target_local = IVec3::new(
                    global_vox.x.rem_euclid(cs),
                    global_vox.y.rem_euclid(cs),
                    global_vox.z.rem_euclid(cs),
                );

                let Some(&entity) = chunk_mgr.loaded.get(&target_chunk) else { continue };
                if entity == Entity::PLACEHOLDER { continue; }

                let Ok((_, mut vd)) = chunk_q.get_mut(entity) else { continue };
                let idx = voxel_index(
                    target_local.x as usize,
                    target_local.y as usize,
                    target_local.z as usize,
                );
                if idx >= vd.0.len() { continue; }

                let old = vd.0[idx];
                let new = match brush.mode {
                    BrushMode::Remove => Voxel::Air,
                    BrushMode::Place  => {
                        if old == Voxel::Air { brush.selected_voxel } else { continue; }
                    }
                    BrushMode::Paint  => {
                        if old.is_solid() { brush.selected_voxel } else { continue; }
                    }
                };

                if old == new { continue; }

                vd.0[idx] = new;
                commands.entity(entity).insert(ChunkDirty).insert(ManuallyEdited);

                edits.push(VoxelEdit {
                    chunk_entity: entity,
                    local_index:  idx,
                    old_voxel:    old,
                    new_voxel:    new,
                });
            }
        }
    }

    edits
}
