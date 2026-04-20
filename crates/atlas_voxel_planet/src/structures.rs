//! Procedural city / structure generation.
//!
//! Small procedurally-placed buildings are scattered around the player in
//! suitable biomes.  Each structure is a set of child `PbrBundle` entities
//! (walls, roof, entrance) parented under a root transform, exactly like trees
//! in [`crate::vegetation`].
//!
//! # Structure types
//! | Kind          | Biomes                           | Description                       |
//! |---------------|----------------------------------|-----------------------------------|
//! `Hut`          | Plains, Forest, Savanna          | Stone-wall hut with flat slab roof|
//! `SandstoneHut` | Desert, Beach                    | Sandstone variant                 |
//! `WatchTower`   | Tundra, Mountain                 | Tall stone tower, no roof         |
//! `IceHut`       | Arctic, SnowPeak                 | Snow-walled dome hut              |
//! `Ruin`         | any (rare)                       | Partial walls, no roof            |

use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::biome::{classify_biome, Biome};
use crate::components::*;
use crate::config::*;
use crate::planet::terrain_radius_at;
use crate::vegetation::simple_moisture;

// ─────────────────────────────────────────────────────────────────────────────

pub struct StructuresPlugin;

impl Plugin for StructuresPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_structures_around_player);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Components
// ─────────────────────────────────────────────────────────────────────────────

/// Marks a spawned structure entity.
#[derive(Component)]
pub struct Structure;

