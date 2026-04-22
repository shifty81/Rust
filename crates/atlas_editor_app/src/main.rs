//! `atlas_editor_app` — Atlas Engine editor executable.
//!
//! This is the **single entry point** for the voxel engine.  The editor loads
//! the full voxel world (solar system, planet terrain, atmosphere, vegetation)
//! and provides a planet-aware free-fly camera for world inspection and editing.
//!
//! When the editor is linked to a Nova-Forge game repository (set via
//! `project/Config/project.ron` or via Edit → Nova-Forge), the default voxel
//! sandbox is suppressed and the viewport waits for game content instead.
//!
//! To explore a generated world, press **Play (▶)** to enter Play-In-Editor
//! (PIE) mode, which spawns the first-person player controller directly inside
//! the editor.  Press **Stop (■)** to return to the editor camera.

use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::log::LogPlugin;
use bevy::prelude::*;

// ── Voxel planet engine ──────────────────────────────────────────────────────
use atlas_voxel_planet::{GameplayPlugins, WorldPlugins, VoxelWorldEnabled};
use atlas_voxel_planet::{Planet, Ocean, Sun, Moon, SunLight, Star, OrbitalBody, VoxelChunk};

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
use atlas_editor_project::{EditorProjectPlugin, GameLinkState, ProjectSettings};
use atlas_editor_scene::EditorScenePlugin;
use atlas_editor_ui::EditorUiPlugin;
use atlas_editor_viewport::EditorViewportPlugin;
use atlas_editor_world_settings::EditorWorldSettingsPlugin;
use atlas_editor_voxel_tools::VoxelToolsPlugin;
use atlas_editor_export::EditorExportPlugin;

// ─────────────────────────────────────────────────────────────────────────────
// Startup helper — check whether project.ron already has a Nova-Forge path
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `true` when the persisted `project/Config/project.ron` in the
/// current working directory has a `nova_forge_game_path` set.
fn project_ron_has_nova_forge_link() -> bool {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let path = cwd.join("project/Config/project.ron");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| ron::from_str::<ProjectSettings>(&text).ok())
        .map(|s| s.nova_forge_game_path.is_some())
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime link-state watcher
// ─────────────────────────────────────────────────────────────────────────────

/// Watches [`GameLinkState`] for changes.
///
/// * When a game repo is **linked** mid-session: disables the voxel world and
///   despawns all existing solar-system / terrain entities so the viewport
///   shows an empty scene ready for Nova-Forge content.
/// * When a game repo is **unlinked** mid-session: re-enables the voxel world.
///   A full world restart is not attempted at runtime; the user should restart
///   the editor to restore the sandbox planet.
fn sync_voxel_world_on_link_change(
    game_link:   Res<GameLinkState>,
    mut enabled: ResMut<VoxelWorldEnabled>,
    mut commands: Commands,
    // All voxel / solar-system entity types that need to be cleaned up.
    solar_q: Query<Entity, Or<(
        With<Sun>, With<Moon>, With<SunLight>,
        With<OrbitalBody>, With<Star>,
    )>>,
    planet_q: Query<Entity, Or<(With<Planet>, With<Ocean>, With<VoxelChunk>)>>,
) {
    if !game_link.is_changed() { return; }

    if game_link.is_linked() {
        if enabled.0 {
            info!("[Nova-Forge] Game repo linked — hiding voxel sandbox.");
            enabled.0 = false;
            for entity in solar_q.iter().chain(planet_q.iter()) {
                commands.entity(entity).despawn_recursive();
            }
        }
    } else if !enabled.0 {
        // Re-enable for when the user unlinks; the planet won't re-spawn at
        // runtime — the editor must be restarted to restore the sandbox.
        info!("[Nova-Forge] Game repo unlinked — voxel world will be active on next launch.");
        enabled.0 = true;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    // ── Pre-flight: detect existing Nova-Forge link from project.ron ─────────
    // This must happen *before* App::new() so we can insert VoxelWorldEnabled
    // prior to WorldPlugins calling init_resource (which only sets it when
    // absent).  Reading a small RON file synchronously here is acceptable.
    let linked_at_startup = project_ron_has_nova_forge_link();

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

    let mut app = App::new();

    // When linked, suppress the voxel sandbox *before* WorldPlugins registers
    // its init_resource call so that the Startup spawn systems are skipped.
    if linked_at_startup {
        app.insert_resource(VoxelWorldEnabled(false));
    }

    app
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
        // `WorldPlugins` = terrain / atmosphere / flora / fauna / world I/O.
        // Its Startup systems are gated on VoxelWorldEnabled, so inserting
        // VoxelWorldEnabled(false) above is sufficient to suppress the sandbox.
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
            EditorExportPlugin,
        ))

        // ── Runtime Nova-Forge link/unlink watcher ───────────────────────────
        .add_systems(Update, sync_voxel_world_on_link_change)

        .run();
}
