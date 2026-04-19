//! `atlas_editor_core` — editor modes, panel contracts, docking layout, shared events.

use bevy::prelude::*;
use bevy::ecs::event::ManualEventReader;
use atlas_commands::{EditorCommand, EditorCommandContext, CommandHistory};
use atlas_selection::{FocusedEntity, SelectedEntities, SelectionChanged};

// ────────────────────────────────────────────────────────────────────────────
// Shared entity metadata
// ────────────────────────────────────────────────────────────────────────────

/// Display name shown in the outliner and details panel for an entity.
#[derive(Component, Default, Clone)]
pub struct EntityLabel(pub String);

/// Marks the camera used by the editor viewport.
#[derive(Component)]
pub struct EditorCamera;

// ────────────────────────────────────────────────────────────────────────────
// Editor mode state machine
// ────────────────────────────────────────────────────────────────────────────

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EditorMode {
    #[default]
    Editing,
    PlayingInEditor,
    Simulating,
    Paused,
}

// ────────────────────────────────────────────────────────────────────────────
// Panel IDs
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Viewport,
    Outliner,
    Details,
    ContentBrowser,
    OutputLog,
    Scene,
}

// ────────────────────────────────────────────────────────────────────────────
// Shared editor events
// ────────────────────────────────────────────────────────────────────────────

#[derive(Event, Debug)]
pub struct RequestEditorMode(pub EditorMode);

#[derive(Event, Debug)]
pub struct RefreshPanels;

// ────────────────────────────────────────────────────────────────────────────
// Entity creation — Create menu
// ────────────────────────────────────────────────────────────────────────────

/// Primitive kinds that can be created from the Create menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Blank,
    Cube,
    Sphere,
    Plane,
    DirectionalLight,
    PointLight,
}

impl PrimitiveKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Blank            => "Entity",
            Self::Cube             => "Cube",
            Self::Sphere           => "Sphere",
            Self::Plane            => "Plane",
            Self::DirectionalLight => "Directional Light",
            Self::PointLight       => "Point Light",
        }
    }
}

/// Records which primitive kind was used to create an entity.
/// Enables accurate duplication.
#[derive(Component, Clone, Copy)]
pub struct SpawnedFromKind(pub PrimitiveKind);

/// Spawn a new primitive entity at the world origin.
#[derive(Event, Debug, Clone, Copy)]
pub struct SpawnEntityRequest(pub PrimitiveKind);

/// Despawn the given entity (undo-able).
#[derive(Event, Debug, Clone, Copy)]
pub struct DeleteEntityRequest(pub Entity);

/// Duplicate the given entity (spawns a copy with " (Copy)" suffix).
#[derive(Event, Debug, Clone, Copy)]
pub struct DuplicateEntityRequest(pub Entity);

// ────────────────────────────────────────────────────────────────────────────
// DeleteEntityCommand (undo-able despawn)
// ────────────────────────────────────────────────────────────────────────────

pub struct DeleteEntityCommand {
    entity:    Entity,
    label:     String,
    transform: Transform,
}

impl EditorCommand for DeleteEntityCommand {
    fn apply(&mut self, ctx: &mut EditorCommandContext) {
        if ctx.world.get_entity(self.entity).is_some() {
            ctx.world.despawn(self.entity);
        }
    }

    fn undo(&mut self, ctx: &mut EditorCommandContext) {
        // Re-create the entity; update self.entity so redo targets the right one.
        let new_entity = ctx.world.spawn((
            TransformBundle { local: self.transform, global: GlobalTransform::default() },
            VisibilityBundle::default(),
            EntityLabel(self.label.clone()),
        )).id();
        self.entity = new_entity;
    }

    fn label(&self) -> &str { "Delete" }
}

// ────────────────────────────────────────────────────────────────────────────
// Cursor for exclusive delete system
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct DeleteEntityCursor(ManualEventReader<DeleteEntityRequest>);

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorCorePlugin;

impl Plugin for EditorCorePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<EditorMode>()
            .add_event::<RequestEditorMode>()
            .add_event::<RefreshPanels>()
            .add_event::<SpawnEntityRequest>()
            .add_event::<DeleteEntityRequest>()
            .add_event::<DuplicateEntityRequest>()
            .init_resource::<DeleteEntityCursor>()
            .add_systems(
                Update,
                (
                    handle_mode_requests,
                    handle_spawn_entity,
                    handle_duplicate_entity,
                )
                    .chain(),
            )
            .add_systems(Update, handle_delete_entity);
    }
}

