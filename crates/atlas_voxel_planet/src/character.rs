//! Third-person character model and camera mode toggle.
//!
//! # Controls
//! * **V** — toggle between First-Person and Third-Person camera.
//!
//! # Third-person model
//! The character body is a set of `PbrBundle` child entities parented under the
//! `Player` entity.  Parts are always present; their `Visibility` component is
//! flipped when the camera mode switches so they do not appear in first-person.
//!
//! ```text
//!          ┌─────┐   ← head (Sphere)
//!          │torso│   ← body (Cuboid)
//!         /│     │\
//!    arm L  └─────┘  arm R
//!          /       \
//!        leg L   leg R
//! ```
//!
//! # Animations (procedural, no asset required)
//! * **Leg / arm swing** – the legs oscillate ±25° and the arms mirror them,
//!   proportional to the player's horizontal speed.
//! * **Idle breathing** – a slow sinusoidal Y-scale on the torso when still.
//! * **Camera bob** – the camera Y offset oscillates slightly while walking.

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::components::*;
use crate::config::*;

// ─────────────────────────────────────────────────────────────────────────────

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .init_resource::<CharacterAnimPhase>()
            .add_systems(
                Update,
                (
                    spawn_character_body_once,
                    toggle_camera_mode,
                    animate_character_body,
                    update_camera_for_mode,
                )
                    .chain(),
            );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Resources
// ─────────────────────────────────────────────────────────────────────────────

/// Which perspective the player camera currently uses.
#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum CameraMode {
    #[default]
    FirstPerson,
    ThirdPerson,
}

