//! `nf_editor_world_settings` — floating panel for live editing of the voxel
//! planet configuration.
//!
//! Shows:
//! * **Terrain** — read-only planet constants + editable `NoiseSeed`
//! * **Chunks** — `render_distance`, `max_chunks_per_frame`, live loaded count
//! * **Day/Night** — `day_fraction` slider, `total_days` readout
//! * **Weather** — current kind dropdown, intensity slider
//! * **Vegetation** — read-only spawn-chance and radius constants
//! * **Player** — read-only speed/gravity constants
//!
//! A "Regenerate World" button sends [`RegenerateWorld`] to despawn all chunks
//! and restart generation with the current [`NoiseSeed`].

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::EditorMode;
use nf_voxel_planet::{
    ChunkManager, NoiseSeed, RegenerateWorld, WeatherKind, WeatherState, WorldSettings, WorldTime,
    AXIAL_TILT, DAY_LENGTH_SECONDS, FOG_END, FOG_START, GRASS_SPAWN_CHANCE, GRAVITY_STRENGTH,
    MAX_TERRAIN_HEIGHT, MOISTURE_NOISE_SCALE, PLANET_RADIUS, PLAYER_EYE_HEIGHT,
    PLAYER_JUMP_SPEED, PLAYER_RUN_SPEED, PLAYER_WALK_SPEED, TERRAIN_NOISE_SCALE,
    TREE_SPAWN_CHANCE, VEGETATION_RADIUS,
};

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

pub struct EditorWorldSettingsPlugin;

impl Plugin for EditorWorldSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_world_settings_panel);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Panel
// ─────────────────────────────────────────────────────────────────────────────

