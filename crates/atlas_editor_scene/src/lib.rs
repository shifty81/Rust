//! `atlas_editor_scene` — scene panel: open/save/new with real RON serialisation,
//! dirty-state tracking, and scene-suppress cooldown after load/new.

use bevy::prelude::*;
use atlas_scene::{
    ActiveScenePath, EditorMetadata, SceneDirty, SceneEntity, SceneFile,
    SCENE_FORMAT_VERSION,
};
use atlas_core::TransformData;
use atlas_editor_core::{EditorMode, EntityLabel};
use atlas_selection::{FocusedEntity, SelectedEntities, SelectionChanged};

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

/// Request to open a scene from the given path.
#[derive(Event)]
pub struct OpenSceneRequest(pub std::path::PathBuf);

/// Request to save the current scene (to its existing path, or warn if none).
#[derive(Event)]
pub struct SaveSceneRequest;

/// Request to create a blank new scene, clearing all user entities.
#[derive(Event)]
pub struct NewSceneRequest;

// ────────────────────────────────────────────────────────────────────────────
// Suppress-dirty cooldown (prevents load/new from immediately re-dirtying)
// ────────────────────────────────────────────────────────────────────────────

/// Counts down for N frames after a load/new operation, suppressing dirty
/// tracking so that the deferred despawn/spawn commands do not re-dirty the
/// scene the instant after it is cleared.
#[derive(Resource, Default)]
pub struct SceneSuppressDirty(pub u32);

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorScenePlugin;

impl Plugin for EditorScenePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<OpenSceneRequest>()
            .add_event::<SaveSceneRequest>()
            .add_event::<NewSceneRequest>()
            .init_resource::<SceneSuppressDirty>()
            .add_systems(
                Update,
                (handle_open, handle_save, handle_new, track_scene_dirty)
                    .run_if(in_state(EditorMode::Editing)),
            );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Transform ↔ TransformData helpers
// ────────────────────────────────────────────────────────────────────────────

fn to_transform_data(tf: &Transform) -> TransformData {
    TransformData {
        translation: [tf.translation.x, tf.translation.y, tf.translation.z],
        rotation: [tf.rotation.x, tf.rotation.y, tf.rotation.z, tf.rotation.w],
        scale: [tf.scale.x, tf.scale.y, tf.scale.z],
    }
}

fn from_transform_data(td: &TransformData) -> Transform {
    Transform {
        translation: Vec3::new(td.translation[0], td.translation[1], td.translation[2]),
        rotation:    Quat::from_xyzw(
            td.rotation[0], td.rotation[1], td.rotation[2], td.rotation[3],
        ),
        scale: Vec3::new(td.scale[0], td.scale[1], td.scale[2]),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Save
// ────────────────────────────────────────────────────────────────────────────

fn handle_save(
    mut events:  EventReader<SaveSceneRequest>,
    active_path: Res<ActiveScenePath>,
    mut dirty:   ResMut<SceneDirty>,
    entity_q:    Query<(&EntityLabel, &Transform)>,
) {
    for _ev in events.read() {
        let path = match &active_path.0 {
            Some(p) => p.clone(),
            None    => { warn!("Save requested but no scene path is set."); continue; }
        };

        let entities: Vec<SceneEntity> = entity_q
            .iter()
            .map(|(lbl, tf)| SceneEntity {
                id:             atlas_core::StableId::new(),
                name:           lbl.0.clone(),
                parent:         None,
                transform:      to_transform_data(tf),
                components:     vec![],
                prefab_instance: None,
                editor_meta:    EditorMetadata::default(),
            })
            .collect();

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_owned();

        let scene = SceneFile {
            version:       SCENE_FORMAT_VERSION,
            name:          stem,
            world_settings: Default::default(),
            entities,
        };

        let text = match ron::ser::to_string_pretty(&scene, Default::default()) {
            Ok(t)  => t,
            Err(e) => { error!("Scene serialisation error: {e}"); continue; }
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match std::fs::write(&path, text) {
            Ok(()) => {
                info!("Scene saved → {}", path.display());
                dirty.0 = false;
            }
            Err(e) => error!("Scene write error: {e}"),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Open
// ────────────────────────────────────────────────────────────────────────────

fn handle_open(
    mut events:   EventReader<OpenSceneRequest>,
    mut active_path: ResMut<ActiveScenePath>,
    mut dirty:    ResMut<SceneDirty>,
    mut suppress: ResMut<SceneSuppressDirty>,
    entity_q:     Query<Entity, With<EntityLabel>>,
    mut focused:  ResMut<FocusedEntity>,
    mut selected: ResMut<SelectedEntities>,
    mut changed:  EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    for ev in events.read() {
        let path = ev.0.clone();

        let text = match std::fs::read_to_string(&path) {
            Ok(t)  => t,
            Err(e) => { warn!("Cannot read scene {}: {e}", path.display()); continue; }
        };

        let scene: SceneFile = match ron::from_str(&text) {
            Ok(s)  => s,
            Err(e) => { error!("Scene parse error ({}): {e}", path.display()); continue; }
        };

        // Clear all existing user entities.
        for entity in entity_q.iter() {
            commands.entity(entity).despawn_recursive();
        }
        focused.0 = None;
        selected.clear();
        changed.send(SelectionChanged);

        // Spawn entities from the loaded file.
        for se in &scene.entities {
            let tf = from_transform_data(&se.transform);
            commands.spawn((
                TransformBundle { local: tf, global: GlobalTransform::default() },
                VisibilityBundle::default(),
                EntityLabel(se.name.clone()),
            ));
        }

        active_path.0 = Some(path.clone());
        dirty.0       = false;
        suppress.0    = 3; // suppress dirty for 3 frames so deferred spawns don't re-dirty
        info!("Scene loaded ← {}", path.display());
    }
}

// ────────────────────────────────────────────────────────────────────────────
// New
// ────────────────────────────────────────────────────────────────────────────

fn handle_new(
    mut events:   EventReader<NewSceneRequest>,
    mut dirty:    ResMut<SceneDirty>,
    mut path:     ResMut<ActiveScenePath>,
    mut suppress: ResMut<SceneSuppressDirty>,
    entity_q:     Query<Entity, With<EntityLabel>>,
    mut focused:  ResMut<FocusedEntity>,
    mut selected: ResMut<SelectedEntities>,
    mut changed:  EventWriter<SelectionChanged>,
    mut commands: Commands,
) {
    for _ev in events.read() {
        for entity in entity_q.iter() {
            commands.entity(entity).despawn_recursive();
        }
        focused.0 = None;
        selected.clear();
        changed.send(SelectionChanged);

        path.0     = None;
        dirty.0    = false;
        suppress.0 = 3;
        info!("New scene created.");
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Dirty tracking
// ────────────────────────────────────────────────────────────────────────────

fn track_scene_dirty(
    changed_transforms: Query<Entity, (With<EntityLabel>, Changed<Transform>)>,
    added_labels:       Query<Entity, Added<EntityLabel>>,
    mut dirty:          ResMut<SceneDirty>,
    mut suppress:       ResMut<SceneSuppressDirty>,
) {
    if suppress.0 > 0 {
        suppress.0 -= 1;
        return;
    }
    if !changed_transforms.is_empty() || !added_labels.is_empty() {
        dirty.0 = true;
    }
}
