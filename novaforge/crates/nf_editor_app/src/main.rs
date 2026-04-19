//! `nf_editor_app` — NovaForge editor executable.
//!
//! Composes all editor and runtime plugins into a single Bevy app.

use bevy::prelude::*;

// Runtime crates
use nf_assets::AssetsPlugin;
use nf_scene::ScenePlugin;
use nf_render::RenderPlugin;
use nf_game::GamePlugin;
use nf_prefab::PrefabPlugin;
use nf_selection::SelectionPlugin;
use nf_gizmos::GizmosPlugin;
use nf_commands::CommandHistoryPlugin;

// Editor crates
use nf_editor_core::EditorCorePlugin;
use nf_editor_ui::EditorUiPlugin;
use nf_editor_viewport::EditorViewportPlugin;
use nf_editor_outliner::EditorOutlinerPlugin;
use nf_editor_details::EditorDetailsPlugin;
use nf_editor_content::EditorContentPlugin;
use nf_editor_scene::EditorScenePlugin;
use nf_editor_play::EditorPlayPlugin;
use nf_editor_log::EditorLogPlugin;
use nf_editor_project::EditorProjectPlugin;

fn main() {
    App::new()
        // ── host window ──────────────────────────────────────────────────────
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "NovaForge Editor".into(),
                ..default()
            }),
            ..default()
        }))

        // ── shared / runtime plugins ─────────────────────────────────────────
        .add_plugins((
            AssetsPlugin,
            ScenePlugin,
            RenderPlugin,
            GamePlugin,
            PrefabPlugin,
            SelectionPlugin,
            GizmosPlugin,
            CommandHistoryPlugin,
        ))

        // ── editor plugins ───────────────────────────────────────────────────
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
        ))

        .run();
}
