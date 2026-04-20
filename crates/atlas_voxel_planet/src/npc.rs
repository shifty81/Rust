//! NPC dialogue and quest system.
//!
//! Biome-appropriate NPCs spawn near the player (similar to structures).
//! Walk up to an NPC and press **E** to open a dialogue panel.  Some NPCs
//! offer quests; completing a quest's objectives rewards inventory items.
//!
//! # Controls
//! * **E** — interact with nearest NPC (within 4 m).
//! * **E** (again) / **Escape** — close the dialogue panel.
//!
//! # Architecture
//! * [`NpcPlugin`] registers all systems.
//! * [`Npc`] + [`NpcKind`] + [`NpcDialogue`] components on each NPC entity.
//! * [`DialogueState`] resource: which NPC entity is currently in conversation.
//! * [`QuestLog`] resource: list of [`Quest`] structs tracking objectives.
//! * [`QuestObjective`]: what the player must deliver (item + count).
//! * When a quest is active and the player presses **E** near the quest NPC,
//!   `check_quest_turn_in` deducts items from the inventory and marks the
//!   quest complete, awarding reward items.
//!
//! # NPC types (biome → kind)
//! | Kind       | Biomes                  | Role                     |
//! |------------|-------------------------|--------------------------|
//! | Trader     | Plains / Forest / Beach | Gives a gathering quest  |
//! | Hermit     | Tundra / Mountain       | Gives a mining quest     |
//! | Nomad      | Desert / Savanna        | Gives a building quest   |
//! | Fisherman  | ShallowOcean / Beach    | Gives a gathering quest  |
//! | Explorer   | Arctic / SnowPeak       | Gives a rare-item quest  |

use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::biome::{classify_biome, Biome, Voxel};
use crate::components::*;
use crate::config::*;
use crate::inventory::Inventory;
use crate::planet::terrain_radius_at;
use crate::vegetation::simple_moisture;

// ─────────────────────────────────────────────────────────────────────────────

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueState>()
            .init_resource::<QuestLog>()
            .add_systems(Startup, setup_dialogue_ui)
            .add_systems(
                Update,
                (
                    spawn_npcs_around_player,
                    handle_interact_key,
                    update_dialogue_ui,
                    check_quest_turn_in,
                )
                    .chain(),
            );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Components
// ─────────────────────────────────────────────────────────────────────────────

/// Marks an NPC entity.
#[derive(Component)]
pub struct Npc;

/// What role this NPC plays.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcKind {
    Trader,
    Hermit,
    Nomad,
    Fisherman,
    Explorer,
}

