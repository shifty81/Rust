//! `atlas_editor_app` — Nova-Forge Editor executable.
//!
//! This is the **single entry point** for the Nova-Forge editor.  The editor
//! starts with an empty viewport ready for Nova-Forge game content.  The voxel
//! sandbox (solar system, procedural planet) is permanently disabled — it is
//! not part of the Nova-Forge workflow.
//!
//! Link a Nova-Forge game repository via **Edit → Nova-Forge** to enable
//! game-asset browsing, content export, and game launching.

use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::log::LogPlugin;
use bevy::prelude::*;

// ── Voxel planet engine (resources / systems still registered; sandbox suppressed) ──
use atlas_voxel_planet::{GameplayPlugins, WorldPlugins};

// ── Shared infrastructure plugins ───────────────────────────────────────────
use atlas_assets::AssetsPlugin;
use atlas_commands::CommandHistoryPlugin;
use atlas_gizmos::GizmosPlugin;
use atlas_prefab::PrefabPlugin;
use atlas_render::RenderPlugin;
use atlas_scene::ScenePlugin;
use atlas_selection::SelectionPlugin;

// ── Editor plugins ───────────────────────────────────────────────────────────
use atlas_editor_content::EditorContentPlugin;
use atlas_editor_core::EditorCorePlugin;
use atlas_editor_details::EditorDetailsPlugin;
use atlas_editor_log::EditorLogPlugin;
use atlas_editor_outliner::EditorOutlinerPlugin;
use atlas_editor_play::EditorPlayPlugin;
use atlas_editor_project::EditorProjectPlugin;
use atlas_editor_scene::EditorScenePlugin;
use atlas_editor_ui::EditorUiPlugin;
use atlas_editor_viewport::EditorViewportPlugin;
use atlas_editor_world_settings::EditorWorldSettingsPlugin;
use atlas_editor_voxel_tools::VoxelToolsPlugin;
use atlas_editor_export::EditorExportPlugin;

fn main() {
    // Bevy's file-watcher (enabled below for hot-reload) requires an absolute
    // path on Windows — the `notify` crate fails with a relative path even
    // when the directory exists.  Resolve `assets/` to an absolute path and
    // ensure the directory exists before building the App.
    let assets_dir = std::env::current_dir()
        .expect("could not determine working directory")
        .join("assets");
    let _ = std::fs::create_dir_all(&assets_dir);
    let assets_path = assets_dir
        .to_str()
        .expect("assets path contains invalid UTF-8")
        .to_owned();

    App::new()
        // ── Host window ──────────────────────────────────────────────────────
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Nova-Forge Editor".into(),
                    ..default()
                }),
                ..default()
            })
            .set(LogPlugin {
                custom_layer: atlas_editor_log::build_editor_log_layer,
                // Quiet the wgpu/Vulkan layers that emit validation spam every
                // frame on some drivers.
                filter: "info,wgpu_core=warn,wgpu_hal=warn,naga=warn".to_string(),
                ..default()
            })
            // Hot-reload authored RON content while the editor is running.
            .set(AssetPlugin {
                file_path: assets_path,
                watch_for_changes_override: Some(true),
                ..default()
            })
        )

        // ── Diagnostics ──────────────────────────────────────────────────────
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin,
        ))

        // ── Voxel planet engine ──────────────────────────────────────────────
        // WorldPlugins / GameplayPlugins register resources and systems used
        // by editor panels (ChunkManager, WorldSettings, PIE, etc.).
        // VoxelWorldEnabled defaults to false so no sandbox entities are ever
        // spawned — the viewport stays empty, ready for Nova-Forge content.
        .add_plugins(WorldPlugins)
        .add_plugins(GameplayPlugins)

        // ── Shared infrastructure ────────────────────────────────────────────
        .add_plugins((
            AssetsPlugin,
            ScenePlugin,
            RenderPlugin,
            PrefabPlugin,
            SelectionPlugin,
            GizmosPlugin,
            CommandHistoryPlugin,
        ))

        // ── Editor plugins ───────────────────────────────────────────────────
        .add_plugins((
            EditorCorePlugin,
            EditorUiPlugin,
            EditorViewportPlugin,
            EditorOutlinerPlugin,
            EditorDetailsPlugin,
            EditorContentPlugin,
            EditorScenePlugin,
            EditorPlayPlugin,
            EditorLogPlugin,
            EditorProjectPlugin,
            EditorWorldSettingsPlugin,
            VoxelToolsPlugin,
            EditorExportPlugin,
        ))

        .run();
}

