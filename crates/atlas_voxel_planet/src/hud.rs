//! In-game HUD: altitude/speed (space) + time-of-day, weather, health/stamina (ground).
//!
//! # Space HUD
//! Shown when the player is above `ATMOSPHERE_FADE_START`.  Displays altitude,
//! speed, and the name + distance of the nearest orbital body.
//!
//! # Ground HUD
//! Shown when the player is below `ATMOSPHERE_FADE_START`.  Anchored to the
//! top-right corner; displays:
//! * **Time of day** — dawn / morning / noon / afternoon / dusk / night
//! * **Weather** — current condition (Clear, Cloudy, Rain, Snow, Storm)
//! * **Health** — `HP 85 / 100` with an ASCII bar
//! * **Stamina** — `SP 60 / 100` with an ASCII bar

use bevy::prelude::*;

use crate::components::*;
use crate::config::*;

// ────────────────────────────────────────────────────────────────────────────

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_space_hud, setup_ground_hud))
            .add_systems(Update, (update_space_hud, update_ground_hud));
    }
}

// ────────────────────────────────────────────────────────────────────────────
//  Components
// ────────────────────────────────────────────────────────────────────────────

/// Marks the text node used by the space HUD.
#[derive(Component)]
pub struct SpaceHudText;

/// Marks the text node used by the ground HUD.
#[derive(Component)]
pub struct GroundHudText;

// ────────────────────────────────────────────────────────────────────────────
//  Setup
// ────────────────────────────────────────────────────────────────────────────

fn setup_space_hud(mut commands: Commands) {
    // Root node: anchored to the top-right corner.
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            top:   Val::Px(12.0),
            right: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        parent.spawn((
            TextBundle::from_section(
                "",
                TextStyle {
                    font_size: 16.0,
                    color:     Color::srgba(0.90, 0.95, 1.00, 0.85),
                    ..default()
                },
            ),
            SpaceHudText,
        ));
    });
}

fn setup_ground_hud(mut commands: Commands) {
    // Root node: anchored to the top-left corner.
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            top:   Val::Px(12.0),
            left:  Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: Val::Px(4.0),
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        parent.spawn((
            TextBundle::from_section(
                "",
                TextStyle {
                    font_size: 15.0,
                    color: Color::srgba(1.00, 0.95, 0.80, 0.90),
                    ..default()
                },
            ),
            GroundHudText,
        ));
    });
}

// ────────────────────────────────────────────────────────────────────────────
//  Update — Space HUD
// ────────────────────────────────────────────────────────────────────────────