/// Which kind of structure was generated.
#[derive(Component, Clone, Copy, Debug)]
pub enum StructureKind {
    Hut,
    SandstoneHut,
    WatchTower,
    IceHut,
    Ruin,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Max structures allowed within `vegetation_radius` of the player.
const MAX_STRUCTURES: usize = 8;
/// Extra radius multiplier before despawning (structures are larger, so kept
/// slightly longer than vegetation).
const DESPAWN_RADIUS_FACTOR: f32 = 1.6;
/// Placement attempts per frame.
const SPAWN_ATTEMPTS: usize = 2;
/// Minimum distance from player at which a structure may appear.
const MIN_SPAWN_DIST: f32 = 25.0;

// ─────────────────────────────────────────────────────────────────────────────
//  Spawn system
// ─────────────────────────────────────────────────────────────────────────────

pub fn spawn_structures_around_player(
    mut commands:   Commands,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut materials:  ResMut<Assets<StandardMaterial>>,
    player_q:       Query<&Transform, With<Player>>,
    struct_q:       Query<(Entity, &Transform), (With<Structure>, Without<Player>)>,
    seed:           Res<NoiseSeed>,
    world_settings: Res<WorldSettings>,
) {
    let Ok(player_tf) = player_q.get_single() else { return };
    let player_pos  = player_tf.translation;
    let local_up    = player_pos.normalize_or_zero();

    // Only on the surface.
    let altitude = player_pos.length() - PLANET_RADIUS;
    if altitude > ATMOSPHERE_FADE_START { return; }

    let spawn_r   = world_settings.vegetation_radius * 1.4;
    let despawn_r = spawn_r * DESPAWN_RADIUS_FACTOR;

    // Despawn distant structures.
    for (entity, tf) in &struct_q {
        if (tf.translation - player_pos).length() > despawn_r {
            commands.entity(entity).despawn_recursive();
        }
    }

    let existing = struct_q
        .iter()
        .filter(|(_, tf)| (tf.translation - player_pos).length() < spawn_r)
        .count();

    if existing >= MAX_STRUCTURES { return; }

    let ref_right = if local_up.abs().dot(Vec3::X) < 0.9 {
        Vec3::X.cross(local_up).normalize()
    } else {
        Vec3::Z.cross(local_up).normalize()
    };
    let ref_fwd = local_up.cross(ref_right).normalize();

    let mut rng = rand::thread_rng();
    let mut spawned = 0;

    for _ in 0..SPAWN_ATTEMPTS {
        if existing + spawned >= MAX_STRUCTURES { break; }

        let angle  = rng.gen_range(0.0f32..2.0 * PI);
        let spread = rng.gen_range(MIN_SPAWN_DIST..spawn_r);

        let horiz    = ref_right * angle.cos() + ref_fwd * angle.sin();
        let cand_dir = (local_up + horiz * spread / PLANET_RADIUS).normalize();

        let surface_r = terrain_radius_at(cand_dir, seed.0);
        let alt       = surface_r - PLANET_RADIUS;
        let lat       = cand_dir.y;
        let moisture  = simple_moisture(cand_dir, seed.0);
        let biome     = classify_biome(lat, alt, moisture);

        let Some(kind) = structure_for_biome(biome, &mut rng) else { continue };

        // Lift the structure base by one voxel so it sits on top of the
        // highest solid voxel column rather than embedded in it (chunk
        // generation fills up to `surface_r + VOXEL_SIZE`).
        let pos = cand_dir * (surface_r + VOXEL_SIZE);
        spawn_structure(&mut commands, &mut meshes, &mut materials, pos, cand_dir, kind, &mut rng);
        spawned += 1;
    }
}

/// Pick a structure kind for the biome (or `None` if none should spawn).
fn structure_for_biome(biome: Biome, rng: &mut impl Rng) -> Option<StructureKind> {
    let roll: f32 = rng.gen_range(0.0..1.0);
    match biome {
        Biome::Plains | Biome::Savanna | Biome::Forest | Biome::TropicalForest => {
            if roll < 0.006 { Some(StructureKind::Hut) }
            else if roll < 0.007 { Some(StructureKind::Ruin) }
            else { None }
        }
        Biome::Desert | Biome::Beach => {
            if roll < 0.005 { Some(StructureKind::SandstoneHut) }
            else if roll < 0.006 { Some(StructureKind::Ruin) }
            else { None }
        }
        Biome::Tundra | Biome::Mountain => {
            if roll < 0.004 { Some(StructureKind::WatchTower) }
            else if roll < 0.005 { Some(StructureKind::Ruin) }
            else { None }
        }
        Biome::Arctic | Biome::SnowPeak => {
            if roll < 0.004 { Some(StructureKind::IceHut) }
            else { None }
        }
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Structure builder
// ─────────────────────────────────────────────────────────────────────────────

fn spawn_structure(
    commands:   &mut Commands,
    meshes:     &mut Assets<Mesh>,
    materials:  &mut Assets<StandardMaterial>,
    position:   Vec3,
    surface_up: Vec3,
    kind:       StructureKind,
    rng:        &mut impl Rng,
) {
    // Orient root so local-Y = surface normal.
    let ref_vec  = if surface_up.abs().dot(Vec3::X) < 0.9 { Vec3::X } else { Vec3::Z };
    let right    = surface_up.cross(ref_vec).normalize();
    let forward  = right.cross(surface_up).normalize();
    // Random yaw so structures face different directions.
    let yaw      = rng.gen_range(0.0f32..2.0 * PI);
    let yaw_rot  = Quat::from_axis_angle(surface_up, yaw);
    let fwd      = yaw_rot * forward;
    let rgt      = yaw_rot * right;
    let rotation = Quat::from_mat3(&Mat3::from_cols(rgt, surface_up, fwd));

    let root = commands.spawn((
        TransformBundle::from_transform(
            Transform::from_translation(position).with_rotation(rotation),
        ),
        VisibilityBundle::default(),
        Structure,
        kind,
        Name::new(format!("{kind:?}")),
    )).id();

    match kind {
        StructureKind::Hut => build_hut(commands, meshes, materials, root, rng, false),
        StructureKind::SandstoneHut => build_hut(commands, meshes, materials, root, rng, true),
        StructureKind::WatchTower => build_watch_tower(commands, meshes, materials, root, rng),
        StructureKind::IceHut => build_ice_hut(commands, meshes, materials, root, rng),
        StructureKind::Ruin => build_ruin(commands, meshes, materials, root, rng),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Hut builder (stone or sandstone)
// ─────────────────────────────────────────────────────────────────────────────

fn build_hut(
    commands:    &mut Commands,
    meshes:      &mut Assets<Mesh>,
    materials:   &mut Assets<StandardMaterial>,
    root:        Entity,
    rng:         &mut impl Rng,
    sandstone:   bool,
) {
    let w: f32 = rng.gen_range(4.0f32..7.0f32);
    let d: f32 = rng.gen_range(4.0f32..7.0f32);
    let h: f32 = rng.gen_range(3.0f32..5.0f32);
    let wall_t: f32 = 0.6;

    let wall_color = if sandstone {
        Color::srgb(0.82, 0.72, 0.45)
    } else {
        Color::srgb(0.55, 0.52, 0.48)
    };
    let roof_color = if sandstone {
        Color::srgb(0.70, 0.60, 0.38)
    } else {
        Color::srgb(0.40, 0.38, 0.35)
    };

    let wall_mat = materials.add(StandardMaterial {
        base_color: wall_color, perceptual_roughness: 0.90, ..default()
    });
    let roof_mat = materials.add(StandardMaterial {
        base_color: roof_color, perceptual_roughness: 0.85, ..default()
    });

    // 4 walls (hollow box: front, back, left, right).
    let parts: &[(Vec3, Vec3)] = &[
        // front wall (Z-)
        (Vec3::new(0.0, h * 0.5, -d * 0.5 + wall_t * 0.5), Vec3::new(w, h, wall_t)),
        // back wall  (Z+)
        (Vec3::new(0.0, h * 0.5,  d * 0.5 - wall_t * 0.5), Vec3::new(w, h, wall_t)),
        // left wall  (X-)
        (Vec3::new(-w * 0.5 + wall_t * 0.5, h * 0.5, 0.0), Vec3::new(wall_t, h, d - 2.0 * wall_t)),
        // right wall (X+)
        (Vec3::new( w * 0.5 - wall_t * 0.5, h * 0.5, 0.0), Vec3::new(wall_t, h, d - 2.0 * wall_t)),
    ];

    for &(pos, size) in parts {
        let child = commands.spawn(PbrBundle {
            mesh:      meshes.add(Cuboid::new(size.x, size.y, size.z)),
            material:  wall_mat.clone(),
            transform: Transform::from_translation(pos),
            ..default()
        }).id();
        commands.entity(root).add_child(child);
    }

    // Flat roof slab.
    let roof = commands.spawn(PbrBundle {
        mesh:      meshes.add(Cuboid::new(w + wall_t, wall_t * 1.5, d + wall_t)),
        material:  roof_mat,
        transform: Transform::from_translation(Vec3::new(0.0, h + wall_t * 0.75, 0.0)),
        ..default()
    }).id();
    commands.entity(root).add_child(roof);

    // Door opening cut: a dark box slightly inset in the front wall.
    let door_h = h * 0.55;
    let door_w = 1.0f32.min(w * 0.25);
    let door_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.10, 0.08),
        unlit: true,
        ..default()
    });
    let door = commands.spawn(PbrBundle {
        mesh:      meshes.add(Cuboid::new(door_w, door_h, wall_t + 0.05)),
        material:  door_mat,
        transform: Transform::from_translation(Vec3::new(0.0, door_h * 0.5, -d * 0.5 + wall_t * 0.5)),
        ..default()
    }).id();
    commands.entity(root).add_child(door);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Watch tower builder
// ─────────────────────────────────────────────────────────────────────────────

fn build_watch_tower(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root:      Entity,
    rng:       &mut impl Rng,
) {
    let base_w: f32 = rng.gen_range(2.5f32..4.0f32);
    let height: f32 = rng.gen_range(8.0f32..14.0f32);
    let wall_t: f32 = 0.7;

    let stone_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.40, 0.38),
        perceptual_roughness: 0.92,
        ..default()
    });

