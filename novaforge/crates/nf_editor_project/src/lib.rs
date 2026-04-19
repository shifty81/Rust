//! `nf_editor_project` — project loading, settings, content root paths.

use bevy::prelude::*;
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
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            version:       PROJECT_SETTINGS_VERSION,
            project_name:  "NovaForge Project".into(),
            content_root:  PathBuf::from("project/Content"),
            scenes_root:   PathBuf::from("project/Scenes"),
            prefabs_root:  PathBuf::from("project/Prefabs"),
            cache_root:    PathBuf::from("project/Cache"),
            default_scene: None,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Active project resource
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct ActiveProject {
    pub settings: ProjectSettings,
    /// Absolute path to the project root directory.
    pub root_path: Option<PathBuf>,
}

impl ActiveProject {
    pub fn content_path(&self) -> Option<PathBuf> {
        self.root_path
            .as_ref()
            .map(|r| r.join(&self.settings.content_root))
    }

    pub fn scenes_path(&self) -> Option<PathBuf> {
        self.root_path
            .as_ref()
            .map(|r| r.join(&self.settings.scenes_root))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

/// Request to open a project from the given root directory.
#[derive(Event)]
pub struct OpenProjectRequest(pub PathBuf);

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorProjectPlugin;

impl Plugin for EditorProjectPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ActiveProject>()
            .add_event::<OpenProjectRequest>()
            .add_systems(Update, handle_open_project);
    }
}

fn handle_open_project(
    mut events:  EventReader<OpenProjectRequest>,
    mut project: ResMut<ActiveProject>,
) {
    for ev in events.read() {
        info!("Opening project at: {}", ev.0.display());
        project.root_path = Some(ev.0.clone());
        // Load project.ron settings from disk (Phase 1 implementation).
    }
}