/// Dialogue lines and optional quest index for this NPC.
#[derive(Component, Clone)]
pub struct NpcDialogue {
    /// NPC display name.
    pub name: &'static str,
    /// Greeting line shown when the player first opens dialogue.
    pub greeting: &'static str,
    /// Line shown after a quest has been completed.
    pub completion_line: &'static str,
    /// Index into `QuestLog::quests` that this NPC offers (None = no quest).
    pub quest_index: Option<usize>,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Quests
// ─────────────────────────────────────────────────────────────────────────────

/// A single quest objective: the player must have `count` of `voxel` in the
/// hotbar inventory.
#[derive(Clone)]
pub struct QuestObjective {
    pub voxel: Voxel,
    pub count: u32,
}

/// A quest the player can accept and complete.
#[derive(Clone)]
pub struct Quest {
    pub title: &'static str,
    pub description: &'static str,
    pub objective: QuestObjective,
    /// Item (voxel, count) awarded on completion.
    pub reward_voxel: Voxel,
    pub reward_count: u32,
    pub status: QuestStatus,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuestStatus {
    Available,
    Active,
    Complete,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Resources
// ─────────────────────────────────────────────────────────────────────────────

/// Which NPC is currently in dialogue (None = panel closed).
#[derive(Resource, Default)]
pub struct DialogueState {
    /// The NPC entity this player is talking to.
    pub npc_entity: Option<Entity>,
}

/// Tracks all quests in the game.
#[derive(Resource)]
pub struct QuestLog {
    pub quests: Vec<Quest>,
}

impl Default for QuestLog {
    fn default() -> Self {
        Self {
            quests: vec![
                Quest {
                    title:       "Gravel Run",
                    description: "The Trader needs 5 pieces of Gravel for road work.",
                    objective:   QuestObjective { voxel: Voxel::Gravel, count: 5 },
                    reward_voxel: Voxel::Sandstone,
                    reward_count: 4,
                    status:       QuestStatus::Available,
                },
                Quest {
                    title:       "Stone Supply",
                    description: "The Hermit wants 6 chunks of Stone to patch his shelter.",
                    objective:   QuestObjective { voxel: Voxel::Stone, count: 6 },
                    reward_voxel: Voxel::Crystal,
                    reward_count: 2,
                    status:       QuestStatus::Available,
                },
                Quest {
                    title:       "Sand Dunes",
                    description: "The Nomad requests 8 handfuls of Sand for tent repairs.",
                    objective:   QuestObjective { voxel: Voxel::Sand, count: 8 },
                    reward_voxel: Voxel::Obsidian,
                    reward_count: 2,
                    status:       QuestStatus::Available,
                },
                Quest {
                    title:       "Snowball Supply",
                    description: "The Fisherman needs 4 blocks of Snow to keep fish fresh.",
                    objective:   QuestObjective { voxel: Voxel::Snow, count: 4 },
                    reward_voxel: Voxel::Gravel,
                    reward_count: 6,
                    status:       QuestStatus::Available,
                },
                Quest {
                    title:       "Crystal Cache",
                    description: "The Explorer seeks 3 Crystal shards for navigation.",
                    objective:   QuestObjective { voxel: Voxel::Crystal, count: 3 },
                    reward_voxel: Voxel::Stone,
                    reward_count: 10,
                    status:       QuestStatus::Available,
                },
            ],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  UI components
// ─────────────────────────────────────────────────────────────────────────────

/// Marks the root dialogue panel node.
#[derive(Component)]
pub struct DialoguePanel;

/// The NPC name text inside the panel.
#[derive(Component)]
pub struct DialogueNpcName;

/// The NPC greeting / quest body text.
#[derive(Component)]
pub struct DialogueBody;

/// The quest status / accept / turn-in text.
#[derive(Component)]
pub struct DialogueQuestLine;

// ─────────────────────────────────────────────────────────────────────────────
//  Constants
// ─────────────────────────────────────────────────────────────────────────────

const MAX_NPCS: usize = 6;
const NPC_INTERACT_RADIUS: f32 = 4.0;
const DESPAWN_RADIUS_FACTOR: f32 = 1.6;
const SPAWN_ATTEMPTS: usize = 2;
const MIN_SPAWN_DIST: f32 = 8.0;

// ─────────────────────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn npc_for_biome(biome: Biome, rng: &mut impl Rng) -> Option<(NpcKind, &'static str, usize)> {
    let roll: f32 = rng.gen();
    match biome {
        Biome::Plains | Biome::Forest | Biome::TropicalForest => {
            if roll < 0.05 { Some((NpcKind::Trader,    "Elara",  0)) } else { None }
        }
        Biome::Tundra | Biome::Mountain => {
            if roll < 0.04 { Some((NpcKind::Hermit,    "Borin",  1)) } else { None }
        }
        Biome::Desert | Biome::Savanna => {
            if roll < 0.04 { Some((NpcKind::Nomad,     "Zara",   2)) } else { None }
        }
        Biome::Beach | Biome::ShallowOcean => {
            if roll < 0.05 { Some((NpcKind::Fisherman, "Cael",   3)) } else { None }
        }
        Biome::Arctic | Biome::SnowPeak => {
            if roll < 0.03 { Some((NpcKind::Explorer,  "Vira",   4)) } else { None }
        }
        _ => None,
    }
}

const NPC_GREETINGS: [&str; 5] = [
    "Hello traveller! I could use some supplies — do you have any Gravel? [Quest: Gravel Run]",
    "Brrr! Hard life up here. I need Stone for repairs. [Quest: Stone Supply]",
    "The desert demands tribute. Bring me Sand if you seek my wisdom. [Quest: Sand Dunes]",
    "Ahoy! I need Snow to keep my catch fresh. Trade? [Quest: Snowball Supply]",
    "The stars guided me here. Crystal shards are what I seek. [Quest: Crystal Cache]",
];

const NPC_COMPLETIONS: [&str; 5] = [
    "Wonderful! Take this Sandstone — may it serve you well.",
    "Excellent craftsmanship, friend. Here, a Crystal for your trouble.",
    "The dunes thank you. This Obsidian is yours.",
    "My fish will keep nicely now. Have some Gravel for your garden.",
    "Magnificent! These will guide me home. Ten blocks of Stone, as promised.",
];

fn npc_dialogue(quest_index: usize) -> NpcDialogue {
    NpcDialogue {
        name:            ["Elara", "Borin", "Zara", "Cael", "Vira"][quest_index],
        greeting:        NPC_GREETINGS[quest_index],
        completion_line: NPC_COMPLETIONS[quest_index],
        quest_index:     Some(quest_index),
    }
}

fn inventory_count(inventory: &Inventory, voxel: Voxel) -> u32 {
    const HOTBAR_VOXELS: [Voxel; 9] = [
        Voxel::Stone, Voxel::Dirt, Voxel::Grass, Voxel::Sand,
        Voxel::Sandstone, Voxel::Snow, Voxel::Gravel, Voxel::Crystal,
        Voxel::Obsidian,
    ];
    HOTBAR_VOXELS.iter()
        .position(|&v| v == voxel)
        .map(|slot| inventory.counts[slot])
        .unwrap_or(0)
}

fn deduct_inventory(inventory: &mut Inventory, voxel: Voxel, count: u32) {
    const HOTBAR_VOXELS: [Voxel; 9] = [
        Voxel::Stone, Voxel::Dirt, Voxel::Grass, Voxel::Sand,
        Voxel::Sandstone, Voxel::Snow, Voxel::Gravel, Voxel::Crystal,
        Voxel::Obsidian,
    ];
    if let Some(slot) = HOTBAR_VOXELS.iter().position(|&v| v == voxel) {
        inventory.counts[slot] = inventory.counts[slot].saturating_sub(count);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Spawn NPC mesh — a simple humanoid silhouette (box body + sphere head)
// ─────────────────────────────────────────────────────────────────────────────

fn spawn_npc(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos:       Vec3,
    up:        Vec3,
    kind:      NpcKind,
    dialogue:  NpcDialogue,
) {
    let body_color = match kind {
        NpcKind::Trader    => Color::srgb(0.55, 0.38, 0.20),
        NpcKind::Hermit    => Color::srgb(0.35, 0.35, 0.40),
        NpcKind::Nomad     => Color::srgb(0.78, 0.62, 0.30),
        NpcKind::Fisherman => Color::srgb(0.25, 0.45, 0.65),
        NpcKind::Explorer  => Color::srgb(0.60, 0.80, 0.90),
    };

    let body_mat = materials.add(StandardMaterial {
        base_color:        body_color,
        perceptual_roughness: 0.9,
        ..default()
    });
    let head_mat = materials.add(StandardMaterial {
        base_color:        Color::srgb(0.85, 0.72, 0.58),
        perceptual_roughness: 0.8,
        ..default()
    });

    let body_mesh = meshes.add(Cuboid::new(0.5, 0.8, 0.3));
    let head_mesh = meshes.add(Sphere::new(0.22));

    let look_rot = Quat::from_rotation_arc(Vec3::Y, up);

    let root = commands.spawn((
        TransformBundle::from_transform(Transform {
            translation: pos,
            rotation:    look_rot,
            ..default()
        }),
        VisibilityBundle::default(),
        Npc,
        kind,
        dialogue,
        Name::new("NPC"),
    )).id();

    let body = commands.spawn(PbrBundle {
        mesh:      body_mesh,
        material:  body_mat,
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    }).id();

    let head = commands.spawn(PbrBundle {
        mesh:      head_mesh,
        material:  head_mat,
        transform: Transform::from_xyz(0.0, 1.1, 0.0),
        ..default()
    }).id();

    commands.entity(root).push_children(&[body, head]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Systems
// ─────────────────────────────────────────────────────────────────────────────

pub fn spawn_npcs_around_player(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q:      Query<&Transform, With<Player>>,
    npc_q:         Query<(Entity, &Transform), (With<Npc>, Without<Player>)>,
    seed:          Res<NoiseSeed>,
    world_settings: Res<WorldSettings>,
) {
    let Ok(player_tf) = player_q.get_single() else { return };
    let player_pos = player_tf.translation;
    let local_up   = player_pos.normalize_or_zero();

    let altitude = player_pos.length() - PLANET_RADIUS;
    if altitude > ATMOSPHERE_FADE_START { return; }

    let spawn_r   = world_settings.vegetation_radius * 1.2;
    let despawn_r = spawn_r * DESPAWN_RADIUS_FACTOR;

    for (entity, tf) in &npc_q {
        if (tf.translation - player_pos).length() > despawn_r {
            commands.entity(entity).despawn_recursive();
        }
    }

    let existing = npc_q
        .iter()
        .filter(|(_, tf)| (tf.translation - player_pos).length() < spawn_r)
        .count();

    if existing >= MAX_NPCS { return; }

    let ref_right = if local_up.abs().dot(Vec3::X) < 0.9 {
        Vec3::X.cross(local_up).normalize()
    } else {
        Vec3::Z.cross(local_up).normalize()
    };
    let ref_fwd = local_up.cross(ref_right).normalize();

    let mut rng = rand::thread_rng();
    let mut spawned = 0;

    for _ in 0..SPAWN_ATTEMPTS {
        if existing + spawned >= MAX_NPCS { break; }

        let angle  = rng.gen_range(0.0f32..2.0 * PI);
        let spread = rng.gen_range(MIN_SPAWN_DIST..spawn_r);

        let horiz    = ref_right * angle.cos() + ref_fwd * angle.sin();
        let cand_dir = (local_up + horiz * spread / PLANET_RADIUS).normalize();

        let surface_r = terrain_radius_at(cand_dir, seed.0);
        let alt       = surface_r - PLANET_RADIUS;
        let lat       = cand_dir.y;
        let moisture  = simple_moisture(cand_dir, seed.0);
        let biome     = classify_biome(lat, alt, moisture);

        let Some((kind, _, quest_idx)) = npc_for_biome(biome, &mut rng) else { continue };

        let pos = cand_dir * (surface_r + 0.05);
        spawn_npc(&mut commands, &mut meshes, &mut materials, pos, cand_dir, kind,
                  npc_dialogue(quest_idx));
        spawned += 1;
    }
}

/// E key: open dialogue with the nearest NPC within `NPC_INTERACT_RADIUS`.
/// If dialogue is already open, close it.
pub fn handle_interact_key(
    keyboard:     Res<ButtonInput<KeyCode>>,
    player_q:     Query<&Transform, With<Player>>,
    npc_q:        Query<(Entity, &Transform), With<Npc>>,
    dialogue_q:   Query<&NpcDialogue>,
    mut state:    ResMut<DialogueState>,
    mut quest_log: ResMut<QuestLog>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) { return; }

    // Close if already open.
    if state.npc_entity.is_some() {
        state.npc_entity = None;
        return;
    }

    let Ok(player_tf) = player_q.get_single() else { return };
    let player_pos = player_tf.translation;

    // Find closest NPC within reach.
    let mut closest: Option<(Entity, f32)> = None;
    for (entity, tf) in &npc_q {
        let dist = (tf.translation - player_pos).length();
        if dist < NPC_INTERACT_RADIUS {
            if closest.map_or(true, |(_, d)| dist < d) {
                closest = Some((entity, dist));
            }
        }
    }

    if let Some((entity, _)) = closest {
        state.npc_entity = Some(entity);

        // Activate the quest for this NPC if still Available.
        if let Ok(dialogue) = dialogue_q.get(entity) {
            if let Some(qi) = dialogue.quest_index {
                if let Some(quest) = quest_log.quests.get_mut(qi) {
                    if quest.status == QuestStatus::Available {
                        quest.status = QuestStatus::Active;
                    }
                }
            }
        }
    }
}

/// Turn in quest when player talks to the NPC again with enough items.
pub fn check_quest_turn_in(
    mut state:     ResMut<DialogueState>,
    dialogue_q:    Query<&NpcDialogue>,
    mut inventory: ResMut<Inventory>,
    mut quest_log: ResMut<QuestLog>,
) {
    let Some(npc_entity) = state.npc_entity else { return };
    let Ok(dialogue) = dialogue_q.get(npc_entity) else { return };
    let Some(qi) = dialogue.quest_index else { return };

    let quests = &mut quest_log.quests;
    if qi >= quests.len() { return; }

    if quests[qi].status != QuestStatus::Active { return; }

    let obj_voxel = quests[qi].objective.voxel;
    let obj_count = quests[qi].objective.count;
    let have      = inventory_count(&inventory, obj_voxel);

    if have >= obj_count {
        deduct_inventory(&mut inventory, obj_voxel, obj_count);
        let reward_v = quests[qi].reward_voxel;
        let reward_n = quests[qi].reward_count;
        for _ in 0..reward_n {
            inventory.add(reward_v);
        }
        quests[qi].status = QuestStatus::Complete;
        // Close dialogue so it shows completion message on re-open.
        state.npc_entity = None;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Dialogue UI
// ─────────────────────────────────────────────────────────────────────────────

pub fn setup_dialogue_ui(mut commands: Commands) {
    // Panel: centred bottom-third of screen.
    let panel = commands.spawn((
        NodeBundle {
            style: Style {
                position_type:   PositionType::Absolute,
                bottom:          Val::Px(130.0),
                left:            Val::Percent(20.0),
                width:           Val::Percent(60.0),
                flex_direction:  FlexDirection::Column,
                padding:         UiRect::all(Val::Px(14.0)),
                row_gap:         Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.05, 0.05, 0.10, 0.92)),
            visibility: Visibility::Hidden,
            ..default()
        },
        DialoguePanel,
    )).id();

    let name_text = commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 16.0,
                color:     Color::srgba(1.0, 0.9, 0.5, 1.0),
                ..default()
            },
        ),
        DialogueNpcName,
    )).id();

    let body_text = commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 13.0,
                color:     Color::WHITE,
                ..default()
            },
        ),
        DialogueBody,
    )).id();

