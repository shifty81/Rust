//! Simple wildlife AI — wandering animals spawned near the player based on biome.
//!
//! Each creature has a short-lived wander goal: every few seconds it picks a new
//! random heading and walks along the planet surface toward it.  Creatures are
//! despawned when the player moves far enough away, mirroring the vegetation
//! spawn/despawn pattern.

use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::biome::{classify_biome, Biome};
use crate::components::*;
use crate::config::*;
use crate::planet::terrain_radius_at;
use crate::vegetation::simple_moisture;

// ────────────────────────────────────────────────────────────────────────────

pub struct WildlifePlugin;

impl Plugin for WildlifePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_wildlife_around_player, update_creature_ai).chain());
    }
}

// ────────────────────────────────────────────────────────────────────────────
//  Components
// ────────────────────────────────────────────────────────────────────────────

/// Marks a creature entity.
#[derive(Component)]
pub struct Creature;

/// What kind of animal this is.
#[derive(Component, Clone, Copy, Debug)]
pub enum CreatureKind {
    /// Gentle deer — spawns in Forest and Plains biomes.
    Deer,
    /// Fast rabbit — spawns in most temperate biomes.
    Rabbit,
    /// Desert camel — spawns in Desert biomes.
    Camel,
    /// Polar bear — spawns in Arctic and Tundra biomes.
    PolarBear,
}

/// Per-creature wander state.
#[derive(Component)]
pub struct CreatureAI {
    /// Normalised world-space target direction (planet surface point).
    pub target_dir:   Vec3,
    /// Seconds until this creature picks a new wander target.
    pub wander_timer: f32,
    /// Walk speed on the surface (m/s).
    pub speed:        f32,
}

// ────────────────────────────────────────────────────────────────────────────
//  Spawn system
// ────────────────────────────────────────────────────────────────────────────

/// Small upward offset (metres) added to the terrain radius when placing a
/// creature so feet don't clip the surface.
const CREATURE_SURFACE_OFFSET: f32 = 0.02;
/// Maximum number of creatures allowed near the player at once.
const MAX_CREATURES: usize = 20;
/// Radius (m) within which creatures are kept alive.
const DESPAWN_RADIUS_FACTOR: f32 = 1.5;
/// Attempts per frame to spawn a new creature.
const SPAWN_ATTEMPTS_PER_FRAME: usize = 4;
/// Minimum distance from player at which creatures may spawn.
const MIN_SPAWN_DIST: f32 = 20.0;

pub fn spawn_wildlife_around_player(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query:  Query<&Transform, With<Player>>,
    creature_query: Query<(Entity, &Transform), (With<Creature>, Without<Player>)>,
    seed:          Res<NoiseSeed>,
    world_settings: Res<WorldSettings>,
) {
    let Ok(player_tf) = player_query.get_single() else { return };
    let player_pos = player_tf.translation;
    let local_up   = player_pos.normalize_or_zero();

    // Only spawn creatures on or near the surface.
    let altitude = player_pos.length() - PLANET_RADIUS;
    if altitude > ATMOSPHERE_FADE_START { return; }

    let spawn_r   = world_settings.vegetation_radius;
    let despawn_r = spawn_r * DESPAWN_RADIUS_FACTOR;

    // Despawn distant creatures.
    for (entity, tf) in &creature_query {
        if (tf.translation - player_pos).length() > despawn_r {
            commands.entity(entity).despawn_recursive();
        }
    }

    let existing = creature_query
        .iter()
        .filter(|(_, tf)| (tf.translation - player_pos).length() < spawn_r)
        .count();

    if existing >= MAX_CREATURES { return; }

    let mut rng = rand::thread_rng();
    let mut spawned_this_frame: usize = 0;

    let ref_right = if local_up.abs().dot(Vec3::X) < 0.9 {
        Vec3::X.cross(local_up).normalize()
    } else {
        Vec3::Z.cross(local_up).normalize()
    };
    let ref_fwd = local_up.cross(ref_right).normalize();

    for _ in 0..SPAWN_ATTEMPTS_PER_FRAME {
        if existing + spawned_this_frame >= MAX_CREATURES { break; }

        let angle  = rng.gen_range(0.0f32..2.0 * PI);
        let spread = rng.gen_range(MIN_SPAWN_DIST..spawn_r);

        let horiz    = ref_right * angle.cos() + ref_fwd * angle.sin();
        let cand_dir = (local_up + horiz * spread / PLANET_RADIUS).normalize();

        let surface_r = terrain_radius_at(cand_dir, seed.0);
        let altitude  = surface_r - PLANET_RADIUS;
        let latitude  = cand_dir.y;
        let moisture  = simple_moisture(cand_dir, seed.0);
        let biome     = classify_biome(latitude, altitude, moisture);

        let kind_opt = creature_for_biome(biome, &mut rng);
        let Some(kind) = kind_opt else { continue };

        let pos = cand_dir * (surface_r + CREATURE_SURFACE_OFFSET);
        spawn_creature(&mut commands, &mut meshes, &mut materials, pos, cand_dir, kind, &mut rng);
        spawned_this_frame += 1;
    }
}

