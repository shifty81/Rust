//! `atlas_editor_world_settings` — floating panel for live editing of the voxel
//! planet configuration.
//!
//! Shows:
//! * **Terrain** — editable noise seed + all runtime terrain noise parameters
//! * **Chunks** — `render_distance`, `max_chunks_per_frame`, live loaded count
//! * **Day/Night** — `day_fraction` slider, `total_days` readout
//! * **Weather** — current kind dropdown, intensity slider
//! * **Vegetation** — runtime spawn-chance and radius controls
//! * **Player** — read-only speed/gravity constants
//! * **Game World Preview** — 128×128 flat top-down biome preview texture
//!
//! A "Regenerate World" button sends [`RegenerateWorld`] to despawn all chunks
//! and restart generation with the current noise parameters.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{egui, EguiContexts};
use noise::NoiseFn;
use atlas_editor_core::{EditorMode, PanelVisibility};
use atlas_voxel_planet::{
    biome::{biome_surface_color, classify_biome},
    ChunkManager, NoiseSeed, RegenerateWorld, WeatherKind, WeatherState, WorldSettings, WorldTime,
    CAVE_MIN_DEPTH, DAY_LENGTH_SECONDS, FOG_END, FOG_START, GRAVITY_STRENGTH, PLANET_RADIUS,
    PLAYER_EYE_HEIGHT, PLAYER_JUMP_SPEED, PLAYER_RUN_SPEED, PLAYER_WALK_SPEED,
    planet::NoiseCache,
};

// ─────────────────────────────────────────────────────────────────────────────
// World-preview texture resource
// ─────────────────────────────────────────────────────────────────────────────

/// Resolution (pixels per side) of the flat-world preview texture.
const PREVIEW_SIZE: u32 = 128;
/// World-space radius (metres) represented by the preview half-width.
const PREVIEW_WORLD_RADIUS: f32 = 512.0;

