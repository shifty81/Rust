//! Minimap overlay.
//!
//! A small 64 × 64 pixel texture is displayed in the bottom-right corner of
//! the screen.  It shows a top-down view of the terrain biome colours around
//! the player (radius ≈ 200 m), plus coloured dots for the player itself,
//! nearby creatures, and nearby structures.
//!
//! # Colour legend
//! | Dot colour | Meaning |
//! |-----------|---------|
//! | White     | Local player |
//! | Red       | Creature |
//! | Yellow    | Structure |
//!
//! # Architecture
//! * [`MinimapResource`] stores the `Handle<Image>` and a refresh timer.
//! * `setup_minimap` creates the texture asset and a `UiImage` node.
//! * `update_minimap` runs every ~1 s: samples biome colour at every pixel
//!   using the [`NoiseCache`], then draws entity dots on top.
//! * [`MinimapNode`] markers identify the UI image nodes that need updating.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::texture::Image as BevyImage;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::biome::{biome_surface_color, classify_biome};
use crate::components::*;
use crate::planet::NoiseCache;
use crate::structures::Structure;
use crate::wildlife::Creature;

// ─────────────────────────────────────────────────────────────────────────────

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapResource>()
            .add_systems(Startup, setup_minimap)
            .add_systems(Update, update_minimap);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Resolution of the minimap texture (pixels per side).
const MINIMAP_SIZE: u32 = 64;
/// Real-world radius represented by the minimap edge (metres).
const MINIMAP_RADIUS: f32 = 200.0;
/// Display size of the minimap widget (CSS pixels).
const MINIMAP_DISPLAY_PX: f32 = 96.0;
/// Minimum seconds between texture rebuilds.
const MINIMAP_REFRESH_SECS: f32 = 1.0;

// ─────────────────────────────────────────────────────────────────────────────
//  Resources / components
// ─────────────────────────────────────────────────────────────────────────────

/// Holds the minimap texture handle and refresh bookkeeping.
#[derive(Resource, Default)]
pub struct MinimapResource {
    pub image_handle: Handle<BevyImage>,
    pub timer: f32,
}

/// Marks the UI node that displays the minimap texture.
#[derive(Component)]
pub struct MinimapNode;

// ─────────────────────────────────────────────────────────────────────────────
//  Startup
// ─────────────────────────────────────────────────────────────────────────────

pub fn setup_minimap(
    mut commands:  Commands,
    mut images:    ResMut<Assets<BevyImage>>,
    mut res:       ResMut<MinimapResource>,
) {
    let pixel_count = (MINIMAP_SIZE * MINIMAP_SIZE) as usize;
    let data = vec![20u8, 20, 30, 200].repeat(pixel_count);

    let img = BevyImage::new(
        Extent3d { width: MINIMAP_SIZE, height: MINIMAP_SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    res.image_handle = images.add(img);

    // ── Outer border node ────────────────────────────────────────────────────
    let border = commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom:        Val::Px(80.0),   // above the hotbar
            right:         Val::Px(12.0),
            width:         Val::Px(MINIMAP_DISPLAY_PX + 6.0),
            height:        Val::Px(MINIMAP_DISPLAY_PX + 6.0),
            padding:       UiRect::all(Val::Px(3.0)),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.70)),
        ..default()
    }).id();

    // ── Image node ───────────────────────────────────────────────────────────
    let img_node = commands.spawn((
        ImageBundle {
            image: UiImage::new(res.image_handle.clone()),
            style: Style {
                width:  Val::Px(MINIMAP_DISPLAY_PX),
                height: Val::Px(MINIMAP_DISPLAY_PX),
                ..default()
            },
            ..default()
        },
        MinimapNode,
    )).id();

    commands.entity(border).add_child(img_node);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Update
// ─────────────────────────────────────────────────────────────────────────────

