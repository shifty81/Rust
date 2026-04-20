//! Inventory and voxel building system.
//!
//! # Controls (when cursor is locked)
//! * **G** — mine/break the voxel the player is looking at (up to 6 m away).
//! * **B** — place the currently selected voxel type at the targeted face.
//! * **1–9** — select a hotbar slot.
//! * **Mouse wheel** — cycle hotbar selection.
//!
//! # Architecture
//! * `Inventory` resource holds item stacks (voxel type → count) + the active
//!   hotbar slot.
//! * `VoxelRaycastResult` resource caches the voxel the player is looking at
//!   each frame so the HUD and the break/place systems share the same result.
//! * The hotbar HUD is a Bevy-UI row of slot boxes rendered below the space HUD.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::biome::Voxel;
use crate::components::*;
use crate::config::*;
use crate::planet::chunk_voxel_index;

// ─────────────────────────────────────────────────────────────────────────────

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .init_resource::<VoxelRaycastResult>()
            .add_systems(Startup, setup_inventory_hud)
            .add_systems(
                Update,
                (
                    voxel_raycast,
                    handle_break_place,
                    cycle_active_slot,
                    update_inventory_hud,
                )
                    .chain(),
            );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Resources
// ─────────────────────────────────────────────────────────────────────────────

/// Total hotbar slots.
pub const HOTBAR_SLOTS: usize = 9;

/// The ordered list of voxel types that appear in the hotbar.
const HOTBAR_VOXELS: [Voxel; HOTBAR_SLOTS] = [
    Voxel::Stone,
    Voxel::Dirt,
    Voxel::Grass,
    Voxel::Sand,
    Voxel::Sandstone,
    Voxel::Snow,
    Voxel::Gravel,
    Voxel::Crystal,
    Voxel::Obsidian,
];

/// Player inventory: voxel material counts and the active hotbar slot.
#[derive(Resource)]
pub struct Inventory {
    /// Per-slot item counts, indexed by `HOTBAR_VOXELS`.
    pub counts: [u32; HOTBAR_SLOTS],
    /// Index into `HOTBAR_VOXELS` that is currently active.
    pub active_slot: usize,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            counts:      [0; HOTBAR_SLOTS],
            active_slot: 0,
        }
    }
}

impl Inventory {
    /// Return the voxel type in the currently active slot.
    pub fn active_voxel(&self) -> Voxel {
        HOTBAR_VOXELS[self.active_slot]
    }

    /// Add one unit of `voxel` to the slot it belongs to, if it has a slot.
    pub fn add(&mut self, voxel: Voxel) {
        if let Some(slot) = HOTBAR_VOXELS.iter().position(|&v| v == voxel) {
            self.counts[slot] = self.counts[slot].saturating_add(1);
        }
    }

    /// Remove one unit from the active slot.  Returns `true` on success.
    pub fn consume_active(&mut self) -> bool {
        if self.counts[self.active_slot] > 0 {
            self.counts[self.active_slot] -= 1;
            true
        } else {
            false
        }
    }
}

