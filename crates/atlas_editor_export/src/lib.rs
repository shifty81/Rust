//! `atlas_editor_export` — Content export pipeline.
//!
//! Translates editor-authored assets into Nova-Forge game-consumable RON files
//! and provides the **Launch Nova-Forge** capability.
//!
//! # Events consumed
//! * [`ExportToGameRequest`] — export all content to `{game_root}/assets/`.
//! * [`LaunchGameRequest`]   — spawn the Nova-Forge game binary (non-blocking).
//!
//! Both events are no-ops (with a warning log) when no game path is linked.
//!
//! # Export targets (current)
//! | Source | Destination |
//! |--------|-------------|
//! | `project/Content/Recipes/*.recipe.ron` | `{game}/assets/voxygen/item/recipes/*.ron` |
//! | [`WorldSettings`] noise params | `{game}/assets/world/world_config.ron` |
//! | `project/Scenes/*.atlasscene` | `{game}/assets/world/sites/*.ron` (structural placement) |

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use atlas_editor_project::GameLinkState;
use atlas_editor_log::OutputLog;
use atlas_voxel_planet::{WorldSettings, NoiseSeed};

// ─────────────────────────────────────────────────────────────────────────────
// Public events
// ─────────────────────────────────────────────────────────────────────────────

/// Trigger a full content export to the linked Nova-Forge game repository.
///
/// Each file written is logged to the [`OutputLog`] panel.  If no game is
/// linked a warning is shown and nothing is written.
#[derive(Event, Debug, Clone, Copy)]
pub struct ExportToGameRequest;

/// Launch the Nova-Forge game binary (or `cargo run`) in the linked repository.
///
/// Non-blocking: the editor stays responsive.  If no game is linked a warning
/// is shown.
#[derive(Event, Debug, Clone, Copy)]
pub struct LaunchGameRequest;

// ─────────────────────────────────────────────────────────────────────────────
// Exportable world config schema
// ─────────────────────────────────────────────────────────────────────────────

/// A minimal world-configuration struct that can be written to
/// `{game}/assets/world/world_config.ron` and read by Nova-Forge's world
/// generator.
///
/// Field names match the expected keys in the game's `WorldOpts`-compatible
/// RON schema.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorldConfigExport {
    pub noise_seed:         u32,
    pub max_terrain_height: f32,
    pub terrain_noise_scale: f32,
    pub moisture_noise_scale: f32,
    pub noise_octaves:      usize,
    pub noise_lacunarity:   f64,
    pub noise_persistence:  f64,
    pub cave_enabled:       bool,
    pub cave_scale:         f64,
    pub cave_threshold:     f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

pub struct EditorExportPlugin;

impl Plugin for EditorExportPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<ExportToGameRequest>()
            .add_event::<LaunchGameRequest>()
            .add_systems(Update, (handle_export, handle_launch).chain());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Systems
// ─────────────────────────────────────────────────────────────────────────────

fn handle_export(
    mut events:   EventReader<ExportToGameRequest>,
    game_link:    Res<GameLinkState>,
    settings:     Res<WorldSettings>,
    seed:         Res<NoiseSeed>,
    mut log:      ResMut<OutputLog>,
) {
    for _ev in events.read() {
        let Some(ref game_path) = game_link.game_path else {
            log.warn("[Export] Cannot export — no Nova-Forge game repo is linked.");
            continue;
        };

        log.info(format!("[Export] Exporting to: {}", game_path.display()));

        // ── 1. World Config ────────────────────────────────────────────────
        if let Err(e) = export_world_config(game_path, &settings, seed.0, &mut log) {
            log.error(format!("[Export] World config failed: {e}"));
        }

        // ── 2. Recipes ─────────────────────────────────────────────────────
        let editor_recipes = PathBuf::from("project/Content/Recipes");
        if editor_recipes.exists() {
            if let Err(e) = export_recipes(game_path, &editor_recipes, &mut log) {
                log.error(format!("[Export] Recipes failed: {e}"));
            }
        } else {
            log.info("[Export] No editor recipes found (project/Content/Recipes does not exist).");
        }

        // ── 3. Scenes ──────────────────────────────────────────────────────
        let editor_scenes = PathBuf::from("project/Scenes");
        if editor_scenes.exists() {
            if let Err(e) = export_scenes(game_path, &editor_scenes, &mut log) {
                log.error(format!("[Export] Scenes failed: {e}"));
            }
        }

        log.info("[Export] Done.");
    }
}

