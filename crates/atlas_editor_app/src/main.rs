//! `atlas_editor_app` — Atlas Engine editor executable.
//!
//! This is the **single entry point** for the voxel engine.  The editor loads
//! the full voxel world (solar system, planet terrain, atmosphere, vegetation)
//! and provides a planet-aware free-fly camera for world inspection and editing.
//!
//! To explore a generated world, press **Play (▶)** to enter Play-In-Editor
//! (PIE) mode, which spawns the first-person player controller directly inside
//! the editor.  Press **Stop (■)** to return to the editor camera.

use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::log::LogPlugin;
use bevy::prelude::*;

// ── Voxel planet engine ──────────────────────────────────────────────────────
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
                // Quiet the wgpu/Vulkan layers that emit validation spam every
                // frame on some drivers — see `atlas_editor_log::EditorLogLayer`
                // for the in-app VUID suppression.
                filter: "info,wgpu_core=warn,wgpu_hal=warn,naga=warn".to_string(),
                ..default()
            })
            // Hot-reload authored RON content (recipes, biomes, voxels, …)
            // while the editor is running.  Bevy's file-watcher feature is
            // enabled at the workspace level; this flag turns it on at
            // runtime so `AssetEvent::Modified` fires on edits under
            // `assets/Content/`.
            .set(AssetPlugin {
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
        // `WorldPlugins` = terrain / atmosphere / flora / fauna / world I/O —
        // safe to have always on in the editor.
        //
        // `GameplayPlugins` = HUD, minimap, hotbar, crafting panel, dialogue,
        // character rig, ambient audio, multiplayer.  Registered here so PIE
        // can use them; `atlas_editor_play::toggle_gameplay_ui_visibility`
        // hides their root UI nodes while in Editing mode so they don't fight
        // the editor panels for screen space.
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
        ))

        .run();
}
