//! `nf_editor_app` — NovaForge editor executable.
//!
//! Centred around the Rust Voxel Planet Engine.  The editor loads the full
//! voxel world (solar system, planet terrain, atmosphere, vegetation) at
//! startup and provides a planet-aware free-fly camera.  The player controller
//! is only active during Play-In-Editor sessions.

use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

// ── Voxel planet engine ──────────────────────────────────────────────────────
use nf_voxel_planet::VoxelPlanetPlugins;

// ── Shared infrastructure plugins ───────────────────────────────────────────
use nf_assets::AssetsPlugin;
use nf_commands::CommandHistoryPlugin;
use nf_gizmos::GizmosPlugin;
use nf_prefab::PrefabPlugin;
use nf_render::RenderPlugin;
use nf_scene::ScenePlugin;
use nf_selection::SelectionPlugin;

// ── Editor plugins ───────────────────────────────────────────────────────────
use nf_editor_content::EditorContentPlugin;
use nf_editor_core::EditorCorePlugin;
use nf_editor_details::EditorDetailsPlugin;
use nf_editor_log::EditorLogPlugin;
use nf_editor_outliner::EditorOutlinerPlugin;
use nf_editor_play::EditorPlayPlugin;
use nf_editor_project::EditorProjectPlugin;
use nf_editor_scene::EditorScenePlugin;
use nf_editor_ui::EditorUiPlugin;
use nf_editor_viewport::EditorViewportPlugin;
use nf_editor_world_settings::EditorWorldSettingsPlugin;

fn main() {
    App::new()
        // ── Host window ──────────────────────────────────────────────────────
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "NovaForge Editor — Voxel Planet".into(),
                ..default()
            }),
            ..default()
        }))

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
        ))

        .run();
}