fn draw_world_settings_panel(
    mut contexts:   EguiContexts,
    mode:           Res<State<EditorMode>>,
    mut seed:       ResMut<NoiseSeed>,
    mut settings:   ResMut<WorldSettings>,
    mut world_time: ResMut<WorldTime>,
    mut weather:    ResMut<WeatherState>,
    chunk_mgr:      Res<ChunkManager>,
    mut regen_ev:   EventWriter<RegenerateWorld>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new("🌍 World Settings")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            // ── Terrain ──────────────────────────────────────────────────────
            egui::CollapsingHeader::new("⛰  Terrain")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("terrain_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Planet Radius");
                            ui.label(format!("{PLANET_RADIUS:.0} m"));
                            ui.end_row();

                            ui.label("Max Height");
                            ui.label(format!("{MAX_TERRAIN_HEIGHT:.0} m"));
                            ui.end_row();

                            ui.label("Terrain Noise Scale");
                            ui.label(format!("{TERRAIN_NOISE_SCALE:.2}"));
                            ui.end_row();

                            ui.label("Moisture Noise Scale");
                            ui.label(format!("{MOISTURE_NOISE_SCALE:.2}"));
                            ui.end_row();

                            ui.label("Noise Seed");
                            ui.add(egui::DragValue::new(&mut seed.0).speed(1.0));
                            ui.end_row();
                        });

                    ui.separator();
                    if ui
                        .add_sized(
                            [ui.available_width(), 24.0],
                            egui::Button::new("♻  Regenerate World"),
                        )
                        .clicked()
                    {
                        regen_ev.send(RegenerateWorld);
                    }
                });

            ui.separator();

            // ── Chunks ───────────────────────────────────────────────────────
            egui::CollapsingHeader::new("🧱  Chunks")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("chunk_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Render Distance");
                            ui.add(
                                egui::DragValue::new(&mut settings.render_distance)
                                    .speed(1.0)
                                    .range(1..=20),
                            );
                            ui.end_row();

                            ui.label("Max Per Frame");
                            ui.add(
                                egui::DragValue::new(&mut settings.max_chunks_per_frame)
                                    .speed(1.0)
                                    .range(1..=32),
                            );
                            ui.end_row();

                            ui.label("Loaded Chunks");
                            ui.label(format!("{}", chunk_mgr.loaded.len()));
                            ui.end_row();

                            ui.label("Pending Queue");
                            ui.label(format!("{}", chunk_mgr.pending.len()));
                            ui.end_row();
                        });
                });

            ui.separator();

            // ── Day / Night ───────────────────────────────────────────────────
            egui::CollapsingHeader::new("☀  Day / Night")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("time_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Day Length");
                            ui.label(format!("{DAY_LENGTH_SECONDS:.0} s"));
                            ui.end_row();

                            ui.label("Axial Tilt");
                            ui.label(format!("{:.1}°", AXIAL_TILT.to_degrees()));
                            ui.end_row();

                            ui.label("Day Fraction");
                            ui.add(
                                egui::Slider::new(&mut world_time.day_fraction, 0.0..=1.0)
                                    .text(""),
                            );
                            ui.end_row();

                            ui.label("Total Days");
                            ui.label(format!("{:.1}", world_time.total_days));
                            ui.end_row();
                        });
                });

            ui.separator();

            // ── Weather ───────────────────────────────────────────────────────
            egui::CollapsingHeader::new("⛅  Weather")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("weather_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Kind");
                            egui::ComboBox::from_id_source("weather_kind")
                                .selected_text(weather_label(&weather.kind))
                                .show_ui(ui, |ui| {
                                    let kinds = [
                                        WeatherKind::Clear,
                                        WeatherKind::Cloudy,
                                        WeatherKind::Rain,
                                        WeatherKind::Snow,
                                        WeatherKind::Storm,
                                    ];
                                    for k in &kinds {
                                        if ui
                                            .selectable_label(weather.kind == *k, weather_label(k))
                                            .clicked()
                                        {
                                            weather.kind = k.clone();
                                        }
                                    }
                                });
                            ui.end_row();

                            ui.label("Intensity");
                            ui.add(
                                egui::Slider::new(&mut weather.intensity, 0.0..=1.0).text(""),
                            );
                            ui.end_row();
                        });
                });

            ui.separator();

            // ── Vegetation ───────────────────────────────────────────────────
            egui::CollapsingHeader::new("🌲  Vegetation")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("veg_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Spawn Radius");
                            ui.label(format!("{VEGETATION_RADIUS:.0} m"));
                            ui.end_row();

                            ui.label("Tree Chance");
                            ui.label(format!("{TREE_SPAWN_CHANCE:.4}"));
                            ui.end_row();

                            ui.label("Grass Chance");
                            ui.label(format!("{GRASS_SPAWN_CHANCE:.4}"));
                            ui.end_row();
                        });
                });

            ui.separator();

            // ── Player ───────────────────────────────────────────────────────
            egui::CollapsingHeader::new("👤  Player")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("player_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Walk Speed");
                            ui.label(format!("{PLAYER_WALK_SPEED:.1} m/s"));
                            ui.end_row();

                            ui.label("Run Speed");
                            ui.label(format!("{PLAYER_RUN_SPEED:.1} m/s"));
                            ui.end_row();

                            ui.label("Jump Speed");
                            ui.label(format!("{PLAYER_JUMP_SPEED:.1} m/s"));
                            ui.end_row();

                            ui.label("Gravity");
                            ui.label(format!("{GRAVITY_STRENGTH:.2} m/s²"));
                            ui.end_row();

                            ui.label("Eye Height");
                            ui.label(format!("{PLAYER_EYE_HEIGHT:.2} m"));
                            ui.end_row();

                            ui.label("Fog Start");
                            ui.label(format!("{FOG_START:.0} m"));
                            ui.end_row();

                            ui.label("Fog End");
                            ui.label(format!("{FOG_END:.0} m"));
                            ui.end_row();
                        });
                });
        });
}

fn weather_label(kind: &WeatherKind) -> &'static str {
    match kind {
        WeatherKind::Clear  => "Clear",
        WeatherKind::Cloudy => "Cloudy",
        WeatherKind::Rain   => "Rain",
        WeatherKind::Snow   => "Snow",
        WeatherKind::Storm  => "Storm",
    }
}
