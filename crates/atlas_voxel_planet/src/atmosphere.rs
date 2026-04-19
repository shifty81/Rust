use bevy::pbr::{FogFalloff, FogSettings};
use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::components::*;
use crate::config::*;

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherState>()
            .init_resource::<WorldTime>()
            .add_systems(Startup, setup_sky)
            .add_systems(
                Update,
                (
                    update_world_time,
                    update_sky_color,
                    update_fog,
                    update_weather,
                    animate_weather_particles,
                )
                    .chain(),
            );
    }
}

fn setup_sky(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.4, 0.65, 0.9)));
}

fn update_world_time(time: Res<Time>, mut world_time: ResMut<WorldTime>) {
    world_time.day_fraction += time.delta_seconds() / DAY_LENGTH_SECONDS;
    if world_time.day_fraction >= 1.0 {
        world_time.day_fraction -= 1.0;
        world_time.total_days   += 1.0;
    }
}

fn update_sky_color(
    world_time:       Res<WorldTime>,
    mut clear_color:  ResMut<ClearColor>,
    mut sun_light_q:  Query<&mut DirectionalLight, With<SunLight>>,
    mut ambient:      ResMut<AmbientLight>,
    player_q:         Query<&Transform, With<Player>>,
) {
    let t         = world_time.day_fraction;
    let sun_angle = (t - 0.25) * 2.0 * PI;
    let elevation = sun_angle.sin();

    // ── Altitude-based sky colour ────────────────────────────────────────────
    let altitude = if let Ok(player_tf) = player_q.get_single() {
        (player_tf.translation.length() - PLANET_RADIUS).max(0.0)
    } else {
        0.0
    };

    if altitude >= ATMOSPHERE_HEIGHT {
        // Space — pure black sky.
        clear_color.0 = Color::linear_rgb(0.0, 0.0, 0.0);
        if let Ok(mut sun_light) = sun_light_q.get_single_mut() {
            // Sun still illuminates objects in space.
            sun_light.illuminance = 80_000.0;
            sun_light.color = Color::WHITE;
        }
        ambient.brightness = 5.0; // very dim in space
        return;
    }

    let sky_base = sky_gradient(elevation);

    if altitude > ATMOSPHERE_FADE_START {
        // Fade sky colour toward black approaching space.
        let t_fade = (altitude - ATMOSPHERE_FADE_START)
            / (ATMOSPHERE_HEIGHT - ATMOSPHERE_FADE_START);
        let LinearRgba { red, green, blue, .. } = sky_base.to_linear();
        let fade = 1.0 - t_fade * t_fade;
        clear_color.0 = Color::linear_rgb(red * fade, green * fade, blue * fade);
    } else {
        clear_color.0 = sky_base;
    }

    if let Ok(mut sun_light) = sun_light_q.get_single_mut() {
        let intensity  = (elevation * 1.5).clamp(0.0, 1.0);
        sun_light.illuminance = intensity * 100_000.0;

        let warmth = (1.0 - (elevation.abs() - 0.5).abs() * 2.0).clamp(0.0, 1.0);
        sun_light.color = Color::srgb(1.0, 1.0 - warmth * 0.22, 1.0 - warmth * 0.45);
    }

    let amb = (elevation * 0.8 + 0.15).clamp(0.05, 0.8);
    ambient.brightness = amb * 300.0;
}

fn sky_gradient(elevation: f32) -> Color {
    if elevation < -0.1 {
        return Color::srgb(0.01, 0.01, 0.08);
    }
    if elevation < 0.05 {
        let k = (elevation + 0.1) / 0.15;
        return lerp_color(Color::srgb(0.01, 0.01, 0.08), Color::srgb(0.85, 0.40, 0.10), k);
    }
    if elevation < 0.20 {
        let k = (elevation - 0.05) / 0.15;
        return lerp_color(Color::srgb(0.85, 0.40, 0.10), Color::srgb(0.55, 0.75, 0.95), k);
    }
    let k = ((elevation - 0.20) / 0.80).clamp(0.0, 1.0);
    lerp_color(Color::srgb(0.55, 0.75, 0.95), Color::srgb(0.30, 0.55, 0.90), k)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let LinearRgba { red: ar, green: ag, blue: ab, alpha: aa } = a.to_linear();
    let LinearRgba { red: br, green: bg, blue: bb, alpha: ba } = b.to_linear();
    Color::linear_rgba(
        ar + (br - ar) * t,
        ag + (bg - ag) * t,
        ab + (bb - ab) * t,
        aa + (ba - aa) * t,
    )
}

