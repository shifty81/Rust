//! `atlas_editor_content` — Content Browser panel.
//!
//! The left folder tree has two roots:
//! * **Editor Content** — `project/Content/` folders (always shown).
//! * **Game Assets** — `{nova_forge_root}/assets/` subtree (only when linked).
//!
//! Clicking a game-asset folder scans it for `.ron` files and shows them in
//! the asset grid with their kind icon.  Game assets are read-only; the tooltip
//! explains that edits should be made through Export.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use atlas_assets::{AssetRegistry, AssetKind};
use atlas_editor_core::{EditorMode, EditorPanelOrder, PanelVisibility};
use atlas_editor_project::GameLinkState;

// ────────────────────────────────────────────────────────────────────────────
// Local state
// ────────────────────────────────────────────────────────────────────────────

/// Which folder is currently selected in the left tree.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SelectedFolder {
    /// One of the built-in editor content folders (e.g. "Scenes").
    EditorFolder(String),
    /// A folder under the linked game's `assets/` directory.
    GameFolder(PathBuf),
}

#[derive(Resource, Default)]
struct ContentBrowserState {
    selected_folder: Option<SelectedFolder>,
    /// Read-only entries loaded from the currently selected game folder.
    game_folder_entries: Vec<(String, AssetKind)>,
    /// Detail window: path of the file currently shown (read-only).
    detail_file: Option<PathBuf>,
    /// Cached content of the detail file (loaded lazily).
    detail_text: Option<String>,
    /// Cached top-level directories under `{game}/assets/` — refreshed only
    /// when the linked game path changes.
    cached_game_dirs: Vec<PathBuf>,
    /// The game path that `cached_game_dirs` was built from.
    game_dirs_from_path: Option<PathBuf>,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorContentPlugin;

impl Plugin for EditorContentPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ContentBrowserState>()
            .add_systems(Update, draw_content_panel.in_set(EditorPanelOrder::BottomContent));
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Panel
// ────────────────────────────────────────────────────────────────────────────

fn draw_content_panel(
    mut contexts: EguiContexts,
    registry:     Res<AssetRegistry>,
    mode:         Res<State<EditorMode>>,
    visibility:   Res<PanelVisibility>,
    game_link:    Res<GameLinkState>,
    mut state:    ResMut<ContentBrowserState>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }
    if !visibility.content_browser {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::bottom("atlas_content_browser")
        .default_height(200.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Content Browser");
            ui.separator();

            ui.horizontal(|ui| {
                // ── Left: Folder tree ────────────────────────────────────
                egui::SidePanel::left("atlas_content_folders")
                    .default_width(180.0)
                    .resizable(true)
                    .show_inside(ui, |ui| {
                        // ── Editor Content root ──────────────────────────
                        ui.label(
                            egui::RichText::new("📁 Editor Content")
                                .strong()
                                .color(egui::Color32::from_rgb(200, 200, 255)),
                        );
                        ui.indent("editor_folders", |ui| {
                            for folder in ["Scenes", "Prefabs", "Meshes", "Materials", "Audio"] {
                                let is_selected = state.selected_folder
                                    == Some(SelectedFolder::EditorFolder(folder.to_owned()));
                                if ui.selectable_label(is_selected, folder).clicked() {
                                    state.selected_folder =
                                        Some(SelectedFolder::EditorFolder(folder.to_owned()));
                                    state.game_folder_entries.clear();
                                    state.detail_file = None;
                                }
                            }
                        });

                        // ── Game Assets root (only when linked) ──────────
                        if let Some(assets_path) = game_link.assets_path() {
                            // Refresh the cached directory list only when the
                            // linked path changes (not every frame).
                            if state.game_dirs_from_path.as_deref() != Some(assets_path.as_path()) {
                                state.cached_game_dirs = if let Ok(rd) = std::fs::read_dir(&assets_path) {
                                    let mut dirs: Vec<PathBuf> = rd
                                        .flatten()
                                        .filter(|e| e.path().is_dir())
                                        .map(|e| e.path())
                                        .collect();
                                    dirs.sort();
                                    dirs
                                } else {
                                    Vec::new()
                                };
                                state.game_dirs_from_path = Some(assets_path);
                            }

                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("🎮 Game Assets")
                                    .strong()
                                    .color(egui::Color32::from_rgb(100, 220, 100)),
                            );
                            ui.indent("game_asset_folders", |ui| {
                                for dir in state.cached_game_dirs.clone() {
                                    let label = dir
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("?");
                                    let is_selected = state.selected_folder
                                        == Some(SelectedFolder::GameFolder(dir.clone()));
                                    if ui.selectable_label(is_selected, label).clicked() {
                                        state.selected_folder =
                                            Some(SelectedFolder::GameFolder(dir.clone()));
                                        state.detail_file = None;
                                        state.game_folder_entries = scan_game_folder(&dir);
                                    }
                                }
                            });
                        } else {
                            // Game unlinked — reset cached dirs.
                            if state.game_dirs_from_path.is_some() {
                                state.cached_game_dirs.clear();
                                state.game_dirs_from_path = None;
                            }
                        }
                    });

                // ── Right: Asset grid ────────────────────────────────────
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match &state.selected_folder {
                        None | Some(SelectedFolder::EditorFolder(_)) => {
                            // Editor asset grid from AssetRegistry.
                            let assets = registry.all();
                            if assets.is_empty() {
                                ui.label("(no assets registered)");
                            } else {
                                egui::Grid::new("atlas_asset_grid").show(ui, |ui| {
                                    for (i, record) in assets.iter().enumerate() {
                                        let icon = kind_icon(record.kind.clone());
                                        ui.label(format!("{icon} {}", record.path));
                                        if (i + 1) % 4 == 0 { ui.end_row(); }
                                    }
                                });
                            }
                        }
                        Some(SelectedFolder::GameFolder(_)) => {
                            // Game asset grid (read-only).
                            let entries = state.game_folder_entries.clone();
                            if entries.is_empty() {
                                ui.label(
                                    egui::RichText::new("(no .ron files in this folder)")
                                        .color(egui::Color32::GRAY),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new(
                                        "Read-only — edit via Nova-Forge → Export to Game",
                                    )
                                    .small()
                                    .color(egui::Color32::from_rgb(180, 180, 80)),
                                );
                                ui.separator();
                                egui::Grid::new("atlas_game_asset_grid").show(ui, |ui| {
                                    for (i, (name, kind)) in entries.iter().enumerate() {
                                        let icon = kind_icon(kind.clone());
                                        let resp = ui.selectable_label(
                                            false,
                                            format!("{icon} {name}"),
                                        );
                                        if resp.clicked() {
                                            // Determine full path for detail view.
                                            if let Some(SelectedFolder::GameFolder(dir)) =
                                                &state.selected_folder
                                            {
                                                let full = dir.join(name);
                                                state.detail_file = Some(full.clone());
                                                state.detail_text = std::fs::read_to_string(&full).ok();
                                            }
                                        }
                                        resp.on_hover_text(
                                            "Click to preview. Edit via Export pipeline.",
                                        );
                                        if (i + 1) % 4 == 0 { ui.end_row(); }
                                    }
                                });
                            }
                        }
                    }
                });
            });
        });

    // ── Read-only detail window ──────────────────────────────────────────────
    if let (Some(detail_path), Some(detail_text)) =
        (state.detail_file.clone(), state.detail_text.clone())
    {
        let file_name = detail_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_owned();

        let mut open = true;
        egui::Window::new(format!("📄 {file_name}  [read-only]"))
            .id(egui::Id::new("atlas_game_asset_detail"))
            .open(&mut open)
            .default_width(480.0)
            .resizable(true)
            .scroll([false, true])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(detail_path.display().to_string())
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut detail_text.as_str())
                                .code_editor()
                                .desired_width(f32::INFINITY),
                        );
                    });
            });

        if !open {
            state.detail_file = None;
            state.detail_text = None;
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

/// Map [`AssetKind`] to an emoji icon.
fn kind_icon(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Scene    => "🗺",
        AssetKind::Prefab   => "📦",
        AssetKind::Mesh     => "🧊",
        AssetKind::Texture  => "🖼",
        AssetKind::Material => "🎨",
        AssetKind::Audio    => "🔊",
        _                   => "📄",
    }
}

/// Guess the [`AssetKind`] from a file name extension.
fn guess_kind(name: &str) -> AssetKind {
    if name.ends_with(".atlasscene")   { return AssetKind::Scene; }
    if name.ends_with(".atlasprefab")  { return AssetKind::Prefab; }
    if name.ends_with(".glb") || name.ends_with(".gltf") { return AssetKind::Mesh; }
    if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".ktx2") {
        return AssetKind::Texture;
    }
    if name.ends_with(".ogg") || name.ends_with(".mp3") || name.ends_with(".wav") {
        return AssetKind::Audio;
    }
    AssetKind::Unknown
}

/// Scan a directory for files (non-recursive) and build `(name, kind)` pairs.
fn scan_game_folder(dir: &std::path::Path) -> Vec<(String, AssetKind)> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut entries: Vec<(String, AssetKind)> = rd
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            Some((name.clone(), guess_kind(&name)))
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}
