use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
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

fn setup_player(
    mut commands: Commands,
    seed: Res<NoiseSeed>,
) {
    let surface_up = Vec3::Y;
    let surface_r = terrain_radius_at(surface_up, seed.0);
    let spawn_pos = surface_up * (surface_r + PLAYER_EYE_HEIGHT + SPAWN_HEIGHT);

    let player = commands
        .spawn((
            TransformBundle::from_transform(Transform::from_translation(spawn_pos)),
            VisibilityBundle::default(),
            Player,
            PlayerState::default(),
            Name::new("Player"),
        ))
        .id();

    // Camera as a child of the player at eye height.
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0)),
            ..default()
        },
        PlayerCamera,
        Name::new("PlayerCamera"),
    )).set_parent(player);
}

fn setup_cursor(mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }
}

fn handle_mouse_look(
    mut motion_events: EventReader<MouseMotion>,
    mut player_query: Query<&mut PlayerState, With<Player>>,
) {
    let Ok(mut state) = player_query.get_single_mut() else { return };

    for ev in motion_events.read() {
        state.yaw   -= ev.delta.x * MOUSE_SENSITIVITY;
        state.pitch -= ev.delta.y * MOUSE_SENSITIVITY;
        state.pitch  = state.pitch.clamp(-MAX_PITCH, MAX_PITCH);
    }
}

fn handle_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut Transform, &mut PlayerState), With<Player>>,
) {
    let Ok((transform, mut state)) = player_query.get_single_mut() else { return };

    let speed = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        PLAYER_RUN_SPEED
    } else {
        PLAYER_WALK_SPEED
    };

    let local_up = transform.translation.normalize_or_zero();
    if local_up.length_squared() < 0.5 { return; }

    let ref_vec = if local_up.abs().dot(Vec3::Y) > 0.9 { Vec3::X } else { Vec3::Y };
    let yaw_rot = Quat::from_axis_angle(local_up, state.yaw);
    let base_forward = local_up.cross(ref_vec).normalize();
    let forward = yaw_rot * base_forward;
    let right   = forward.cross(local_up).normalize();

    let mut move_dir = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp)    { move_dir += forward; }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown)  { move_dir -= forward; }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft)  { move_dir -= right;   }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) { move_dir += right;   }

    if move_dir.length_squared() > 0.0 {
        let radial_vel = local_up * state.velocity.dot(local_up);
        state.velocity = radial_vel + move_dir.normalize() * speed;
    } else {
        let radial_vel  = local_up * state.velocity.dot(local_up);
        let horiz_vel   = state.velocity - radial_vel;
        let dampen = (1.0 - 10.0 * time.delta_seconds()).max(0.0);
        state.velocity  = radial_vel + horiz_vel * dampen;
    }

    if keyboard.just_pressed(KeyCode::Space) && state.is_grounded {
        state.velocity += local_up * PLAYER_JUMP_SPEED;
        state.is_grounded = false;
    }
}

fn apply_gravity(
    time: Res<Time>,
    mut player_query: Query<(&mut Transform, &mut PlayerState), With<Player>>,
    seed: Res<NoiseSeed>,
) {
    let Ok((mut transform, mut state)) = player_query.get_single_mut() else { return };

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

    let new_up   = new_pos / new_dist;
    let terrain_r = terrain_radius_at(new_up, seed.0);
    let feet_r    = terrain_r + 0.05;

    if new_dist < feet_r {
        transform.translation = new_up * feet_r;
        let radial = new_up * state.velocity.dot(new_up);
        if radial.dot(new_up) < 0.0 {
            state.velocity -= radial;
        }
        state.is_grounded      = true;
        state.grounded_timer  += time.delta_seconds();
    } else {
        state.is_grounded    = false;
        state.grounded_timer = 0.0;
    }
}

fn align_to_surface(
    mut player_query: Query<(&mut Transform, &PlayerState), With<Player>>,
) {
    let Ok((mut transform, state)) = player_query.get_single_mut() else { return };

    let local_up = transform.translation.normalize_or_zero();
    if local_up.length_squared() < 0.5 { return; }

    let ref_vec = if local_up.abs().dot(Vec3::Y) > 0.9 { Vec3::X } else { Vec3::Y };
    let base_forward  = local_up.cross(ref_vec).normalize();
    let yaw_rot       = Quat::from_axis_angle(local_up, state.yaw);
    let new_forward   = yaw_rot * base_forward;
    let new_right     = new_forward.cross(local_up).normalize();
    let corr_forward  = local_up.cross(new_right).normalize();

    transform.rotation =
        Quat::from_mat3(&Mat3::from_cols(new_right, local_up, -corr_forward));
}

fn update_camera_pitch(
    player_query: Query<&PlayerState, With<Player>>,
    mut cam_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    let Ok(state)      = player_query.get_single()    else { return };
    let Ok(mut cam_tf) = cam_query.get_single_mut()   else { return };

    cam_tf.rotation    = Quat::from_rotation_x(state.pitch);
    cam_tf.translation = Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
}

fn toggle_cursor(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Ok(mut window) = windows.get_single_mut() {
            let locked = window.cursor.grab_mode == CursorGrabMode::Locked;
            window.cursor.grab_mode = if locked { CursorGrabMode::None } else { CursorGrabMode::Locked };
            window.cursor.visible   = locked;
        }
    }
}