/// Cached result of this frame's voxel raycast.
#[derive(Resource, Default)]
pub struct VoxelRaycastResult {
    /// The chunk entity that was hit, together with the flat voxel index.
    pub hit:  Option<(Entity, usize)>,
    /// The chunk entity + index of the adjacent air voxel (placement position).
    pub face: Option<(Entity, usize)>,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Helper: world position → (chunk_coord, local_ivec3)
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum arm-reach for breaking and placing voxels (metres).
const REACH_DISTANCE: f32 = 6.0;
/// Ray-march step size (metres).
const RAY_STEP: f32 = 0.15;

fn world_to_chunk_local(pos: Vec3) -> (IVec3, IVec3) {
    let cs = (CHUNK_SIZE as f32) * VOXEL_SIZE;
    let cx = pos.x.div_euclid(cs) as i32;
    let cy = pos.y.div_euclid(cs) as i32;
    let cz = pos.z.div_euclid(cs) as i32;
    let lx = (pos.x.rem_euclid(cs) as usize).min(CHUNK_SIZE - 1) as i32;
    let ly = (pos.y.rem_euclid(cs) as usize).min(CHUNK_SIZE - 1) as i32;
    let lz = (pos.z.rem_euclid(cs) as usize).min(CHUNK_SIZE - 1) as i32;
    (IVec3::new(cx, cy, cz), IVec3::new(lx, ly, lz))
}

// ─────────────────────────────────────────────────────────────────────────────
//  Voxel raycast system
// ─────────────────────────────────────────────────────────────────────────────

/// Cast a ray from the player camera each frame and cache the voxel in front.
pub fn voxel_raycast(
    cam_q:       Query<&GlobalTransform, With<PlayerCamera>>,
    chunk_mgr:   Res<ChunkManager>,
    voxel_q:     Query<&VoxelData>,
    mut result:  ResMut<VoxelRaycastResult>,
) {
    *result = VoxelRaycastResult::default();

    let Ok(cam_tf) = cam_q.get_single() else { return };
    let origin  = cam_tf.translation();
    let forward = cam_tf.forward().normalize();

    let steps = (REACH_DISTANCE / RAY_STEP) as usize;
    let mut prev_hit: Option<(Entity, usize)> = None;

    for i in 1..=steps {
        let sample = origin + forward * (i as f32 * RAY_STEP);
        let (chunk_coord, local) = world_to_chunk_local(sample);

        let Some(&entity) = chunk_mgr.loaded.get(&chunk_coord) else {
            prev_hit = None;
            continue;
        };
        let Ok(vd) = voxel_q.get(entity) else {
            prev_hit = None;
            continue;
        };

        let idx = chunk_voxel_index(local.x as usize, local.y as usize, local.z as usize);
        if vd.0[idx].is_solid() {
            result.hit  = Some((entity, idx));
            result.face = prev_hit;
            return;
        }

        prev_hit = Some((entity, idx));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Break / place
// ─────────────────────────────────────────────────────────────────────────────

/// G = break voxel, B = place voxel.
pub fn handle_break_place(
    keyboard:      Res<ButtonInput<KeyCode>>,
    result:        Res<VoxelRaycastResult>,
    mut voxel_q:   Query<&mut VoxelData>,
    mut commands:  Commands,
    mut inventory: ResMut<Inventory>,
) {
    // ── Break voxel ──────────────────────────────────────────────────────────
    if keyboard.just_pressed(KeyCode::KeyG) {
        if let Some((entity, idx)) = result.hit {
            if let Ok(mut vd) = voxel_q.get_mut(entity) {
                let broken = vd.0[idx];
                vd.0[idx] = Voxel::Air;
                inventory.add(broken);
                commands.entity(entity).insert(ChunkDirty);
            }
        }
    }

    // ── Place voxel ──────────────────────────────────────────────────────────
    if keyboard.just_pressed(KeyCode::KeyB) {
        if let Some((entity, idx)) = result.face {
            if let Ok(mut vd) = voxel_q.get_mut(entity) {
                if !vd.0[idx].is_solid() && inventory.consume_active() {
                    vd.0[idx] = inventory.active_voxel();
                    commands.entity(entity).insert(ChunkDirty);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Hotbar slot cycling
// ─────────────────────────────────────────────────────────────────────────────

pub fn cycle_active_slot(
    keyboard:      Res<ButtonInput<KeyCode>>,
    mut wheel:     EventReader<MouseWheel>,
    mut inventory: ResMut<Inventory>,
) {
    // Number keys 1–9.
    let digit_keys = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
        KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
        KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
    ];
    for (i, key) in digit_keys.iter().enumerate() {
        if keyboard.just_pressed(*key) {
            inventory.active_slot = i;
        }
    }

    // Mouse scroll wheel.
    for ev in wheel.read() {
        if ev.y > 0.0 {
            inventory.active_slot = inventory.active_slot.saturating_sub(1);
        } else if ev.y < 0.0 {
            inventory.active_slot = (inventory.active_slot + 1).min(HOTBAR_SLOTS - 1);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Hotbar HUD
// ─────────────────────────────────────────────────────────────────────────────

/// Marker for the root node of the hotbar UI.
#[derive(Component)]
pub struct HotbarRoot;

/// Marker for one slot node; `slot_index` identifies which slot.
#[derive(Component)]
pub struct HotbarSlot {
    pub slot_index: usize,
}

/// Marker for the text node that shows item count inside a slot.
#[derive(Component)]
pub struct HotbarSlotText {
    pub slot_index: usize,
}

fn slot_bg_color(active: bool) -> BackgroundColor {
    if active {
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.55))
    } else {
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45))
    }
}

fn voxel_ui_color(voxel: Voxel) -> Color {
    let [r, g, b, _] = voxel.color();
    Color::srgb(r, g, b)
}

const SLOT_SIZE: f32 = 44.0;

fn setup_inventory_hud(mut commands: Commands) {
    // Root: centered at the bottom of the screen.
    let root = commands.spawn((
        NodeBundle {
            style: Style {
                position_type:   PositionType::Absolute,
                bottom:          Val::Px(16.0),
                left:            Val::Percent(50.0),
                margin:          UiRect::left(Val::Px(-(SLOT_SIZE * HOTBAR_SLOTS as f32) * 0.5 - 4.0 * HOTBAR_SLOTS as f32)),
                flex_direction:  FlexDirection::Row,
                column_gap:      Val::Px(4.0),
                ..default()
            },
            ..default()
        },
        HotbarRoot,
    )).id();

    for i in 0..HOTBAR_SLOTS {
        let voxel = HOTBAR_VOXELS[i];
        let bg    = slot_bg_color(i == 0);
        let vc    = voxel_ui_color(voxel);

        let slot = commands.spawn((
            NodeBundle {
                style: Style {
                    width:          Val::Px(SLOT_SIZE),
                    height:         Val::Px(SLOT_SIZE),
                    border:         UiRect::all(Val::Px(2.0)),
                    padding:        UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Column,
                    align_items:    AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                background_color: bg,
                border_color: BorderColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                ..default()
            },
            HotbarSlot { slot_index: i },
        )).id();

        // Colour swatch representing the voxel type.
        let swatch = commands.spawn(NodeBundle {
            style: Style {
                width:  Val::Px(SLOT_SIZE - 16.0),
                height: Val::Px(SLOT_SIZE - 24.0),
                ..default()
            },
            background_color: BackgroundColor(vc),
            ..default()
        }).id();

        // Item count text.
        let count_text = commands.spawn((
            TextBundle::from_section(
                "0",
                TextStyle {
                    font_size: 11.0,
                    color:     Color::WHITE,
                    ..default()
                },
            ),
            HotbarSlotText { slot_index: i },
        )).id();

        commands.entity(slot).push_children(&[swatch, count_text]);
        commands.entity(root).add_child(slot);
    }
}

pub fn update_inventory_hud(
    inventory: Res<Inventory>,
    mut slot_q: Query<(&HotbarSlot, &mut BackgroundColor)>,
    mut text_q: Query<(&HotbarSlotText, &mut Text)>,
) {
    if !inventory.is_changed() { return; }

    for (slot, mut bg) in &mut slot_q {
        *bg = slot_bg_color(slot.slot_index == inventory.active_slot);
    }
    for (slot_text, mut text) in &mut text_q {
        let count = inventory.counts[slot_text.slot_index];
        if let Some(s) = text.sections.first_mut() {
            s.value = count.to_string();
        }
    }
}