    // Four vertical slabs forming a hollow square shaft.
    let inner = base_w - wall_t * 2.0;
    let shaft_parts: &[(Vec3, Vec3)] = &[
        (Vec3::new(0.0, height * 0.5, -(base_w - wall_t) * 0.5), Vec3::new(base_w, height, wall_t)),
        (Vec3::new(0.0, height * 0.5,  (base_w - wall_t) * 0.5), Vec3::new(base_w, height, wall_t)),
        (Vec3::new(-(base_w - wall_t) * 0.5, height * 0.5, 0.0), Vec3::new(wall_t, height, inner)),
        (Vec3::new( (base_w - wall_t) * 0.5, height * 0.5, 0.0), Vec3::new(wall_t, height, inner)),
    ];

    for &(pos, size) in shaft_parts {
        let child = commands.spawn(PbrBundle {
            mesh:      meshes.add(Cuboid::new(size.x, size.y, size.z)),
            material:  stone_mat.clone(),
            transform: Transform::from_translation(pos),
            ..default()
        }).id();
        commands.entity(root).add_child(child);
    }

    // Parapet (wider cap at the top).
    let cap = commands.spawn(PbrBundle {
        mesh:      meshes.add(Cuboid::new(base_w + 0.8, wall_t, base_w + 0.8)),
        material:  stone_mat.clone(),
        transform: Transform::from_translation(Vec3::new(0.0, height + wall_t * 0.5, 0.0)),
        ..default()
    }).id();
    commands.entity(root).add_child(cap);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Ice hut builder
// ─────────────────────────────────────────────────────────────────────────────

fn build_ice_hut(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root:      Entity,
    rng:       &mut impl Rng,
) {
    let r: f32 = rng.gen_range(2.5f32..4.5f32);

    let ice_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.90, 1.00, 0.80),
        perceptual_roughness: 0.15,
        metallic: 0.0,
        ..default()
    });

    // Dome: a half-sphere approximated with a UV sphere scaled on Y.
    let dome = commands.spawn(PbrBundle {
        mesh:      meshes.add(Sphere::new(r).mesh().uv(16, 10)),
        material:  ice_mat.clone(),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))
                     .with_scale(Vec3::new(1.0, 0.6, 1.0)),
        ..default()
    }).id();
    commands.entity(root).add_child(dome);

    // Entrance tunnel.
    let tunnel_d = r * 0.6;
    let tunnel_h = r * 0.45;
    let tunnel = commands.spawn(PbrBundle {
        mesh:      meshes.add(Cuboid::new(tunnel_h * 0.9, tunnel_h, tunnel_d)),
        material:  ice_mat,
        transform: Transform::from_translation(Vec3::new(0.0, tunnel_h * 0.5, -(r + tunnel_d * 0.5))),
        ..default()
    }).id();
    commands.entity(root).add_child(tunnel);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Ruin builder (partial walls)