/// Returns the creature kind suitable for the given biome, or `None` if no
/// creature is suitable (or the random roll says none spawns).
fn creature_for_biome(biome: Biome, rng: &mut impl Rng) -> Option<CreatureKind> {
    let roll: f32 = rng.gen_range(0.0..1.0);
    match biome {
        Biome::Forest | Biome::TropicalForest => {
            if roll < 0.012 { Some(CreatureKind::Deer) }
            else if roll < 0.020 { Some(CreatureKind::Rabbit) }
            else { None }
        }
        Biome::Plains | Biome::Savanna => {
            if roll < 0.010 { Some(CreatureKind::Deer) }
            else if roll < 0.018 { Some(CreatureKind::Rabbit) }
            else { None }
        }
        Biome::Desert => {
            if roll < 0.008 { Some(CreatureKind::Camel) } else { None }
        }
        Biome::Arctic | Biome::Tundra => {
            if roll < 0.006 { Some(CreatureKind::PolarBear) } else { None }
        }
        _ => None,
    }
}

fn spawn_creature(
    commands:   &mut Commands,
    meshes:     &mut Assets<Mesh>,
    materials:  &mut Assets<StandardMaterial>,
    position:   Vec3,
    surface_up: Vec3,
    kind:       CreatureKind,
    rng:        &mut impl Rng,
) {
    // Orient the creature so its local-up aligns with the surface normal.
    let ref_vec  = if surface_up.abs().dot(Vec3::X) < 0.9 { Vec3::X } else { Vec3::Z };
    let right    = surface_up.cross(ref_vec).normalize();
    let forward  = right.cross(surface_up).normalize();
    let rotation = Quat::from_mat3(&Mat3::from_cols(right, surface_up, forward));

    let (body_dims, head_r, body_color, head_color, speed_range, body_y, head_y) = match kind {
        CreatureKind::Deer => (
            Vec3::new(0.50, 0.90, 1.20),   // body w/h/d
            0.28,                           // head radius
            Color::srgb(0.62, 0.48, 0.28), // body colour
            Color::srgb(0.55, 0.42, 0.22), // head colour
            1.8f32..3.5f32,                 // speed (m/s)
            0.60,                           // body centre height
            1.15,                           // head centre height
        ),
        CreatureKind::Rabbit => (
            Vec3::new(0.20, 0.25, 0.30),
            0.14,
            Color::srgb(0.80, 0.76, 0.70),
            Color::srgb(0.82, 0.78, 0.72),
            1.2f32..2.2f32,
            0.15,
            0.38,
        ),
        CreatureKind::Camel => (
            Vec3::new(0.70, 1.10, 1.60),
            0.32,
            Color::srgb(0.80, 0.68, 0.42),
            Color::srgb(0.75, 0.62, 0.38),
            0.9f32..1.8f32,
            0.70,
            1.35,
        ),
        CreatureKind::PolarBear => (
            Vec3::new(0.80, 0.80, 1.40),
            0.36,
            Color::srgb(0.92, 0.92, 0.88),
            Color::srgb(0.90, 0.90, 0.86),
            0.8f32..1.6f32,
            0.55,
            1.10,
        ),
    };

    let speed = rng.gen_range(speed_range);
    let start_target = position.normalize();

    let body_mat = materials.add(StandardMaterial {
        base_color: body_color,
        perceptual_roughness: 0.85,
        ..default()
    });
    let head_mat = materials.add(StandardMaterial {
        base_color: head_color,
        perceptual_roughness: 0.85,
        ..default()
    });

    let body_mesh = meshes.add(Cuboid::new(body_dims.x, body_dims.y, body_dims.z));
    let head_mesh = meshes.add(Sphere::new(head_r).mesh().uv(8, 6));

    let root = commands
        .spawn((
            TransformBundle::from_transform(
                Transform::from_translation(position).with_rotation(rotation),
            ),
            VisibilityBundle::default(),
            Creature,
            kind,
            CreatureAI {
                target_dir:   start_target,
                wander_timer: rng.gen_range(2.0f32..6.0f32),
                speed,
            },
            Name::new(format!("{kind:?}")),
        ))
        .id();

    // Body
    commands
        .spawn(PbrBundle {
            mesh:      body_mesh,
            material:  body_mat,
            transform: Transform::from_translation(Vec3::new(0.0, body_y, 0.0)),
            ..default()
        })
        .set_parent(root);

    // Head
    commands
        .spawn(PbrBundle {
            mesh:      head_mesh,
            material:  head_mat,
            transform: Transform::from_translation(Vec3::new(0.0, head_y, body_dims.z * 0.4)),
            ..default()
        })
        .set_parent(root);
}