fn handle_mode_requests(
    mut events: EventReader<RequestEditorMode>,
    mut next:   ResMut<NextState<EditorMode>>,
) {
    for ev in events.read() {
        next.set(ev.0);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Primitive spawn helper (used by both spawn and duplicate handlers)
// ────────────────────────────────────────────────────────────────────────────

fn spawn_kind(
    kind:      PrimitiveKind,
    transform: Transform,
    label:     EntityLabel,
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    match kind {
        PrimitiveKind::Blank => {
            commands.spawn((
                TransformBundle { local: transform, global: GlobalTransform::default() },
                VisibilityBundle::default(),
                label,
                SpawnedFromKind(kind),
            )).id()
        }

        PrimitiveKind::Cube => {
            let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
            let mat  = materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.72),
                perceptual_roughness: 0.8,
                ..default()
            });
            commands.spawn((
                PbrBundle { mesh, material: mat, transform, ..default() },
                label,
                SpawnedFromKind(kind),
            )).id()
        }

        PrimitiveKind::Sphere => {
            let mesh = meshes.add(Sphere::new(0.5).mesh().uv(32, 18));
            let mat  = materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.72),
                perceptual_roughness: 0.8,
                ..default()
            });
            commands.spawn((
                PbrBundle { mesh, material: mat, transform, ..default() },
                label,
                SpawnedFromKind(kind),
            )).id()
        }

        PrimitiveKind::Plane => {
            let mesh = meshes.add(Plane3d::default().mesh().size(5.0, 5.0));
            let mat  = materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                perceptual_roughness: 1.0,
                ..default()
            });
            commands.spawn((
                PbrBundle { mesh, material: mat, transform, ..default() },
                label,
                SpawnedFromKind(kind),
            )).id()
        }

        PrimitiveKind::DirectionalLight => {
            commands.spawn((
                DirectionalLightBundle {
                    directional_light: DirectionalLight {
                        illuminance: 10_000.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    transform,
                    ..default()
                },
                label,
                SpawnedFromKind(kind),
            )).id()
        }

        PrimitiveKind::PointLight => {
            commands.spawn((
                PointLightBundle {
                    point_light: PointLight {
                        intensity: 800.0,
                        radius: 0.1,
                        shadows_enabled: true,
                        ..default()
                    },
                    transform,
                    ..default()
                },
                label,
                SpawnedFromKind(kind),
            )).id()
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Spawn entity handler
// ────────────────────────────────────────────────────────────────────────────

fn handle_spawn_entity(
    mut events:    EventReader<SpawnEntityRequest>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut focused:   ResMut<FocusedEntity>,
    mut selected:  ResMut<SelectedEntities>,
    mut changed:   EventWriter<SelectionChanged>,
) {
    for ev in events.read() {
        let kind  = ev.0;
        let label = EntityLabel(kind.label().into());

        let entity = spawn_kind(
            kind,
            Transform::default(),
            label,
            &mut commands,
            &mut meshes,
            &mut materials,
        );

        focused.0 = Some(entity);
        selected.set_single(entity);
        changed.send(SelectionChanged);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Duplicate entity handler (regular system — no undo for simplicity)
// ────────────────────────────────────────────────────────────────────────────

fn handle_duplicate_entity(
    mut events:    EventReader<DuplicateEntityRequest>,
    entity_q:      Query<(&EntityLabel, &Transform, Option<&SpawnedFromKind>)>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut focused:   ResMut<FocusedEntity>,
    mut selected:  ResMut<SelectedEntities>,
    mut changed:   EventWriter<SelectionChanged>,
) {
    for ev in events.read() {
        let entity = ev.0;
        let Ok((label, transform, kind_opt)) = entity_q.get(entity) else { continue };

        let new_tf = Transform {
            translation: transform.translation + Vec3::new(0.5, 0.5, 0.5),
            ..*transform
        };
        let new_label = EntityLabel(format!("{} (Copy)", label.0));
        let kind = kind_opt.map(|k| k.0).unwrap_or(PrimitiveKind::Blank);

        let new_entity = spawn_kind(
            kind,
            new_tf,
            new_label,
            &mut commands,
            &mut meshes,
            &mut materials,
        );

        focused.0 = Some(new_entity);
        selected.set_single(new_entity);
        changed.send(SelectionChanged);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Delete entity handler (exclusive system — undo-able)
// ────────────────────────────────────────────────────────────────────────────

fn handle_delete_entity(world: &mut World) {
    let entities_to_delete: Vec<Entity> = world.resource_scope(
        |world, mut cursor: Mut<DeleteEntityCursor>| {
            let events = world.resource::<Events<DeleteEntityRequest>>();
            cursor.0.read(events).map(|r| r.0).collect()
        },
    );

    for entity in entities_to_delete {
        let label = world
            .get::<EntityLabel>(entity)
            .map(|l| l.0.clone())
            .unwrap_or_default();
        let transform = world
            .get::<Transform>(entity)
            .copied()
            .unwrap_or_default();

        // Execute via CommandHistory so the action is undo-able.
        world.resource_scope(|world, mut history: Mut<CommandHistory>| {
            history.execute(
                Box::new(DeleteEntityCommand { entity, label, transform }),
                world,
            );
        });

        // Clear focus / selection (entity is now gone).
        if world.resource::<FocusedEntity>().0 == Some(entity) {
            world.resource_mut::<FocusedEntity>().0 = None;
        }
        world.resource_mut::<SelectedEntities>().0.remove(&entity);
        world.send_event(SelectionChanged);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Concrete editor commands
// ────────────────────────────────────────────────────────────────────────────

/// Rename an entity's [`EntityLabel`] — supports undo/redo.
pub struct RenameEntityCommand {
    pub entity:   Entity,
    pub old_name: String,
    pub new_name: String,
}

impl EditorCommand for RenameEntityCommand {
    fn apply(&mut self, ctx: &mut EditorCommandContext) {
        if let Some(mut lbl) = ctx.world.get_mut::<EntityLabel>(self.entity) {
            lbl.0 = self.new_name.clone();
        }
    }
    fn undo(&mut self, ctx: &mut EditorCommandContext) {
        if let Some(mut lbl) = ctx.world.get_mut::<EntityLabel>(self.entity) {
            lbl.0 = self.old_name.clone();
        }
    }
    fn label(&self) -> &str { "Rename" }
}