    let quest_text = commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 12.0,
                color:     Color::srgba(0.75, 1.0, 0.75, 1.0),
                ..default()
            },
        ),
        DialogueQuestLine,
    )).id();

    let hint_text = commands.spawn(TextBundle::from_section(
        "[E] Close",
        TextStyle {
            font_size: 11.0,
            color:     Color::srgba(0.6, 0.6, 0.6, 1.0),
            ..default()
        },
    )).id();

    commands.entity(panel).push_children(&[name_text, body_text, quest_text, hint_text]);
}

pub fn update_dialogue_ui(
    state:      Res<DialogueState>,
    quest_log:  Res<QuestLog>,
    inventory:  Res<Inventory>,
    dialogue_q: Query<&NpcDialogue>,
    mut panel_q: Query<&mut Visibility, With<DialoguePanel>>,
    mut name_q:  Query<&mut Text, (With<DialogueNpcName>, Without<DialogueBody>, Without<DialogueQuestLine>)>,
    mut body_q:  Query<&mut Text, (With<DialogueBody>,    Without<DialogueNpcName>, Without<DialogueQuestLine>)>,
    mut quest_q: Query<&mut Text, (With<DialogueQuestLine>, Without<DialogueNpcName>, Without<DialogueBody>)>,
) {
    let open = state.npc_entity.is_some();

    for mut vis in &mut panel_q {
        *vis = if open { Visibility::Inherited } else { Visibility::Hidden };
    }

    if !open { return; }

    let Some(npc_entity) = state.npc_entity else { return };
    let Ok(dialogue) = dialogue_q.get(npc_entity) else { return };

    for mut t in &mut name_q {
        if let Some(s) = t.sections.first_mut() {
            s.value = dialogue.name.to_string();
        }
    }

    for mut t in &mut body_q {
        if let Some(s) = t.sections.first_mut() {
            s.value = dialogue.greeting.to_string();
        }
    }

    for mut t in &mut quest_q {
        if let Some(s) = t.sections.first_mut() {
            s.value = if let Some(qi) = dialogue.quest_index {
                if let Some(quest) = quest_log.quests.get(qi) {
                    match quest.status {
                        QuestStatus::Available => format!("Quest: {} — {}", quest.title, quest.description),
                        QuestStatus::Active => {
                            let have = inventory_count(&inventory, quest.objective.voxel);
                            format!(
                                "Active: {} [{}/{} {:?}]  — {}",
                                quest.title, have, quest.objective.count, quest.objective.voxel,
                                if have >= quest.objective.count { "Ready to turn in! (talk again)" } else { "Collect more" }
                            )
                        }
                        QuestStatus::Complete => {
                            format!("✓ {} complete — {}", quest.title, dialogue.completion_line)
                        }
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
        }
    }
}