// ────────────────────────────────────────────────────────────────────────────
//  AI update system
// ────────────────────────────────────────────────────────────────────────────

pub fn update_creature_ai(
    time:    Res<Time>,
    seed:    Res<NoiseSeed>,
    mut q:   Query<(&mut Transform, &mut CreatureAI), With<Creature>>,
) {
    let dt  = time.delta_seconds();
    let mut rng = rand::thread_rng();

    for (mut tf, mut ai) in &mut q {
        // ── Wander timer: pick a new target direction periodically ────────────
        ai.wander_timer -= dt;
        if ai.wander_timer <= 0.0 {
            let local_up = tf.translation.normalize_or_zero();
            let ref_vec  = if local_up.abs().dot(Vec3::X) < 0.9 { Vec3::X } else { Vec3::Z };
            let right    = local_up.cross(ref_vec).normalize();
            let fwd      = local_up.cross(right).normalize();

            let angle  = rng.gen_range(0.0f32..2.0 * PI);
            let spread = rng.gen_range(5.0f32..30.0f32);
            let horiz  = right * angle.cos() + fwd * angle.sin();
            ai.target_dir   = (local_up + horiz * spread / PLANET_RADIUS).normalize();
            ai.wander_timer = rng.gen_range(3.0f32..8.0f32);
        }

        // ── Move toward target dir along the sphere surface ───────────────────
        let pos      = tf.translation;
        let dist     = pos.length();
        if dist < 1.0 { continue; }
        let local_up = pos / dist;

        // Project target_dir onto the tangent plane at current position.
        let to_target = ai.target_dir - local_up * ai.target_dir.dot(local_up);
        if to_target.length_squared() < 1e-6 { continue; }
        let move_dir = to_target.normalize();

        // Step along the tangent plane.
        let new_pos_flat = pos + move_dir * ai.speed * dt;

        // Snap back to terrain surface.
        let new_dir   = new_pos_flat.normalize();
        let surface_r = terrain_radius_at(new_dir, seed.0);
        tf.translation = new_dir * (surface_r + CREATURE_SURFACE_OFFSET);

        // ── Orient to surface + face movement direction ───────────────────────
        // right-hand rule: right = up × forward so local-X points right,
        // local-Y points away from planet, local-Z points in movement direction.
        let new_up  = tf.translation.normalize_or_zero();
        let fwd_vec = move_dir - new_up * move_dir.dot(new_up);
        if fwd_vec.length_squared() > 1e-6 {
            let fwd_n = fwd_vec.normalize();
            let right = new_up.cross(fwd_n).normalize();
            tf.rotation = Quat::from_mat3(&Mat3::from_cols(right, new_up, fwd_n));
        }
    }
}
