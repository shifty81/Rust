use bevy::prelude::*;
use std::f32::consts::PI;

use crate::components::*;
use crate::config::*;

pub struct SolarSystemPlugin;

impl Plugin for SolarSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_solar_system)
            .add_systems(
                Update,
                (update_orbits, update_self_rotations, update_sun_light).chain(),
            );
    }
}

fn setup_solar_system(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sun_orbit_axis = Vec3::new(0.0, AXIAL_TILT.cos(), AXIAL_TILT.sin()).normalize();

    // Sun
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(SUN_RADIUS).mesh().uv(32, 18)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.95, 0.70),
                emissive: LinearRgba::new(25.0, 20.0, 4.0, 1.0),
                unlit: true,
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(SUN_DISTANCE, 0.0, 0.0)),
            ..default()
        },
        Sun,
        OrbitalBody::new(SUN_DISTANCE, DAY_LENGTH_SECONDS, Vec3::ZERO)
            .with_axis(sun_orbit_axis),
        SelfRotation { axis: Vec3::Y, angular_speed: 0.04 },
        Name::new("Sun"),
    ));

    // Directional sunlight
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance:      100_000.0,
                shadows_enabled:  true,
                color:            Color::srgb(1.0, 0.96, 0.88),
                ..default()
            },
            transform: Transform::from_xyz(SUN_DISTANCE, 0.0, 0.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        SunLight,
        Name::new("SunLight"),
    ));

    // Moon
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(MOON_RADIUS).mesh().uv(16, 10)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.68, 0.66, 0.62),
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(MOON_DISTANCE, 0.0, 0.0)),
            ..default()
        },
        Moon,
        OrbitalBody::new(MOON_DISTANCE, MOON_ORBIT_PERIOD, Vec3::ZERO)
            .with_axis(Vec3::new(0.0, 1.0, 0.09).normalize()),
        SelfRotation { axis: Vec3::Y, angular_speed: 0.02 },
        Name::new("Moon"),
    ));

    // Other planets
    let planet_defs: &[(&str, f32, f32, Color, f32, f32)] = &[
        ("Mercury", P2_ORBIT,  1_800.0,  Color::srgb(0.60, 0.56, 0.50),  350.0, 0.0),
        ("Venus",   P3_ORBIT,  3_200.0,  Color::srgb(0.90, 0.83, 0.45),  900.0, 0.6),
        ("Mars",    P4_ORBIT,  2_600.0,  Color::srgb(0.78, 0.38, 0.18),  480.0, 1.2),
        ("Jupiter", P5_ORBIT, 18_000.0,  Color::srgb(0.80, 0.70, 0.55), 1_200.0, 2.0),
        ("Saturn",  P6_ORBIT, 15_000.0,  Color::srgb(0.90, 0.84, 0.58), 2_400.0, 3.0),
        ("Uranus",  P7_ORBIT,  8_000.0,  Color::srgb(0.55, 0.84, 0.94), 4_800.0, 4.5),
        ("Neptune", P8_ORBIT,  7_800.0,  Color::srgb(0.28, 0.38, 0.90), 9_600.0, 5.8),
    ];

    for (name, orbit_radius, visual_radius, color, period, initial_angle) in planet_defs {
        let angle = *initial_angle;
        let pos   = Vec3::new(orbit_radius * angle.cos(), 0.0, orbit_radius * angle.sin());
        commands.spawn((
            PbrBundle {
                mesh:     meshes.add(Sphere::new(*visual_radius).mesh().uv(16, 10)),
                material: materials.add(StandardMaterial { base_color: *color, ..default() }),
                transform: Transform::from_translation(pos),
                ..default()
            },
            OrbitalBody {
                orbit_radius: *orbit_radius,
                orbit_period: *period,
                orbit_angle:  angle,
                orbit_center: Vec3::ZERO,
                orbit_axis:   Vec3::Y,
            },
            SelfRotation { axis: Vec3::Y, angular_speed: 0.06 },
            Name::new(*name),
        ));
    }

    commands.insert_resource(AmbientLight {
        color:      Color::srgb(0.05, 0.05, 0.12),
        brightness: 40.0,
    });
}

fn update_orbits(time: Res<Time>, mut query: Query<(&mut Transform, &mut OrbitalBody)>) {
    let dt = time.delta_seconds();
    for (mut transform, mut body) in &mut query {
        body.orbit_angle += 2.0 * PI * dt / body.orbit_period;
        body.orbit_angle %= 2.0 * PI;
        let rot    = Quat::from_axis_angle(body.orbit_axis, body.orbit_angle);
        let offset = Vec3::new(body.orbit_radius, 0.0, 0.0);
        transform.translation = body.orbit_center + rot * offset;
    }
}

fn update_self_rotations(time: Res<Time>, mut query: Query<(&mut Transform, &SelfRotation)>) {
    let dt = time.delta_seconds();
    for (mut transform, rot) in &mut query {
        transform.rotate(Quat::from_axis_angle(rot.axis, rot.angular_speed * dt));
    }
}

fn update_sun_light(
    sun_query:   Query<&Transform, (With<Sun>, Without<SunLight>)>,
    mut light_q: Query<&mut Transform, With<SunLight>>,
) {
    if let (Ok(sun_tf), Ok(mut light_tf)) =
        (sun_query.get_single(), light_q.get_single_mut())
    {
        let to_planet = -sun_tf.translation.normalize();
        if to_planet.length_squared() > 0.0 {
            light_tf.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, to_planet);
        }
    }
}
