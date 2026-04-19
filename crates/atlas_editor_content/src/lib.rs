//! `atlas_editor_content` — Content Browser panel.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use atlas_assets::{AssetRegistry, AssetKind};
use atlas_editor_core::EditorMode;

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorContentPlugin;

impl Plugin for EditorContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_content_panel);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Panel
// ────────────────────────────────────────────────────────────────────────────

fn draw_content_panel(
    mut contexts: EguiContexts,
    registry:     Res<AssetRegistry>,
    mode:         Res<State<EditorMode>>,
) {
    if *mode.get() != EditorMode::Editing {
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
                // Folder tree (left)
                egui::SidePanel::left("atlas_content_folders")
                    .default_width(160.0)
                    .resizable(true)
                    .show_inside(ui, |ui| {
                        ui.label("Content/");
                        ui.indent("folders", |ui| {
                            for folder in ["Scenes", "Prefabs", "Meshes", "Materials", "Audio"] {
                                let _ = ui.selectable_label(false, folder);
                            }
                        });
                    });

                // Asset grid (right)
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let assets = registry.all();
                    if assets.is_empty() {
                        ui.label("(no assets registered)");
                    } else {
                        egui::Grid::new("atlas_asset_grid").show(ui, |ui| {
                            for (i, record) in assets.iter().enumerate() {
                                let icon = match record.kind {
                                    AssetKind::Scene    => "🗺",
                                    AssetKind::Prefab   => "📦",
                                    AssetKind::Mesh     => "🧊",
                                    AssetKind::Texture  => "🖼",
                                    AssetKind::Material => "🎨",
                                    AssetKind::Audio    => "🔊",
                                    _                   => "📄",
                                };
                                ui.label(format!("{icon} {}", record.path));
                                if (i + 1) % 4 == 0 { ui.end_row(); }
                            }
                        });
                    }
                });
            });
        });
}
