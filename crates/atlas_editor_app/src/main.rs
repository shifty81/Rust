//! `atlas_editor_app` — Atlas Engine editor executable.
//!
//! Centred around the Rust Voxel Planet Engine.  The editor loads the full
//! voxel world (solar system, planet terrain, atmosphere, vegetation) at
//! startup and provides a planet-aware free-fly camera.  The player controller
//! is only active during Play-In-Editor sessions.

use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::log::LogPlugin;
use bevy::prelude::*;

// ── Voxel planet engine ──────────────────────────────────────────────────────
use atlas_voxel_planet::VoxelPlanetPlugins;

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

fn main() {
    App::new()
        // ── Host window ──────────────────────────────────────────────────────
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Atlas Editor — Voxel Planet".into(),
                    ..default()
                }),
                ..default()
            })
            .set(LogPlugin {
                custom_layer: atlas_editor_log::build_editor_log_layer,
                ..default()
            })
        )

        // ── Diagnostics ──────────────────────────────────────────────────────
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin,
        ))

        // ── Voxel planet engine (world without player) ───────────────────────
        .add_plugins(VoxelPlanetPlugins)

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
        ))

        .run();
}
