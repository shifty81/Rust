use bevy::input::mouse::MouseMotion;
use bevy::pbr::{FogFalloff, FogSettings};
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

use crate::components::*;
use crate::config::*;
use crate::planet::terrain_radius_at;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_player, setup_cursor).chain())
            .add_systems(
                Update,
                (
                    update_chunk_viewpoint_from_player,
                    handle_mouse_look,
                    handle_movement,
                    apply_gravity,
                    align_to_surface,
                    update_camera_pitch,
                    toggle_cursor,
                )
                    .chain(),
            );
    }
}

/// Spawn the player entity and its attached camera.  Call this from PIE start
/// or a Startup system.  For the editor, use this instead of `PlayerPlugin`
/// so that Startup is not run automatically at app launch.
pub fn spawn_voxel_player(commands: &mut Commands, seed: u32) {
    let surface_up = Vec3::Y;
    let surface_r  = terrain_radius_at(surface_up, seed);
    let spawn_pos  = surface_up * (surface_r + PLAYER_EYE_HEIGHT + SPAWN_HEIGHT);

    let player = commands
        .spawn((
            TransformBundle::from_transform(Transform::from_translation(spawn_pos)),
            VisibilityBundle::default(),
            Player,
            PlayerState::default(),
            Name::new("Player"),
        ))
        .id();

    commands
        .spawn((
            Camera3dBundle {
                transform: Transform::from_translation(Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0)),
                ..default()
            },
            FogSettings {
                color: Color::srgb(0.35, 0.48, 0.66),
                directional_light_color: Color::srgba(1.0, 0.95, 0.80, 0.5),
                directional_light_exponent: 30.0,
                falloff: FogFalloff::Linear {
                    start: FOG_START,
                    end:   FOG_END,
                },
            },
            PlayerCamera,
            Name::new("PlayerCamera"),
        ))
        .set_parent(player);
}

fn setup_player(mut commands: Commands, seed: Res<NoiseSeed>) {
    spawn_voxel_player(&mut commands, seed.0);
}

/// Keep the ChunkViewpoint resource in sync with the player's world position.
pub fn update_chunk_viewpoint_from_player(
    player_q:      Query<&Transform, With<Player>>,
    mut viewpoint: ResMut<ChunkViewpoint>,
) {
    if let Ok(tf) = player_q.get_single() {
        viewpoint.0 = tf.translation;
    }
}

fn setup_cursor(mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible   = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }
}

pub fn handle_mouse_look(
    mut motion_events: EventReader<MouseMotion>,
    mut player_query:  Query<&mut PlayerState, With<Player>>,
) {
    let Ok(mut state) = player_query.get_single_mut() else { return };

    for ev in motion_events.read() {
        state.yaw   -= ev.delta.x * MOUSE_SENSITIVITY;
        state.pitch -= ev.delta.y * MOUSE_SENSITIVITY;
        state.pitch  = state.pitch.clamp(-MAX_PITCH, MAX_PITCH);
    }
}

pub fn handle_movement(
    time:         Res<Time>,
    keyboard:     Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Transform, &mut PlayerState), With<Player>>,
) {
    let Ok((transform, mut state)) = player_q.get_single_mut() else { return };

    // ── Toggle flight mode (F key) ───────────────────────────────────────────
    if keyboard.just_pressed(KeyCode::KeyF) {
        state.is_flying = !state.is_flying;
        if !state.is_flying {
            // Cancel vertical velocity when landing back to walk mode.
            state.velocity = Vec3::ZERO;
            state.is_grounded = false;
        }
    }

    if state.is_flying {
        // ── Free-fly / space-flight mode ─────────────────────────────────────
        let speed = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            PLAYER_FLY_RUN_SPEED
        } else {
            PLAYER_FLY_SPEED
        };

        // Build look direction from player's own yaw + pitch.
        let look = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
        let fwd   = look * Vec3::NEG_Z;
        let right = look * Vec3::X;
        let up    = look * Vec3::Y;

        let mut move_dir = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp)    { move_dir += fwd;   }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown)  { move_dir -= fwd;   }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft)  { move_dir -= right; }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) { move_dir += right; }
        if keyboard.pressed(KeyCode::KeyE) { move_dir += up;  }
        if keyboard.pressed(KeyCode::KeyQ) { move_dir -= up;  }

        if move_dir.length_squared() > 0.0 {
            state.velocity = move_dir.normalize() * speed;
        } else {
            // Dampen residual velocity.
            state.velocity *= (1.0 - 5.0 * time.delta_seconds()).max(0.0);
        }

        return;
    }

    // ── Walking mode ─────────────────────────────────────────────────────────
    let speed = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        PLAYER_RUN_SPEED
    } else {
        PLAYER_WALK_SPEED
    };

    let local_up = transform.translation.normalize_or_zero();
    if local_up.length_squared() < 0.5 { return; }

    let ref_vec    = if local_up.abs().dot(Vec3::Y) > 0.9 { Vec3::X } else { Vec3::Y };
    let yaw_rot    = Quat::from_axis_angle(local_up, state.yaw);
    let base_fwd   = local_up.cross(ref_vec).normalize();
    let forward    = yaw_rot * base_fwd;
    let right      = forward.cross(local_up).normalize();

    let mut move_dir = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp)    { move_dir += forward; }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown)  { move_dir -= forward; }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft)  { move_dir -= right;   }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) { move_dir += right;   }

    if move_dir.length_squared() > 0.0 {
        let radial_vel = local_up * state.velocity.dot(local_up);
        state.velocity = radial_vel + move_dir.normalize() * speed;
    } else {
        let radial_vel = local_up * state.velocity.dot(local_up);
        let horiz_vel  = state.velocity - radial_vel;
        let dampen     = (1.0 - 10.0 * time.delta_seconds()).max(0.0);
        state.velocity = radial_vel + horiz_vel * dampen;
    }

    if keyboard.just_pressed(KeyCode::Space) && state.is_grounded {
        state.velocity += local_up * PLAYER_JUMP_SPEED;
        state.is_grounded = false;
    }
}