/// Accumulated animation phase values, advanced each frame.
#[derive(Resource, Default)]
pub struct CharacterAnimPhase {
    /// Leg/arm swing phase (radians).
    pub stride: f32,
    /// Idle breathing phase (radians).
    pub breath: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Character body parts (components)
// ─────────────────────────────────────────────────────────────────────────────

/// Marks the root "body group" entity parented under `Player`.
/// Body-part entities are children of this root.
#[derive(Component)]
pub struct CharacterBodyRoot;

#[derive(Component)] pub struct CharacterTorso;
#[derive(Component)] pub struct CharacterHead;
#[derive(Component)] pub struct CharacterArmLeft;
#[derive(Component)] pub struct CharacterArmRight;
#[derive(Component)] pub struct CharacterLegLeft;
#[derive(Component)] pub struct CharacterLegRight;

// ─────────────────────────────────────────────────────────────────────────────
//  Constants
// ─────────────────────────────────────────────────────────────────────────────

/// How far behind the player the camera sits in third-person (local Z+, metres).
const TP_CAMERA_BACK: f32   = 3.5;
/// How far above the player origin the camera sits in third-person (metres).
const TP_CAMERA_UP:   f32   = 1.8;
/// Skin tone for the character model.
const SKIN_COLOR:  Color = Color::srgb(0.87, 0.72, 0.53);
/// Shirt / body colour.
const SHIRT_COLOR: Color = Color::srgb(0.25, 0.40, 0.72);
/// Trouser colour.
const PANT_COLOR:  Color = Color::srgb(0.15, 0.15, 0.40);
/// Max leg swing angle (radians).
const MAX_SWING: f32 = 0.44; // ≈ 25°
/// Walking phase advance rate multiplier (tunes how fast the legs swing).
const STRIDE_RATE: f32 = 3.5;
/// Breathing phase advance rate (radians/s).
const BREATH_RATE: f32 = 0.9;

// Body geometry offsets relative to the player root (Y=0 at feet).
const TORSO_H:   f32 = 0.65;
const TORSO_W:   f32 = 0.50;
const TORSO_D:   f32 = 0.30;
const LEG_H:     f32 = 0.70;
const LEG_W:     f32 = 0.20;
const ARM_H:     f32 = 0.55;
const ARM_W:     f32 = 0.16;
const HEAD_R:    f32 = 0.20;

/// Y of torso centre above player root.
const TORSO_Y:  f32 = LEG_H + TORSO_H * 0.5;
/// Y of leg pivot (top of leg, hip joint) above root.
const LEG_Y:    f32 = LEG_H;
/// Y of arm pivot (shoulder) above root.
const ARM_Y:    f32 = LEG_H + TORSO_H - 0.05;
/// Y of head centre above root.
const HEAD_Y:   f32 = LEG_H + TORSO_H + HEAD_R + 0.02;

// ─────────────────────────────────────────────────────────────────────────────
//  Body-spawn system (one-shot: only runs until body exists)
// ─────────────────────────────────────────────────────────────────────────────

fn spawn_character_body_once(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q:      Query<Entity, (With<Player>, Without<CharacterBodyRoot>)>,
    body_q:        Query<(), With<CharacterBodyRoot>>,
) {
    if !body_q.is_empty() { return; }
    let Ok(player_entity) = player_q.get_single() else { return };

    let skin  = materials.add(StandardMaterial { base_color: SKIN_COLOR,  perceptual_roughness: 0.80, ..default() });
    let shirt = materials.add(StandardMaterial { base_color: SHIRT_COLOR, perceptual_roughness: 0.85, ..default() });
    let pants = materials.add(StandardMaterial { base_color: PANT_COLOR,  perceptual_roughness: 0.85, ..default() });

    // Root group — invisible in first-person.
    let body_root = commands.spawn((
        TransformBundle::default(),
        VisibilityBundle { visibility: Visibility::Hidden, ..default() },
        CharacterBodyRoot,
        Name::new("CharacterBody"),
    )).id();

    // Torso.
    let torso = commands.spawn((
        PbrBundle {
            mesh:     meshes.add(Cuboid::new(TORSO_W, TORSO_H, TORSO_D)),
            material: shirt.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, TORSO_Y, 0.0)),
            ..default()
        },
        CharacterTorso,
    )).id();

    // Head.
    let head = commands.spawn((
        PbrBundle {
            mesh:     meshes.add(Sphere::new(HEAD_R).mesh().uv(10, 8)),
            material: skin.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, HEAD_Y, 0.0)),
            ..default()
        },
        CharacterHead,
    )).id();

    // Arms: pivot at shoulder, mesh centre offset half-way down.
    let arm_offset = Vec3::new(0.0, -ARM_H * 0.5, 0.0);
    let arm_l = commands.spawn((
        PbrBundle {
            mesh:      meshes.add(Cuboid::new(ARM_W, ARM_H, ARM_W)),
            material:  shirt.clone(),
            transform: Transform::from_translation(
                Vec3::new(-(TORSO_W * 0.5 + ARM_W * 0.5 + 0.02), ARM_Y, 0.0)
            ),
            ..default()
        },
        CharacterArmLeft,
    )).id();
    let arm_r = commands.spawn((
        PbrBundle {
            mesh:      meshes.add(Cuboid::new(ARM_W, ARM_H, ARM_W)),
            material:  shirt,
            transform: Transform::from_translation(
                Vec3::new(TORSO_W * 0.5 + ARM_W * 0.5 + 0.02, ARM_Y, 0.0)
            ),
            ..default()
        },
        CharacterArmRight,
    )).id();

    // Legs: pivot at hip, mesh centre offset half-way down.
    let leg_offset = Vec3::new(0.0, -LEG_H * 0.5, 0.0);
    let leg_l = commands.spawn((
        PbrBundle {
            mesh:      meshes.add(Cuboid::new(LEG_W, LEG_H, LEG_W)),
            material:  pants.clone(),
            transform: Transform::from_translation(
                Vec3::new(-(LEG_W * 0.5 + 0.01), LEG_H * 0.5, 0.0)
            ),
            ..default()
        },
        CharacterLegLeft,
    )).id();
    let leg_r = commands.spawn((
        PbrBundle {
            mesh:      meshes.add(Cuboid::new(LEG_W, LEG_H, LEG_W)),
            material:  pants,
            transform: Transform::from_translation(
                Vec3::new(LEG_W * 0.5 + 0.01, LEG_H * 0.5, 0.0)
            ),
            ..default()
        },
        CharacterLegRight,
    )).id();

    commands.entity(body_root).push_children(&[torso, head, arm_l, arm_r, leg_l, leg_r]);
    commands.entity(player_entity).add_child(body_root);
    // Tag player so this system does not re-run.
    commands.entity(player_entity).insert(CharacterBodyRoot);

    // suppress the unused variable warnings for offset helpers
    let _ = arm_offset;
    let _ = leg_offset;
}

// ─────────────────────────────────────────────────────────────────────────────
//  Camera mode toggle
// ─────────────────────────────────────────────────────────────────────────────

