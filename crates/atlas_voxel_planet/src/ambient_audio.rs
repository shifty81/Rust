//! Biome-specific ambient audio.
//!
//! Plays a looping background sound that matches the player's current biome.
//! The audio track cross-fades (volume fade-out → swap → fade-in) when the
//! biome changes.
//!
//! # Audio assets
//! Place OGG Vorbis files in `assets/audio/ambient/` with the following names:
//!
//! | File | Played in biome |
//! |------|-----------------|
//! | `plains.ogg`   | Plains |
//! | `forest.ogg`   | Forest / Tropical Forest |
//! | `desert.ogg`   | Desert / Savanna |
//! | `arctic.ogg`   | Arctic / Tundra / SnowPeak |
//! | `mountain.ogg` | Mountain |
//! | `ocean.ogg`    | ShallowOcean / DeepOcean / Beach |
//! | `space.ogg`    | Above atmosphere |
//!
//! If a file is missing, Bevy logs a warning but the game continues silently.
//!
//! # Architecture
//! * [`AmbientAudioPlugin`] registers all systems.
//! * [`AmbientAudioState`] resource tracks the current [`AmbientTrack`] and a
//!   fade timer.
//! * `update_ambient_audio` runs every frame: determines the target track from
//!   the player's biome, manages a [`SoundSink`] entity, and drives the fade.

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;

use crate::biome::{classify_biome, Biome};
use crate::components::*;
use crate::config::*;
use crate::vegetation::simple_moisture;
use crate::planet::terrain_radius_at;

// ─────────────────────────────────────────────────────────────────────────────

pub struct AmbientAudioPlugin;

impl Plugin for AmbientAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmbientAudioState>()
            .add_systems(Update, update_ambient_audio);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Track enum
// ─────────────────────────────────────────────────────────────────────────────

/// Which ambient track should currently be playing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AmbientTrack {
    #[default]
    None,
    Plains,
    Forest,
    Desert,
    Arctic,
    Mountain,
    Ocean,
    Space,
}