pub fn update_minimap(
    time:          Res<Time>,
    mut res:       ResMut<MinimapResource>,
    mut images:    ResMut<Assets<BevyImage>>,
    noise:         Res<NoiseCache>,
    player_q:      Query<&Transform, With<Player>>,
    creature_q:    Query<&Transform, (With<Creature>, Without<Player>)>,
    structure_q:   Query<&Transform, (With<Structure>, Without<Player>)>,
) {
    res.timer += time.delta_seconds();
    if res.timer < MINIMAP_REFRESH_SECS { return; }
    res.timer = 0.0;

    let Ok(player_tf) = player_q.get_single() else { return };
    let Some(img) = images.get_mut(&res.image_handle) else { return };

    let player_pos  = player_tf.translation;
    let local_up    = player_pos.normalize_or_zero();
    if local_up.length_squared() < 0.1 { return; }

    // Compute two perpendicular surface axes.
    let ref_vec = if local_up.abs().dot(Vec3::Y) > 0.9 { Vec3::X } else { Vec3::Y };
    let east    = local_up.cross(ref_vec).normalize();
    let north   = east.cross(local_up).normalize();

    let n = MINIMAP_SIZE as i32;
    let half = n / 2;

    use noise::NoiseFn;

    for py in 0..n {
        for px in 0..n {
            let dx = (px - half) as f32 * MINIMAP_RADIUS / half as f32;
            let dz = (py - half) as f32 * MINIMAP_RADIUS / half as f32;

            // Surface position for this pixel.
            let world_pt = player_pos + east * dx + north * dz;
            let dir      = world_pt.normalize_or_zero();
            if dir.length_squared() < 0.1 {
                write_pixel(img, px as u32, py as u32, [20, 20, 30, 200]);
                continue;
            }

            // Terrain height at this direction.
            let h_raw  = noise.height_fbm.get([dir.x as f64, dir.y as f64, dir.z as f64]) as f32;
            let altitude = h_raw * noise.max_terrain_height;

            // Moisture at this direction.
            let m_raw    = noise.moisture_fbm.get([dir.x as f64, dir.y as f64, dir.z as f64]) as f32;
            let moisture = (m_raw + 1.0) * 0.5;

            let latitude = dir.y;
            let biome    = classify_biome(latitude, altitude, moisture);
            let [r, g, b, _] = biome_surface_color(biome, altitude);

            let pixel = [
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                220u8,
            ];
            write_pixel(img, px as u32, py as u32, pixel);
        }
    }

    // ── Draw a circular border to indicate the map edge ──────────────────────
    for py in 0..n {
        for px in 0..n {
            let fx = (px - half) as f32 / half as f32;
            let fy = (py - half) as f32 / half as f32;
            let dist_sq = fx * fx + fy * fy;
            if dist_sq > 0.96 {
                // Outside the circle: darken.
                write_pixel(img, px as u32, py as u32, [5, 5, 10, 180]);
            }
        }
    }

    // ── Draw entity dots ─────────────────────────────────────────────────────
    for tf in &creature_q {
        if let Some((px, py)) = world_to_minimap(tf.translation, player_pos, east, north) {
            draw_dot(img, px, py, [230, 60, 60, 255]);
        }
    }
    for tf in &structure_q {
        if let Some((px, py)) = world_to_minimap(tf.translation, player_pos, east, north) {
            draw_dot(img, px, py, [255, 220, 60, 255]);
        }
    }

    // ── Draw player dot (always at centre) ───────────────────────────────────
    let cx = (MINIMAP_SIZE / 2) as i32;
    let cy = (MINIMAP_SIZE / 2) as i32;
    draw_dot(img, cx, cy, [255, 255, 255, 255]);

    // ── Draw a north indicator (tiny magenta pixel at top-centre) ────────────
    write_pixel(img, MINIMAP_SIZE / 2, 2, [200, 60, 200, 255]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn write_pixel(img: &mut BevyImage, px: u32, py: u32, rgba: [u8; 4]) {
    let offset = ((py * MINIMAP_SIZE + px) * 4) as usize;
    if offset + 3 < img.data.len() {
        img.data[offset..offset + 4].copy_from_slice(&rgba);
    }
}

fn draw_dot(img: &mut BevyImage, cx: i32, cy: i32, rgba: [u8; 4]) {
    let n = MINIMAP_SIZE as i32;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let px = cx + dx;
            let py = cy + dy;
            if px >= 0 && py >= 0 && px < n && py < n {
                write_pixel(img, px as u32, py as u32, rgba);
            }
        }
    }
}

/// Project a world position onto minimap pixel coordinates.
/// Returns `None` if the entity is outside the minimap radius.
fn world_to_minimap(
    pos: Vec3, player_pos: Vec3, east: Vec3, north: Vec3,
) -> Option<(i32, i32)> {
    let delta = pos - player_pos;
    // Project onto the local surface plane.
    let dx = delta.dot(east);
    let dz = delta.dot(north);

    let half = (MINIMAP_SIZE / 2) as f32;
    let px = (half + dx * half / MINIMAP_RADIUS) as i32;
    let py = (half + dz * half / MINIMAP_RADIUS) as i32;

    let n = MINIMAP_SIZE as i32;
    if px >= 0 && py >= 0 && px < n && py < n {
        // Also check we are inside the circular boundary.
        let fx = (px - n / 2) as f32 / half;
        let fy = (py - n / 2) as f32 / half;
        if fx * fx + fy * fy <= 0.96 {
            return Some((px, py));
        }
    }
    None
}
