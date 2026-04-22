//! `atlas_editor_project` — project loading, settings persistence, content root paths.
//!
//! On startup the plugin tries to load `project/Config/project.ron` from the
//! working directory.  Sending [`OpenProjectRequest`] switches the root and
//! re-reads the settings file from the new location.  [`SaveProjectRequest`]
//! writes the current settings back to disk.
//!
//! # Nova-Forge game linking
//! [`ProjectSettings::nova_forge_game_path`] stores the absolute path to the
//! cloned Nova-Forge game repository.  This path is persisted so it survives
//! editor restarts.  The [`GameLinkState`] resource exposes the derived status
//! so other panels can react without touching the project resource directly.

use bevy::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ────────────────────────────────────────────────────────────────────────────
// Project settings (persisted to `project/Config/project.ron`)
// ────────────────────────────────────────────────────────────────────────────

pub const PROJECT_SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub version:      u32,
    pub project_name: String,
    pub content_root: PathBuf,
    pub scenes_root:  PathBuf,
    pub prefabs_root: PathBuf,
    pub cache_root:   PathBuf,
    pub default_scene: Option<PathBuf>,
    /// Absolute path to the cloned Nova-Forge game repository root.
    /// When `Some`, the editor exposes game-asset browsing and export.
    #[serde(default)]
    pub nova_forge_game_path: Option<PathBuf>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            version:       PROJECT_SETTINGS_VERSION,
            project_name:  "Nova-Forge Project".into(),
            content_root:  PathBuf::from("project/Content"),
            scenes_root:   PathBuf::from("project/Scenes"),
            prefabs_root:  PathBuf::from("project/Prefabs"),
            cache_root:    PathBuf::from("project/Cache"),
            default_scene: None,
            nova_forge_game_path: None,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Active project resource
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct ActiveProject {
    pub settings:  ProjectSettings,
    /// Absolute path to the project root directory.
    pub root_path: Option<PathBuf>,
}

impl ActiveProject {
    /// Absolute path to the project's content directory.
    pub fn content_path(&self) -> Option<PathBuf> {
        self.root_path.as_ref().map(|r| r.join(&self.settings.content_root))
    }

    /// Absolute path to the project's scenes directory.
    pub fn scenes_path(&self) -> Option<PathBuf> {
        self.root_path.as_ref().map(|r| r.join(&self.settings.scenes_root))
    }

    /// The path of the `project.ron` settings file inside `root_path`.
    pub fn settings_file(&self) -> Option<PathBuf> {
        self.root_path
            .as_ref()
            .map(|r| r.join("project/Config/project.ron"))
    }

    /// Absolute path to the linked Nova-Forge game's `assets/` directory,
    /// or `None` when no game path is configured.
    pub fn game_assets_path(&self) -> Option<PathBuf> {
        self.settings.nova_forge_game_path
            .as_ref()
            .map(|p| p.join("assets"))
    }