/// Holds the flat-world preview texture handle and dirty flag.
#[derive(Resource, Default)]
struct WorldPreviewResource {
    pub image_handle: Handle<Image>,
    /// Set to true when the texture needs a rebuild (first frame or after regen).
    pub dirty: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

pub struct EditorWorldSettingsPlugin;

impl Plugin for EditorWorldSettingsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<WorldPreviewResource>()
            .add_systems(Startup, setup_world_preview)
            .add_systems(Update, (rebuild_world_preview, draw_world_settings_panel).chain());
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
    visibility:     Res<PanelVisibility>,
    preview:        Res<WorldPreviewResource>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }
    if !visibility.world_settings {
        return;
    }

    // Register the preview texture with egui before borrowing the context mutably.
    let preview_tex_id: Option<egui::TextureId> =
        if preview.image_handle != Handle::default() {
            Some(contexts.add_image(preview.image_handle.clone()))
        } else {
            None
        };

    let ctx = contexts.ctx_mut();

    egui::Window::new("🌍 World Settings")
        .default_width(320.0)
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

                            ui.label("Noise Seed");
                            ui.add(egui::DragValue::new(&mut seed.0).speed(1.0));
                            ui.end_row();

                            ui.label("Max Height (m)");
                            ui.add(
                                egui::DragValue::new(&mut settings.max_terrain_height)
                                    .speed(10.0)
                                    .range(50.0..=5000.0),
                            );
                            ui.end_row();

                            ui.label("Terrain Noise Scale");
                            ui.add(
                                egui::DragValue::new(&mut settings.terrain_noise_scale)
                                    .speed(0.05)
                                    .range(0.1..=10.0),
                            );
                            ui.end_row();

                            ui.label("Moisture Noise Scale");
                            ui.add(
                                egui::DragValue::new(&mut settings.moisture_noise_scale)
                                    .speed(0.05)
                                    .range(0.1..=10.0),
                            );
                            ui.end_row();

                            ui.label("Noise Octaves");
                            ui.add(
                                egui::DragValue::new(&mut settings.noise_octaves)
                                    .speed(1.0)
                                    .range(1..=12),
                            );
                            ui.end_row();

                            ui.label("Lacunarity");
                            ui.add(
                                egui::DragValue::new(&mut settings.noise_lacunarity)
                                    .speed(0.05)
                                    .range(1.0..=4.0),
                            );
                            ui.end_row();

                            ui.label("Persistence");
                            ui.add(
                                egui::DragValue::new(&mut settings.noise_persistence)
                                    .speed(0.02)
                                    .range(0.1..=0.9),
                            );
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
                            ui.label("Spawn Radius (m)");
                            ui.add(
                                egui::DragValue::new(&mut settings.vegetation_radius)
                                    .speed(1.0)
                                    .range(10.0..=400.0),
                            );
                            ui.end_row();

                            ui.label("Tree Chance");
                            ui.add(
                                egui::DragValue::new(&mut settings.tree_spawn_chance)
                                    .speed(0.001)
                                    .range(0.0..=0.5),
                            );
                            ui.end_row();

                            ui.label("Grass Chance");
                            ui.add(
                                egui::DragValue::new(&mut settings.grass_spawn_chance)
                                    .speed(0.001)
                                    .range(0.0..=0.5),
                            );
                            ui.end_row();
                        });
                });

            ui.separator();

            // ── Caves ─────────────────────────────────────────────────────────
            egui::CollapsingHeader::new("🕳  Caves")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("cave_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Caves Enabled");
                            ui.checkbox(&mut settings.cave_enabled, "");
                            ui.end_row();

                            ui.label("Cave Noise Scale");
                            ui.add(
                                egui::DragValue::new(&mut settings.cave_scale)
                                    .speed(0.005)
                                    .range(0.005..=0.5),
                            );
                            ui.end_row();

                            ui.label("Cave Threshold");
                            ui.add(
                                egui::Slider::new(&mut settings.cave_threshold, 0.4..=0.95)
                                    .text(""),
                            );
                            ui.end_row();

                            ui.label("Min Depth (vx)");
                            ui.label(format!("{CAVE_MIN_DEPTH}"));
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

            ui.separator();

            // ── Game World Preview ────────────────────────────────────────────
            egui::CollapsingHeader::new("🗺  Game World Preview")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(
                            "Approximate flat-world preview (editor sandbox only)",
                        )
                        .small()
                        .color(egui::Color32::from_rgb(180, 180, 100)),
                    );
                    ui.add_space(4.0);

                    // Display the preview texture if it is ready.
                    if let Some(tex_id) = preview_tex_id {
                        let display_size = egui::vec2(256.0, 256.0);
                        ui.image(egui::load::SizedTexture::new(tex_id, display_size));

                        // Colour legend
                        ui.add_space(4.0);
                        egui::Grid::new("biome_legend")
                            .num_columns(2)
                            .spacing([6.0, 2.0])
                            .show(ui, |ui| {
                                for (name, [r, g, b]) in BIOME_LEGEND {
                                    let color = egui::Color32::from_rgb(
                                        (*r * 255.0) as u8,
                                        (*g * 255.0) as u8,
                                        (*b * 255.0) as u8,
                                    );
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(12.0, 12.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, color);
                                    ui.label(egui::RichText::new(*name).small());
                                    ui.end_row();
                                }
                            });
                    } else {
                        ui.label(egui::RichText::new("(preview not ready)").color(egui::Color32::GRAY));
                    }
                });
        });
}

// ─────────────────────────────────────────────────────────────────────────────
// Biome legend
// ─────────────────────────────────────────────────────────────────────────────

fn weather_label(kind: &WeatherKind) -> &'static str {
    match kind {
        WeatherKind::Clear  => "Clear",
        WeatherKind::Cloudy => "Cloudy",
        WeatherKind::Rain   => "Rain",
        WeatherKind::Snow   => "Snow",
        WeatherKind::Storm  => "Storm",
    }
}