// ─────────────────────────────────────────────────────────────────────────────

fn build_ruin(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root:      Entity,
    rng:       &mut impl Rng,
) {
    let w: f32 = rng.gen_range(4.0f32..9.0f32);
    let d: f32 = rng.gen_range(4.0f32..9.0f32);
    let wall_t: f32 = 0.7;

    let ruin_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.44, 0.40),
        perceptual_roughness: 0.95,
        ..default()
    });

    // Only 2–3 partial walls of random heights.
    let num_walls: usize = rng.gen_range(2..=3);
    let wall_defs: &[(Vec3, Vec3)] = &[
        (Vec3::new(0.0, 0.0, -d * 0.5 + wall_t * 0.5), Vec3::new(w, 0.0, wall_t)),
        (Vec3::new(0.0, 0.0,  d * 0.5 - wall_t * 0.5), Vec3::new(w, 0.0, wall_t)),
        (Vec3::new(-w * 0.5 + wall_t * 0.5, 0.0, 0.0), Vec3::new(wall_t, 0.0, d)),
        (Vec3::new( w * 0.5 - wall_t * 0.5, 0.0, 0.0), Vec3::new(wall_t, 0.0, d)),
    ];

    for i in 0..num_walls {
        let h: f32 = rng.gen_range(1.2f32..3.5f32);
        let (base_pos, base_size) = wall_defs[i];
        let pos  = base_pos + Vec3::new(0.0, h * 0.5, 0.0);
        let size = Vec3::new(base_size.x, h, base_size.z);

        let child = commands.spawn(PbrBundle {
            mesh:      meshes.add(Cuboid::new(size.x, size.y, size.z)),
            material:  ruin_mat.clone(),
            transform: Transform::from_translation(pos),
            ..default()
        }).id();
        commands.entity(root).add_child(child);
    }

    // Scattered rubble cubes.
    let rubble_count: usize = rng.gen_range(3..=8);
    for _ in 0..rubble_count {
        let rx = rng.gen_range(-w * 0.5..w * 0.5);
        let rz = rng.gen_range(-d * 0.5..d * 0.5);
        let rs = rng.gen_range(0.3f32..1.0f32);
        let rubble = commands.spawn(PbrBundle {
            mesh:      meshes.add(Cuboid::new(rs, rs * 0.5, rs)),
            material:  ruin_mat.clone(),
            transform: Transform::from_translation(Vec3::new(rx, rs * 0.25, rz)),
            ..default()
        }).id();
        commands.entity(root).add_child(rubble);
    }
}