fn update_fog(
    world_time: Res<WorldTime>,
    weather:    Res<WeatherState>,
    mut fog_q:  Query<&mut FogSettings, With<PlayerCamera>>,
    player_q:   Query<&Transform, With<Player>>,
) {
    let Ok(mut fog) = fog_q.get_single_mut() else { return };

    let sun_angle = (world_time.day_fraction - 0.25) * 2.0 * PI;
    let elevation = sun_angle.sin();

    let sky = sky_gradient(elevation);
    let LinearRgba { red, green, blue, .. } = sky.to_linear();
    fog.color = Color::linear_rgb(red, green, blue);

    let (fog_start, fog_end) = match weather.kind {
        WeatherKind::Storm  => (FOG_START * 0.25, FOG_END * 0.35),
        WeatherKind::Rain   => (FOG_START * 0.50, FOG_END * 0.60),
        WeatherKind::Snow   => (FOG_START * 0.55, FOG_END * 0.65),
        WeatherKind::Cloudy => (FOG_START * 0.80, FOG_END * 0.85),
        WeatherKind::Clear  => (FOG_START,          FOG_END),
    };

    // ── Altitude-based atmosphere fade ──────────────────────────────────────
    // Above `ATMOSPHERE_FADE_START`: progressively push the fog out; fog colour
    // blends toward black (space).  Above `ATMOSPHERE_HEIGHT`: no fog at all.
    let altitude = if let Ok(player_tf) = player_q.get_single() {
        (player_tf.translation.length() - PLANET_RADIUS).max(0.0)
    } else {
        0.0
    };

    if altitude >= ATMOSPHERE_HEIGHT {
        // Space: effectively no fog — push it beyond any visible geometry.
        fog.falloff = FogFalloff::Linear {
            start: PLANET_RADIUS * 10.0,
            end:   PLANET_RADIUS * 20.0,
        };
        fog.color = Color::linear_rgb(0.0, 0.0, 0.0);
    } else if altitude > ATMOSPHERE_FADE_START {
        let t = (altitude - ATMOSPHERE_FADE_START)
            / (ATMOSPHERE_HEIGHT - ATMOSPHERE_FADE_START);
        // Exponentially push fog distance out.
        let exp_t = t * t;
        let s = fog_start  + (PLANET_RADIUS * 5.0 - fog_start)  * exp_t;
        let e = fog_end    + (PLANET_RADIUS * 10.0 - fog_end)   * exp_t;
        fog.falloff = FogFalloff::Linear { start: s, end: e };

        // Blend sky colour toward black as we approach space.
        let LinearRgba { red: fr, green: fg, blue: fb, .. } = fog.color.to_linear();
        fog.color = Color::linear_rgb(
            fr * (1.0 - exp_t),
            fg * (1.0 - exp_t),
            fb * (1.0 - exp_t),
        );
    } else {
        fog.falloff = FogFalloff::Linear { start: fog_start, end: fog_end };
    }
}

fn update_weather(
    time:          Res<Time>,
    mut weather:   ResMut<WeatherState>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query:  Query<&Transform, With<Player>>,
    particles:     Query<Entity, With<WeatherParticle>>,
) {
    weather.change_timer -= time.delta_seconds();

    if weather.change_timer <= 0.0 {
        let mut rng = rand::thread_rng();
        weather.kind = match rng.gen_range(0u32..5u32) {
            0 => WeatherKind::Clear,
            1 => WeatherKind::Cloudy,
            2 => WeatherKind::Rain,
            3 => WeatherKind::Snow,
            _ => WeatherKind::Storm,
        };
        weather.intensity    = rng.gen_range(0.3f32..1.0f32);
        weather.change_timer = WEATHER_CHANGE_INTERVAL;

        for e in particles.iter() {
            commands.entity(e).despawn();
        }

        if matches!(weather.kind, WeatherKind::Rain | WeatherKind::Snow | WeatherKind::Storm) {
            if let Ok(player_tf) = player_query.get_single() {
                let count = (MAX_WEATHER_PARTICLES as f32 * weather.intensity) as usize;
                spawn_precipitation(
                    &mut commands, &mut meshes, &mut materials,
                    player_tf.translation, &weather.kind, count,
                );
            }
        }
    }
}

fn spawn_precipitation(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center:    Vec3,
    kind:      &WeatherKind,
    count:     usize,
) {
    let mut rng = rand::thread_rng();

    let (color, radius) = match kind {
        WeatherKind::Rain | WeatherKind::Storm => (Color::srgba(0.4, 0.5, 0.9, 0.7), 0.04_f32),
        WeatherKind::Snow                      => (Color::srgba(0.95, 0.97, 1.0, 0.85), 0.08_f32),
        _ => return,
    };

    let mesh_handle = meshes.add(Sphere::new(radius).mesh().uv(4, 4));
    let mat_handle  = materials.add(StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    for _ in 0..count {
        let offset = Vec3::new(
            rng.gen_range(-50.0f32..50.0),
            rng.gen_range(5.0f32..30.0),
            rng.gen_range(-50.0f32..50.0),
        );
        commands.spawn((
            PbrBundle {
                mesh:      mesh_handle.clone(),
                material:  mat_handle.clone(),
                transform: Transform::from_translation(center + offset),
                ..default()
            },
            WeatherParticle,
        ));
    }
}

fn animate_weather_particles(
    time:          Res<Time>,
    weather:       Res<WeatherState>,
    mut part_q:    Query<&mut Transform, With<WeatherParticle>>,
    player_query:  Query<&Transform, (With<Player>, Without<WeatherParticle>)>,
) {
    let Ok(player_tf) = player_query.get_single() else { return };
    let player_pos = player_tf.translation;
    let local_up   = player_pos.normalize_or_zero();
    let dt         = time.delta_seconds();

    let fall_speed = match weather.kind {
        WeatherKind::Rain  => 12.0,
        WeatherKind::Storm => 20.0,
        WeatherKind::Snow  =>  2.5,
        _                  =>  0.0,
    };

    for mut tf in &mut part_q {
        tf.translation -= local_up * fall_speed * dt;
        tf.translation += *player_tf.right() * 1.5 * dt;

        let rel    = tf.translation - player_pos;
        let rel_up = rel.dot(local_up);
        if rel_up < -2.0 {
            let mut rng = rand::thread_rng();
            let new_off = Vec3::new(
                rng.gen_range(-50.0f32..50.0),
                rng.gen_range(10.0f32..40.0),
                rng.gen_range(-50.0f32..50.0),
            );
            tf.translation = player_pos + new_off;
        }
    }
}