const BIOME_LEGEND: &[(&str, [f32; 3])] = &[
    ("Deep Ocean",      [0.04, 0.12, 0.48]),
    ("Shallow Ocean",   [0.08, 0.28, 0.70]),
    ("Beach",           [0.87, 0.82, 0.60]),
    ("Plains",          [0.40, 0.68, 0.25]),
    ("Forest",          [0.10, 0.42, 0.12]),
    ("Tropical Forest", [0.05, 0.48, 0.10]),
    ("Desert",          [0.85, 0.73, 0.38]),
    ("Savanna",         [0.65, 0.72, 0.28]),
    ("Tundra",          [0.55, 0.58, 0.48]),
    ("Arctic",          [0.90, 0.95, 1.00]),
    ("Mountain",        [0.50, 0.47, 0.42]),
    ("Snow Peak",       [0.96, 0.97, 1.00]),
];

// ─────────────────────────────────────────────────────────────────────────────
// Preview texture setup + rebuild
// ─────────────────────────────────────────────────────────────────────────────

fn setup_world_preview(
    mut images:   ResMut<Assets<Image>>,
    mut preview:  ResMut<WorldPreviewResource>,
) {
    let px = (PREVIEW_SIZE * PREVIEW_SIZE) as usize;
    let data = vec![20u8, 20, 30, 255].repeat(px);
    let img = Image::new(
        Extent3d { width: PREVIEW_SIZE, height: PREVIEW_SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    preview.image_handle = images.add(img);
    preview.dirty = true;
}

fn rebuild_world_preview(
    mut preview:  ResMut<WorldPreviewResource>,
    noise:        Res<NoiseCache>,
    mut images:   ResMut<Assets<Image>>,
    mut regen_ev: EventReader<RegenerateWorld>,
) {
    // Mark dirty whenever a world regeneration is requested.
    if regen_ev.read().next().is_some() {
        preview.dirty = true;
    }

    if !preview.dirty { return; }
    let Some(img) = images.get_mut(&preview.image_handle) else { return };
    preview.dirty = false;

    let n = PREVIEW_SIZE as i32;
    let half = n / 2;

    for py in 0..n {
        for px in 0..n {
            // Map pixel to flat world coordinates centred at (0,0).
            let wx = (px - half) as f32 * PREVIEW_WORLD_RADIUS / half as f32;
            let wz = (py - half) as f32 * PREVIEW_WORLD_RADIUS / half as f32;

            // For a flat-world sample we use a planar direction vector
            // (normalised) to query the same spherical noise functions the
            // 3-D planet uses, at a 1:1 scale.
            let len = (wx * wx + wz * wz + 1.0_f32).sqrt();
            let dx = wx / len;
            let dy = 1.0 / len;   // always "above" the equator
            let dz = wz / len;

            let h_raw = noise.height_fbm.get([dx as f64, dy as f64, dz as f64]) as f32;
            let altitude = h_raw * noise.max_terrain_height;

            let m_raw  = noise.moisture_fbm.get([dx as f64, dy as f64, dz as f64]) as f32;
            let moisture = (m_raw + 1.0) * 0.5;

            // Use latitude = 0.5 (mid-latitude) so the preview shows all
            // biome types; the player can see the 3-D globe for polar regions.
            let latitude = 0.5;
            let biome = classify_biome(latitude, altitude, moisture);
            let [r, g, b, _] = biome_surface_color(biome, altitude);

            let pixel = [
                (r * 255.0).min(255.0) as u8,
                (g * 255.0).min(255.0) as u8,
                (b * 255.0).min(255.0) as u8,
                255u8,
            ];
            let offset = ((py * n + px) as usize) * 4;
            if offset + 3 < img.data.len() {
                img.data[offset..offset + 4].copy_from_slice(&pixel);
            }
        }
    }
}