pub fn apply_gravity(
    time:         Res<Time>,
    mut player_q: Query<(&mut Transform, &mut PlayerState), With<Player>>,
    seed:         Res<NoiseSeed>,
) {
    let Ok((mut transform, mut state)) = player_q.get_single_mut() else { return };

    // No gravity in flight mode.
    if state.is_flying {
        transform.translation += state.velocity * time.delta_seconds();
        return;
    }

    let pos  = transform.translation;
    let dist = pos.length();
    if dist < 1.0 { return; }

    let local_up = pos / dist;

    if !state.is_grounded {
        state.velocity -= local_up * GRAVITY_STRENGTH * time.delta_seconds();
    }

    transform.translation += state.velocity * time.delta_seconds();

    let new_pos  = transform.translation;
    let new_dist = new_pos.length();
    if new_dist < 1.0 { return; }

    let new_up    = new_pos / new_dist;
    let terrain_r = terrain_radius_at(new_up, seed.0);
    let feet_r    = terrain_r + PLAYER_FOOT_CLEARANCE;

    if new_dist < feet_r {
        transform.translation = new_up * feet_r;
        let radial = new_up * state.velocity.dot(new_up);
        if radial.dot(new_up) < 0.0 {
            state.velocity -= radial;
        }
        state.is_grounded    = true;
        state.grounded_timer += time.delta_seconds();
    } else {
        state.is_grounded    = false;
        state.grounded_timer = 0.0;
    }
}

pub fn align_to_surface(
    mut player_q: Query<(&mut Transform, &PlayerState), With<Player>>,
) {
    let Ok((mut transform, state)) = player_q.get_single_mut() else { return };

    if state.is_flying {
        // In flight mode: rotate freely by world-space yaw only; pitch is applied
        // on the camera child so the player body just faces the flight direction.
        transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, 0.0, 0.0);
        return;
    }

    let local_up = transform.translation.normalize_or_zero();
    if local_up.length_squared() < 0.5 { return; }

    let ref_vec   = if local_up.abs().dot(Vec3::Y) > 0.9 { Vec3::X } else { Vec3::Y };
    let base_fwd  = local_up.cross(ref_vec).normalize();
    let yaw_rot   = Quat::from_axis_angle(local_up, state.yaw);
    let new_fwd   = yaw_rot * base_fwd;
    let new_right = new_fwd.cross(local_up).normalize();
    let corr_fwd  = local_up.cross(new_right).normalize();

    transform.rotation = Quat::from_mat3(&Mat3::from_cols(new_right, local_up, -corr_fwd));
}

pub fn update_camera_pitch(
    player_q: Query<&PlayerState, With<Player>>,
    mut cam_q: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    let Ok(state)      = player_q.get_single()   else { return };
    let Ok(mut cam_tf) = cam_q.get_single_mut()  else { return };

    cam_tf.rotation    = Quat::from_rotation_x(state.pitch);
    cam_tf.translation = Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
}

pub fn toggle_cursor(
    keyboard:     Res<ButtonInput<KeyCode>>,
    mut windows:  Query<&mut Window>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Ok(mut window) = windows.get_single_mut() {
            let locked = window.cursor.grab_mode == CursorGrabMode::Locked;
            window.cursor.grab_mode = if locked { CursorGrabMode::None } else { CursorGrabMode::Locked };
            window.cursor.visible   = locked;
        }
    }
}