fn toggle_camera_mode(
    keyboard:      Res<ButtonInput<KeyCode>>,
    mut mode:      ResMut<CameraMode>,
    mut body_q:    Query<&mut Visibility, With<CharacterBodyRoot>>,
) {
    if keyboard.just_pressed(KeyCode::KeyV) {
        *mode = match *mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            CameraMode::ThirdPerson => CameraMode::FirstPerson,
        };
        let target_vis = if *mode == CameraMode::ThirdPerson {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for mut vis in &mut body_q {
            *vis = target_vis;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Procedural animation
// ─────────────────────────────────────────────────────────────────────────────

pub fn animate_character_body(
    time:         Res<Time>,
    mode:         Res<CameraMode>,
    player_q:     Query<&PlayerState, With<Player>>,
    mut phase:    ResMut<CharacterAnimPhase>,
    mut torso_q:  Query<&mut Transform, (With<CharacterTorso>,
                    Without<CharacterArmLeft>, Without<CharacterArmRight>,
                    Without<CharacterLegLeft>, Without<CharacterLegRight>,
                    Without<CharacterHead>)>,
    mut arm_l_q:  Query<&mut Transform, (With<CharacterArmLeft>,
                    Without<CharacterTorso>, Without<CharacterArmRight>,
                    Without<CharacterLegLeft>, Without<CharacterLegRight>,
                    Without<CharacterHead>)>,
    mut arm_r_q:  Query<&mut Transform, (With<CharacterArmRight>,
                    Without<CharacterTorso>, Without<CharacterArmLeft>,
                    Without<CharacterLegLeft>, Without<CharacterLegRight>,
                    Without<CharacterHead>)>,
    mut leg_l_q:  Query<&mut Transform, (With<CharacterLegLeft>,
                    Without<CharacterTorso>, Without<CharacterArmLeft>,
                    Without<CharacterArmRight>, Without<CharacterLegRight>,
                    Without<CharacterHead>)>,
    mut leg_r_q:  Query<&mut Transform, (With<CharacterLegRight>,
                    Without<CharacterTorso>, Without<CharacterArmLeft>,
                    Without<CharacterArmRight>, Without<CharacterLegLeft>,
                    Without<CharacterHead>)>,
) {
    if *mode == CameraMode::FirstPerson { return; }
    let Ok(state) = player_q.get_single() else { return };

    let dt = time.delta_seconds();

    // Horizontal speed proxy (use velocity projected away from radial direction).
    let horiz_speed = state.velocity.length();

    // Advance phases.
    if horiz_speed > 0.2 {
        phase.stride += dt * STRIDE_RATE * (horiz_speed / PLAYER_WALK_SPEED).min(2.0);
    } else {
        // Damp stride back to zero.
        let target = (phase.stride / (2.0 * PI)).round() * 2.0 * PI;
        phase.stride += (target - phase.stride) * (dt * 8.0).min(1.0);
    }
    phase.breath += dt * BREATH_RATE;

    let swing = if horiz_speed > 0.2 {
        phase.stride.sin() * MAX_SWING * (horiz_speed / PLAYER_WALK_SPEED).min(1.0).sqrt()
    } else {
        0.0
    };

    let breath_scale = 1.0 + phase.breath.sin() * 0.015;

    // Torso: breathing scale on Y.
    if let Ok(mut tf) = torso_q.get_single_mut() {
        tf.scale.y = breath_scale;
        tf.translation.y = TORSO_Y;
    }

    // Left leg swings forward, right leg backward.
    if let Ok(mut tf) = leg_l_q.get_single_mut() {
        let pivot = Vec3::new(-(LEG_W * 0.5 + 0.01), LEG_Y, 0.0);
        tf.translation = pivot + Quat::from_rotation_x(swing) * Vec3::new(0.0, -LEG_H * 0.5, 0.0);
        tf.rotation    = Quat::from_rotation_x(swing);
    }
    if let Ok(mut tf) = leg_r_q.get_single_mut() {
        let pivot = Vec3::new(LEG_W * 0.5 + 0.01, LEG_Y, 0.0);
        tf.translation = pivot + Quat::from_rotation_x(-swing) * Vec3::new(0.0, -LEG_H * 0.5, 0.0);
        tf.rotation    = Quat::from_rotation_x(-swing);
    }

    // Arms swing opposite to legs.
    if let Ok(mut tf) = arm_l_q.get_single_mut() {
        let shoulder = Vec3::new(-(TORSO_W * 0.5 + ARM_W * 0.5 + 0.02), ARM_Y, 0.0);
        tf.translation = shoulder + Quat::from_rotation_x(-swing * 0.7) * Vec3::new(0.0, -ARM_H * 0.5, 0.0);
        tf.rotation    = Quat::from_rotation_x(-swing * 0.7);
    }
    if let Ok(mut tf) = arm_r_q.get_single_mut() {
        let shoulder = Vec3::new(TORSO_W * 0.5 + ARM_W * 0.5 + 0.02, ARM_Y, 0.0);
        tf.translation = shoulder + Quat::from_rotation_x(swing * 0.7) * Vec3::new(0.0, -ARM_H * 0.5, 0.0);
        tf.rotation    = Quat::from_rotation_x(swing * 0.7);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Camera position
// ─────────────────────────────────────────────────────────────────────────────

pub fn update_camera_for_mode(
    mode:     Res<CameraMode>,
    state_q:  Query<&PlayerState, With<Player>>,
    mut cam_q: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    let Ok(state)       = state_q.get_single()  else { return };
    let Ok(mut cam_tf)  = cam_q.get_single_mut() else { return };

    match *mode {
        CameraMode::FirstPerson => {
            cam_tf.translation = Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
            cam_tf.rotation    = Quat::from_rotation_x(state.pitch);
        }
        CameraMode::ThirdPerson => {
            // Position camera behind and above the player in local space.
            // Local Z+ = behind the player (player faces -Z in local space).
            let local_back = Quat::from_rotation_y(0.0) * Vec3::Z;
            cam_tf.translation = Vec3::new(0.0, TP_CAMERA_UP, TP_CAMERA_BACK);
            // Look toward the player's head.
            let look_target = Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
            let dir = (look_target - cam_tf.translation).normalize();
            cam_tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, dir)
                * Quat::from_rotation_x(state.pitch * 0.3);
            let _ = local_back;
        }
    }
}