pub fn update_space_hud(
    player_q:  Query<(&Transform, &PlayerState), With<Player>>,
    bodies_q:  Query<(&Transform, &Name), With<OrbitalBody>>,
    mut hud_q: Query<(&mut Text, &mut Visibility), With<SpaceHudText>>,
) {
    let Ok((mut text, mut vis)) = hud_q.get_single_mut() else { return };

    let Ok((player_tf, player_state)) = player_q.get_single() else {
        *vis = Visibility::Hidden;
        return;
    };

    let player_pos = player_tf.translation;
    let altitude   = player_pos.length() - PLANET_RADIUS;

    // Only show the HUD above the atmosphere fade start.
    if altitude < ATMOSPHERE_FADE_START {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Inherited;

    let speed_ms = player_state.velocity.length();

    // Find the nearest non-sun orbital body.
    let nearest = bodies_q
        .iter()
        .map(|(tf, name)| {
            let d = (tf.translation - player_pos).length();
            (d, name.as_str().to_owned())
        })
        .filter(|(d, _)| d.is_finite())
        .min_by(|(da, _), (db, _)| da.total_cmp(db));

    let nearest_str = if let Some((dist, name)) = nearest {
        let dist_km = dist / 1_000.0;
        if dist_km >= 1_000.0 {
            format!("Nearest: {} ({:.1} Mm)", name, dist_km / 1_000.0)
        } else {
            format!("Nearest: {} ({:.0} km)", name, dist_km)
        }
    } else {
        String::new()
    };

    let alt_km = altitude / 1_000.0;
    let alt_str = if alt_km >= 1_000.0 {
        format!("ALT  {:.2} Mm", alt_km / 1_000.0)
    } else {
        format!("ALT  {:.1} km", alt_km)
    };

    let spd_str = if speed_ms >= 1_000.0 {
        format!("SPD  {:.2} km/s", speed_ms / 1_000.0)
    } else {
        format!("SPD  {:.0} m/s", speed_ms)
    };

    let mut content = format!("{}\n{}", alt_str, spd_str);
    if !nearest_str.is_empty() {
        content.push('\n');
        content.push_str(&nearest_str);
    }

    if let Some(section) = text.sections.first_mut() {
        section.value = content;
    }
}

// ────────────────────────────────────────────────────────────────────────────
//  Update — Ground HUD
// ────────────────────────────────────────────────────────────────────────────

/// Convert a day fraction (0.0–1.0) to a descriptive time-of-day string.
///
/// Uses ASCII prefixes (not Unicode emoji) because Bevy's default UI font
/// ships with only the `FiraSans` glyph set and emoji would render as the
/// "missing glyph" placeholder box.
fn time_of_day_str(day_fraction: f32) -> &'static str {
    // day_fraction: 0.0 = midnight, 0.25 = dawn, 0.5 = noon, 0.75 = dusk
    match day_fraction {
        f if f < 0.10 => "(   ) Midnight",
        f if f < 0.20 => "(~  ) Before Dawn",
        f if f < 0.30 => "(*  ) Dawn",
        f if f < 0.45 => "(**)  Morning",
        f if f < 0.55 => "(***) Noon",
        f if f < 0.70 => "(** ) Afternoon",
        f if f < 0.80 => "(*  ) Dusk",
        f if f < 0.90 => "(~  ) Evening",
        _             => "(   ) Night",
    }
}

fn weather_str(kind: &WeatherKind) -> &'static str {
    match kind {
        WeatherKind::Clear  => "Clear",
        WeatherKind::Cloudy => "Cloudy",
        WeatherKind::Rain   => "Rain",
        WeatherKind::Snow   => "Snow",
        WeatherKind::Storm  => "Storm",
    }
}

/// Build an ASCII progress bar of width `width` filled to `fraction` (0.0–1.0).
///
/// Uses `#` / `-` (plain ASCII) rather than block-drawing characters so it
/// renders correctly in Bevy's default `FiraSans` font.
fn progress_bar(fraction: f32, width: usize) -> String {
    let filled = ((fraction.clamp(0.0, 1.0) * width as f32).round() as usize).min(width);
    let empty  = width - filled;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
}

pub fn update_ground_hud(
    player_q:  Query<(&Transform, &PlayerState), With<Player>>,
    world_time: Res<WorldTime>,
    weather:    Res<WeatherState>,
    mut hud_q:  Query<(&mut Text, &mut Visibility), With<GroundHudText>>,
) {
    let Ok((mut text, mut vis)) = hud_q.get_single_mut() else { return };

    let Ok((player_tf, player_state)) = player_q.get_single() else {
        *vis = Visibility::Hidden;
        return;
    };

    let altitude = player_tf.translation.length() - PLANET_RADIUS;

    // Only show while in the atmosphere.
    if altitude >= ATMOSPHERE_FADE_START {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Inherited;

    let time_str    = time_of_day_str(world_time.day_fraction);
    let weather_str = weather_str(&weather.kind);

    let hp_frac   = player_state.health  / PLAYER_MAX_HEALTH;
    let st_frac   = player_state.stamina / PLAYER_MAX_STAMINA;
    let hp_bar    = progress_bar(hp_frac, 10);
    let st_bar    = progress_bar(st_frac, 10);

    let content = format!(
        "{}\n{}\nHP {:.0}/{:.0}  {}\nSP {:.0}/{:.0}  {}",
        time_str,
        weather_str,
        player_state.health,  PLAYER_MAX_HEALTH,  hp_bar,
        player_state.stamina, PLAYER_MAX_STAMINA, st_bar,
    );

    if let Some(section) = text.sections.first_mut() {
        section.value = content;
    }
}