    /// Absolute path to the Nova-Forge game binary.
    ///
    /// Resolution order:
    /// 1. `{game_root}/nova-forge.sh`  (Linux / macOS shell wrapper)
    /// 2. `{game_root}/target/release/nova-forge-voxygen` (Linux / macOS)
    /// 3. `{game_root}/target/release/nova-forge-voxygen.exe` (Windows)
    ///
    /// Returns `None` when no game path is configured, or when none of the
    /// candidate paths exist on disk.
    pub fn game_binary_path(&self) -> Option<PathBuf> {
        let root = self.settings.nova_forge_game_path.as_ref()?;
        let candidates = [
            root.join("nova-forge.sh"),
            root.join("target/release/nova-forge-voxygen"),
            root.join("target/release/nova-forge-voxygen.exe"),
        ];
        candidates.into_iter().find(|p| p.exists())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

/// Open a project from the given root directory, loading `project.ron`.
#[derive(Event)]
pub struct OpenProjectRequest(pub PathBuf);

/// Save the current [`ActiveProject`] settings back to `project.ron`.
#[derive(Event)]
pub struct SaveProjectRequest;

/// Link the editor to a Nova-Forge game repository.
///
/// The supplied path must be the repository root (the directory that contains
/// `assets/`, `Cargo.toml`, etc.).  The path is stored in
/// `ActiveProject::settings.nova_forge_game_path`, persisted to `project.ron`,
/// and reflected into [`GameLinkState`] immediately.
#[derive(Event)]
pub struct LinkGameRequest(pub PathBuf);

/// Unlink the currently linked Nova-Forge game repository.
#[derive(Event)]
pub struct UnlinkGameRequest;

// ────────────────────────────────────────────────────────────────────────────
// Game-link status resource
// ────────────────────────────────────────────────────────────────────────────

/// Whether the editor is currently linked to a Nova-Forge game repository.
///
/// Updated every frame from [`ActiveProject`] so panels can react without
/// accessing the full project resource.
#[derive(Resource, Debug, Clone)]
pub struct GameLinkState {
    /// The path to the game repo root, if linked.
    pub game_path: Option<PathBuf>,
}

impl Default for GameLinkState {
    fn default() -> Self {
        Self { game_path: None }
    }
}

impl GameLinkState {
    pub fn is_linked(&self) -> bool { self.game_path.is_some() }

    /// Derived `assets/` path inside the linked game repository.
    pub fn assets_path(&self) -> Option<PathBuf> {
        self.game_path.as_ref().map(|p| p.join("assets"))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorProjectPlugin;

impl Plugin for EditorProjectPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ActiveProject>()
            .init_resource::<GameLinkState>()
            .add_event::<OpenProjectRequest>()
            .add_event::<SaveProjectRequest>()
            .add_event::<LinkGameRequest>()
            .add_event::<UnlinkGameRequest>()
            .add_systems(Startup, try_load_default_project)
            .add_systems(Update, (
                handle_open_project,
                handle_save_project,
                handle_link_game,
                handle_unlink_game,
                sync_game_link_state,
            ));
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Systems
// ────────────────────────────────────────────────────────────────────────────

/// Try to load `project/Config/project.ron` from the working directory.
fn try_load_default_project(mut project: ResMut<ActiveProject>) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let settings_path = cwd.join("project/Config/project.ron");
    if settings_path.exists() {
        match load_settings(&settings_path) {
            Ok(settings) => {
                info!("Loaded project settings from {}", settings_path.display());
                project.settings  = settings;
                project.root_path = Some(cwd);
            }
            Err(e) => {
                warn!("Failed to parse {}: {e}", settings_path.display());
                project.root_path = Some(cwd);
            }
        }
    } else {
        info!("No project.ron found — using default settings.");
        project.root_path = Some(cwd);
    }
}

fn handle_open_project(
    mut events:  EventReader<OpenProjectRequest>,
    mut project: ResMut<ActiveProject>,
) {
    for ev in events.read() {
        info!("Opening project at: {}", ev.0.display());
        project.root_path = Some(ev.0.clone());

        let settings_path = ev.0.join("project/Config/project.ron");
        match load_settings(&settings_path) {
            Ok(settings) => {
                info!("Loaded project settings.");
                project.settings = settings;
            }
            Err(e) => warn!("Could not load project.ron: {e}"),
        }
    }
}

fn handle_save_project(
    mut events:  EventReader<SaveProjectRequest>,
    project:     Res<ActiveProject>,
) {
    for _ev in events.read() {
        if let Some(path) = project.settings_file() {
            match save_settings(&path, &project.settings) {
                Ok(()) => info!("Project settings saved to {}", path.display()),
                Err(e) => warn!("Failed to save project settings: {e}"),
            }
        } else {
            warn!("No project root set — cannot save project.ron.");
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Disk helpers
// ────────────────────────────────────────────────────────────────────────────

fn load_settings(path: &std::path::Path) -> Result<ProjectSettings, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("read error: {e}"))?;
    ron::from_str::<ProjectSettings>(&text)
        .map_err(|e| format!("parse error: {e}"))
}

fn save_settings(path: &std::path::Path, settings: &ProjectSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir error: {e}"))?;
    }
    let text = ron::ser::to_string_pretty(settings, PrettyConfig::default())
        .map_err(|e| format!("serialise error: {e}"))?;
    std::fs::write(path, text)
        .map_err(|e| format!("write error: {e}"))
}

fn handle_link_game(
    mut events:   EventReader<LinkGameRequest>,
    mut project:  ResMut<ActiveProject>,
    mut link_ev:  EventWriter<SaveProjectRequest>,
) {
    for ev in events.read() {
        let path = ev.0.clone();
        if path.is_dir() {
            info!("Linking Nova-Forge game repository at: {}", path.display());
            project.settings.nova_forge_game_path = Some(path);
            // Persist immediately so the link survives editor restarts.
            link_ev.send(SaveProjectRequest);
        } else {
            warn!("LinkGameRequest: path is not a directory: {}", path.display());
        }
    }
}

fn handle_unlink_game(
    mut events:  EventReader<UnlinkGameRequest>,
    mut project: ResMut<ActiveProject>,
    mut save_ev: EventWriter<SaveProjectRequest>,
) {
    for _ev in events.read() {
        info!("Unlinking Nova-Forge game repository.");
        project.settings.nova_forge_game_path = None;
        save_ev.send(SaveProjectRequest);
    }
}

/// Keep [`GameLinkState`] in sync with the current [`ActiveProject`] settings.
fn sync_game_link_state(
    project: Res<ActiveProject>,
    mut state: ResMut<GameLinkState>,
) {
    let new_path = project.settings.nova_forge_game_path.clone();
    if state.game_path != new_path {
        state.game_path = new_path;
    }
}