fn handle_launch(
    mut events: EventReader<LaunchGameRequest>,
    game_link:  Res<GameLinkState>,
    mut log:    ResMut<OutputLog>,
) {
    for _ev in events.read() {
        let Some(ref game_path) = game_link.game_path else {
            log.warn("[Launch] Cannot launch — no Nova-Forge game repo is linked.");
            continue;
        };

        // Resolution order: shell script → release binary → cargo run.
        let candidates: &[&str] = if cfg!(windows) {
            &["target/release/nova-forge-voxygen.exe"]
        } else {
            &["nova-forge.sh", "target/release/nova-forge-voxygen"]
        };

        let binary = candidates
            .iter()
            .find_map(|rel| {
                let p = game_path.join(rel);
                if p.exists() { Some(p) } else { None }
            });

        if let Some(bin) = binary {
            log.info(format!("[Launch] Spawning: {}", bin.display()));
            match std::process::Command::new(&bin)
                .current_dir(game_path)
                .spawn()
            {
                Ok(_)  => log.info("[Launch] Game process started."),
                Err(e) => log.error(format!("[Launch] Failed to spawn process: {e}")),
            }
        } else {
            log.info("[Launch] No prebuilt binary found; falling back to `cargo run -p nova-forge-voxygen`.");
            match std::process::Command::new("cargo")
                .args(["run", "-p", "nova-forge-voxygen"])
                .current_dir(game_path)
                .spawn()
            {
                Ok(_)  => log.info("[Launch] cargo run started."),
                Err(e) => log.error(format!("[Launch] cargo run failed: {e}")),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Export helpers
// ─────────────────────────────────────────────────────────────────────────────

fn export_world_config(
    game_root: &Path,
    settings:  &WorldSettings,
    seed:      u32,
    log:       &mut OutputLog,
) -> Result<(), String> {
    let dest = game_root.join("assets/world/world_config.ron");
    ensure_dir(&dest)?;

    let cfg = WorldConfigExport {
        noise_seed:          seed,
        max_terrain_height:  settings.max_terrain_height,
        terrain_noise_scale: settings.terrain_noise_scale as f32,
        moisture_noise_scale: settings.moisture_noise_scale as f32,
        noise_octaves:       settings.noise_octaves,
        noise_lacunarity:    settings.noise_lacunarity,
        noise_persistence:   settings.noise_persistence,
        cave_enabled:        settings.cave_enabled,
        cave_scale:          settings.cave_scale,
        cave_threshold:      settings.cave_threshold,
    };

    let text = ron::ser::to_string_pretty(&cfg, PrettyConfig::default())
        .map_err(|e| format!("serialise: {e}"))?;
    std::fs::write(&dest, text)
        .map_err(|e| format!("write: {e}"))?;

    log.info(format!("[Export] wrote {}", dest.display()));
    Ok(())
}

fn export_recipes(
    game_root:    &Path,
    recipes_dir:  &Path,
    log:          &mut OutputLog,
) -> Result<(), String> {
    let dest_dir = game_root.join("assets/voxygen/item/recipes");
    ensure_dir(&dest_dir.join("placeholder"))?;

    let rd = std::fs::read_dir(recipes_dir)
        .map_err(|e| format!("read_dir: {e}"))?;

    for entry in rd.flatten() {
        let src = entry.path();
        if src.extension().and_then(|e| e.to_str()) == Some("ron") {
            let file_name = src.file_name().unwrap_or_default();
            let dest = dest_dir.join(file_name);
            std::fs::copy(&src, &dest)
                .map_err(|e| format!("copy {}: {e}", src.display()))?;
            log.info(format!("[Export] recipe → {}", dest.display()));
        }
    }
    Ok(())
}

fn export_scenes(
    game_root:  &Path,
    scenes_dir: &Path,
    log:        &mut OutputLog,
) -> Result<(), String> {
    let dest_dir = game_root.join("assets/world/sites");
    ensure_dir(&dest_dir.join("placeholder"))?;

    let rd = std::fs::read_dir(scenes_dir)
        .map_err(|e| format!("read_dir: {e}"))?;

    for entry in rd.flatten() {
        let src = entry.path();
        if src.extension().and_then(|e| e.to_str()) == Some("atlasscene") {
            // Rename .atlasscene → .ron for the game.
            let stem = src.file_stem().unwrap_or_default();
            let dest = dest_dir.join(stem).with_extension("ron");
            // Direct copy for now — a real mapping would translate the schema.
            std::fs::copy(&src, &dest)
                .map_err(|e| format!("copy {}: {e}", src.display()))?;
            log.info(format!("[Export] scene → {}", dest.display()));
        }
    }
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_all: {e}"))?;
    }
    Ok(())
}