impl AmbientTrack {
    /// Asset path relative to the `assets/` directory.
    pub fn path(self) -> Option<&'static str> {
        match self {
            AmbientTrack::None     => None,
            AmbientTrack::Plains   => Some("audio/ambient/plains.ogg"),
            AmbientTrack::Forest   => Some("audio/ambient/forest.ogg"),
            AmbientTrack::Desert   => Some("audio/ambient/desert.ogg"),
            AmbientTrack::Arctic   => Some("audio/ambient/arctic.ogg"),
            AmbientTrack::Mountain => Some("audio/ambient/mountain.ogg"),
            AmbientTrack::Ocean    => Some("audio/ambient/ocean.ogg"),
            AmbientTrack::Space    => Some("audio/ambient/space.ogg"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Resource
// ─────────────────────────────────────────────────────────────────────────────

/// State maintained by the ambient audio system.
#[derive(Resource)]
pub struct AmbientAudioState {
    /// Which track is currently active (or fading out).
    pub current_track: AmbientTrack,
    /// Entity that owns the `AudioBundle` for the current track.
    pub audio_entity: Option<Entity>,
    /// Fade state machine.
    pub fade: FadeState,
    /// Seconds per fade step.
    pub fade_duration: f32,
    /// Accumulated fade time.
    pub fade_timer: f32,
    /// Next track to fade in after fade-out completes.
    pub pending_track: AmbientTrack,
    /// Master volume for ambient sounds (0.0–1.0).
    pub master_volume: f32,
    /// How often (in seconds) to re-sample the player's biome.
    pub sample_timer: f32,
}

impl Default for AmbientAudioState {
    fn default() -> Self {
        Self {
            current_track: AmbientTrack::None,
            audio_entity:  None,
            fade:          FadeState::Idle,
            fade_duration: 2.5,
            fade_timer:    0.0,
            pending_track: AmbientTrack::None,
            master_volume: 0.35,
            sample_timer:  0.0,
        }
    }
}

/// Simple two-phase fade state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FadeState {
    Idle,
    FadingOut,
    FadingIn,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Marker component
// ─────────────────────────────────────────────────────────────────────────────

/// Marks the entity that owns the ambient audio sink.
#[derive(Component)]
pub struct AmbientSoundEntity;

// ─────────────────────────────────────────────────────────────────────────────
//  System
// ─────────────────────────────────────────────────────────────────────────────

/// Determine the ambient track for the player's current position.
fn track_for_player(
    player_pos: Vec3,
    seed: u32,
) -> AmbientTrack {
    let altitude = player_pos.length() - PLANET_RADIUS;

    if altitude >= ATMOSPHERE_FADE_START {
        return AmbientTrack::Space;
    }

    let dir = player_pos.normalize_or_zero();
    if dir.length_squared() < 0.1 {
        return AmbientTrack::Plains;
    }

    let surface_r = terrain_radius_at(dir, seed);
    let alt = surface_r - PLANET_RADIUS;
    let lat = dir.y;
    let moisture = simple_moisture(dir, seed);
    let biome = classify_biome(lat, alt, moisture);

    match biome {
        Biome::Plains                           => AmbientTrack::Plains,
        Biome::Forest | Biome::TropicalForest   => AmbientTrack::Forest,
        Biome::Desert | Biome::Savanna          => AmbientTrack::Desert,
        Biome::Arctic | Biome::Tundra | Biome::SnowPeak => AmbientTrack::Arctic,
        Biome::Mountain                         => AmbientTrack::Mountain,
        Biome::Beach | Biome::ShallowOcean | Biome::DeepOcean => AmbientTrack::Ocean,
    }
}

pub fn update_ambient_audio(
    time:        Res<Time>,
    asset_server: Res<AssetServer>,
    mut state:   ResMut<AmbientAudioState>,
    mut commands: Commands,
    player_q:    Query<&Transform, With<Player>>,
    sink_q:      Query<&AudioSink, With<AmbientSoundEntity>>,
    seed:        Res<NoiseSeed>,
) {
    let dt = time.delta_seconds();

    // ── Sample player biome at regular intervals ──────────────────────────────
    state.sample_timer += dt;
    let new_track = if state.sample_timer >= 3.0 {
        state.sample_timer = 0.0;
        if let Ok(player_tf) = player_q.get_single() {
            track_for_player(player_tf.translation, seed.0)
        } else {
            state.current_track
        }
    } else {
        state.current_track
    };

    // Initiate fade transition if biome changed.
    if new_track != state.current_track
        && state.fade == FadeState::Idle
        && new_track != state.pending_track
    {
        state.pending_track = new_track;
        state.fade = FadeState::FadingOut;
        state.fade_timer = 0.0;
    }

    // ── Fade state machine ────────────────────────────────────────────────────
    match state.fade {
        FadeState::Idle => {
            // If no audio is playing yet and there is a valid track, start it.
            if state.audio_entity.is_none() && state.current_track != AmbientTrack::None {
                start_track(&mut commands, &asset_server, &mut state);
            }
        }

        FadeState::FadingOut => {
            state.fade_timer += dt;
            let progress = (state.fade_timer / state.fade_duration).min(1.0);
            let vol = state.master_volume * (1.0 - progress);

            if let Ok(sink) = state.audio_entity.and_then(|e| sink_q.get(e).ok()).ok_or(()) {
                sink.set_volume(vol);
            }

            if progress >= 1.0 {
                // Stop old entity.
                if let Some(e) = state.audio_entity.take() {
                    commands.entity(e).despawn();
                }
                state.current_track = state.pending_track;
                state.fade = FadeState::FadingIn;
                state.fade_timer = 0.0;

                if state.current_track != AmbientTrack::None {
                    start_track(&mut commands, &asset_server, &mut state);
                    // Start at zero volume for fade-in.
                    if let Some(e) = state.audio_entity {
                        if let Ok(sink) = sink_q.get(e) {
                            sink.set_volume(0.0);
                        }
                    }
                }
            }
        }

        FadeState::FadingIn => {
            state.fade_timer += dt;
            let progress = (state.fade_timer / state.fade_duration).min(1.0);
            let vol = state.master_volume * progress;

            if let Some(e) = state.audio_entity {
                if let Ok(sink) = sink_q.get(e) {
                    sink.set_volume(vol);
                }
            }

            if progress >= 1.0 {
                state.fade = FadeState::Idle;
            }
        }
    }
}

fn start_track(
    commands:     &mut Commands,
    asset_server: &Res<AssetServer>,
    state:        &mut AmbientAudioState,
) {
    let Some(path) = state.current_track.path() else { return };

    let handle: Handle<AudioSource> = asset_server.load(path);
    let entity = commands.spawn((
        AudioBundle {
            source:   handle,
            settings: PlaybackSettings {
                mode:   PlaybackMode::Loop,
                volume: Volume::new(state.master_volume),
                ..default()
            },
        },
        AmbientSoundEntity,
    )).id();
    state.audio_entity = Some(entity);
}
