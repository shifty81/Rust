//! `atlas_editor_viewport` — viewport panel: planet-aware editor camera, overlays,
//! picking, and drop targets.
//!
//! The editor camera starts above the voxel planet's north pole and supports
//! right-mouse-button look + WASD fly (RMB required) + scroll-wheel speed.
//!
//! # Navigation shortcuts (no RMB required)
//! | Key    | Action |
//! |--------|--------|
//! | `Home` | Teleport to solar-system overview (~10 Mm out) |
//! | `End`  | Teleport to planet-surface overview (~2 km up) |
//! | Scroll | Multiply flight speed (×1.25 per notch) |

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts};
use atlas_editor_core::{EditorCamera, EditorMode, EditorPanelOrder, EntityLabel, ViewportRect};
use atlas_editor_project::GameLinkState;
use atlas_gizmos::{GizmoInteraction, GizmoMode};
use atlas_selection::{FocusedEntity, SelectedEntities, SelectionChanged};
use atlas_voxel_planet::{ChunkManager, ChunkViewpoint, VoxelChunk, CHUNK_SIZE, PLANET_RADIUS, SUN_DISTANCE, VOXEL_SIZE};

// ────────────────────────────────────────────────────────────────────────────
// Editor camera state
// ────────────────────────────────────────────────────────────────────────────

/// Per-frame flight state for the editor free-fly camera.
#[derive(Component)]
pub struct EditorCameraState {
    pub yaw:   f32,
    pub pitch: f32,
    /// Flight speed in m/s (scroll wheel adjusts multiplicatively).
    pub speed: f32,
}

impl Default for EditorCameraState {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: -0.3, speed: 500.0 }
    }
}

// ─── Teleport presets ────────────────────────────────────────────────────────

/// Camera position for the solar-system overview (Home key).
/// Sits ~10× the sun-distance away on the +Z axis and tilts slightly down.
const SOLAR_OVERVIEW_POS:   Vec3  = Vec3::new(0.0, SUN_DISTANCE * 3.0, SUN_DISTANCE * 8.0);
const SOLAR_OVERVIEW_PITCH: f32   = -0.18;
const SOLAR_OVERVIEW_SPEED: f32   = 500_000.0;

/// Camera position for the planet-surface overview (End key).
/// Sits 3 km above the north pole and tilts down to look at the terrain.
const PLANET_OVERVIEW_POS:   Vec3  = Vec3::new(0.0, PLANET_RADIUS + 3_000.0, PLANET_RADIUS * 0.08);
const PLANET_OVERVIEW_PITCH: f32   = -0.55;
const PLANET_OVERVIEW_SPEED: f32   = 500.0;

// ────────────────────────────────────────────────────────────────────────────
// Teleport event (also triggered from the View menu)
// ────────────────────────────────────────────────────────────────────────────

/// Sent by the View menu (or keyboard) to teleport the editor camera to a
/// named view preset.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeleportEditorCamera {
    /// Solar-system overview — fits all planets in view.
    SolarSystem,
    /// Planet surface overview — a few km above the north pole.
    PlanetSurface,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorViewportPlugin;

impl Plugin for EditorViewportPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.add_event::<TeleportEditorCamera>()
            .add_systems(Startup, spawn_editor_camera)
            .add_systems(
                Update,
                (
                    update_chunk_viewpoint_from_editor_camera,
                    viewport_mouse_pick,
                    update_editor_camera,
                )
                    .chain()
                    .run_if(in_state(EditorMode::Editing)),
            )
            .add_systems(
                Update,
                // draw_viewport_panel creates a CentralPanel that consumes all remaining
                // space; it must run after every SidePanel and TopBottomPanel is drawn.
                draw_viewport_panel
                    .run_if(in_state(EditorMode::Editing))
                    .in_set(EditorPanelOrder::Central),
            )
            .add_systems(
                Update,
                // Apply the ViewportRect written by draw_viewport_panel to the
                // 3D camera, so the scene is bounded to the central area
                // instead of the entire window.  Must run after the Central
                // panel has been drawn for the current frame.
                sync_camera_viewport
                    .after(EditorPanelOrder::Central),
            );
    }
}

fn spawn_editor_camera(mut commands: Commands) {
    // Start above the north pole, tilted slightly to look at the planet surface.
    let start_pos = Vec3::new(0.0, PLANET_RADIUS + 500.0, PLANET_RADIUS * 0.3);
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(start_pos)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        EditorCamera,
        EditorCameraState::default(),
    ));
}

// ────────────────────────────────────────────────────────────────────────────
// Keep ChunkViewpoint in sync with the editor camera
// ────────────────────────────────────────────────────────────────────────────

fn update_chunk_viewpoint_from_editor_camera(
    cam_q:         Query<&Transform, With<EditorCamera>>,
    mut viewpoint: ResMut<ChunkViewpoint>,
) {
    if let Ok(tf) = cam_q.get_single() {
        viewpoint.0 = tf.translation;
    }
}

// ─── Teleport helper ─────────────────────────────────────────────────────────

/// Apply a teleport preset to the camera transform and state.
fn apply_teleport(
    transform: &mut Transform,
    state:     &mut EditorCameraState,
    pos:       Vec3,
    pitch:     f32,
    speed:     f32,
) {
    transform.translation = pos;
    state.yaw   = 0.0;
    state.pitch = pitch;
    state.speed = speed;
    transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
}

// ────────────────────────────────────────────────────────────────────────────
// Free-fly camera controls
// ────────────────────────────────────────────────────────────────────────────

fn update_editor_camera(
    time:              Res<Time>,
    keyboard:          Res<ButtonInput<KeyCode>>,
    mouse_button:      Res<ButtonInput<MouseButton>>,
    mut motion_events: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    mut teleport_ev:   EventReader<TeleportEditorCamera>,
    mut cam_q:         Query<(&mut Transform, &mut EditorCameraState), With<EditorCamera>>,
    mut contexts:      EguiContexts,
) {
    let Ok((mut transform, mut state)) = cam_q.get_single_mut() else { return };

    // Yield input to egui when the user is interacting with the UI — prevents
    // WASD / scroll from bleeding through into the 3D camera while a text
    // field is focused or the pointer is over a panel.
    let (egui_wants_pointer, egui_wants_keyboard) = {
        let ctx = contexts.ctx_mut();
        (ctx.wants_pointer_input(), ctx.wants_keyboard_input())
    };
    if egui_wants_pointer || egui_wants_keyboard {
        motion_events.clear();
        scroll_events.clear();
        // Still consume teleport events so they aren't lost.
        for ev in teleport_ev.read() {
            match ev {
                TeleportEditorCamera::SolarSystem => {
                    apply_teleport(&mut transform, &mut state,
                        SOLAR_OVERVIEW_POS, SOLAR_OVERVIEW_PITCH, SOLAR_OVERVIEW_SPEED);
                }
                TeleportEditorCamera::PlanetSurface => {
                    apply_teleport(&mut transform, &mut state,
                        PLANET_OVERVIEW_POS, PLANET_OVERVIEW_PITCH, PLANET_OVERVIEW_SPEED);
                }
            }
        }
        return;
    }

    let rmb = mouse_button.pressed(MouseButton::Right);

    // ── Teleport events (from View menu) ────────────────────────────────────
    for ev in teleport_ev.read() {
        match ev {
            TeleportEditorCamera::SolarSystem => {
                apply_teleport(&mut transform, &mut state,
                    SOLAR_OVERVIEW_POS, SOLAR_OVERVIEW_PITCH, SOLAR_OVERVIEW_SPEED);
            }
            TeleportEditorCamera::PlanetSurface => {
                apply_teleport(&mut transform, &mut state,
                    PLANET_OVERVIEW_POS, PLANET_OVERVIEW_PITCH, PLANET_OVERVIEW_SPEED);
            }
        }
    }

    // ── Keyboard teleport shortcuts (no RMB required) ────────────────────────
    if keyboard.just_pressed(KeyCode::Home) {
        apply_teleport(&mut transform, &mut state,
            SOLAR_OVERVIEW_POS, SOLAR_OVERVIEW_PITCH, SOLAR_OVERVIEW_SPEED);
        return;
    }
    if keyboard.just_pressed(KeyCode::End) {
        apply_teleport(&mut transform, &mut state,
            PLANET_OVERVIEW_POS, PLANET_OVERVIEW_PITCH, PLANET_OVERVIEW_SPEED);
        return;
    }

    // ── Scroll wheel → multiply speed (logarithmic feel) ────────────────────
    for ev in scroll_events.read() {
        if ev.y > 0.0 {
            state.speed *= 1.0 + ev.y.abs() * 0.25;
        } else if ev.y < 0.0 {
            state.speed /= 1.0 + ev.y.abs() * 0.25;
        }
        state.speed = state.speed.clamp(1.0, 1_000_000.0);
    }

    // ── RMB held → look ─────────────────────────────────────────────────────
    if rmb {
        for ev in motion_events.read() {
            state.yaw   -= ev.delta.x * 0.003;
            state.pitch -= ev.delta.y * 0.003;
            state.pitch  = state.pitch.clamp(-1.55, 1.55);
        }
    } else {
        motion_events.clear();
    }

    // Apply accumulated yaw + pitch to rotation.
    transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);

    // ── WASD + Q/E → fly (only while RMB is held) ───────────────────────────
    if rmb {
        let fwd   = transform.rotation * Vec3::NEG_Z;
        let right = transform.rotation * Vec3::X;
        let up    = Vec3::Y;

        let mut move_dir = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) { move_dir += fwd;   }
        if keyboard.pressed(KeyCode::KeyS) { move_dir -= fwd;   }
        if keyboard.pressed(KeyCode::KeyA) { move_dir -= right; }
        if keyboard.pressed(KeyCode::KeyD) { move_dir += right; }
        if keyboard.pressed(KeyCode::KeyE) { move_dir += up;    }
        if keyboard.pressed(KeyCode::KeyQ) { move_dir -= up;    }

        if move_dir.length_squared() > 0.0 {
            transform.translation +=
                move_dir.normalize() * state.speed * time.delta_seconds();
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Viewport mouse picking — LMB click selects nearest chunk / entity
// ────────────────────────────────────────────────────────────────────────────

/// On LMB click (when not dragging a gizmo), cast a ray from the editor camera
/// through the cursor.  Hits are tested against:
/// 1. User-spawned entities with [`EntityLabel`] — sphere AABB around origin.
/// 2. Voxel chunk AABBs.
/// The nearest hit is focused; Ctrl is NOT available here (viewport is single-select).
fn viewport_mouse_pick(
    buttons:      Res<ButtonInput<MouseButton>>,
    keyboard:     Res<ButtonInput<KeyCode>>,
    windows:      Query<&Window, With<PrimaryWindow>>,
    camera_q:     Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    chunk_mgr:    Res<ChunkManager>,
    chunk_q:      Query<&VoxelChunk>,
    user_q:       Query<(Entity, &GlobalTransform), With<EntityLabel>>,
    interaction:  Res<GizmoInteraction>,
    mut focused:  ResMut<FocusedEntity>,
    mut selected: ResMut<SelectedEntities>,
    mut changed:  EventWriter<SelectionChanged>,
    mut contexts: EguiContexts,
) {
    // Skip if a gizmo drag is active or no LMB click.
    if interaction.active { return; }
    if !buttons.just_pressed(MouseButton::Left) { return; }

    // Yield picking to egui when the pointer is over an egui panel / widget.
    if contexts.ctx_mut().wants_pointer_input() { return; }

    let Ok(window) = windows.get_single() else { return };
    let Ok((camera, cam_tf)) = camera_q.get_single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Some(ray) = camera.viewport_to_world(cam_tf, cursor) else { return };

    let ray_origin = ray.origin;
    let ray_dir    = *ray.direction;

    let cs = (CHUNK_SIZE as f32) * VOXEL_SIZE;

    let mut best_dist = f32::MAX;
    let mut best_ent  = None::<Entity>;

    // ── User entities (EntityLabel) — sphere test ────────────────────────────
    for (entity, gtf) in &user_q {
        let center = gtf.translation();
        // Approximate bounding sphere: radius 0.7 m (covers a unit cube).
        let radius = 0.7_f32;
        if let Some(dist) = ray_sphere(ray_origin, ray_dir, center, radius) {
            if dist < best_dist {
                best_dist = dist;
                best_ent  = Some(entity);
            }
        }
    }

    // ── Voxel chunks ─────────────────────────────────────────────────────────
    for (&coord, &entity) in &chunk_mgr.loaded {
        if entity == Entity::PLACEHOLDER { continue; }
        if chunk_q.get(entity).is_err() { continue; }

        let min = Vec3::new(coord.x as f32, coord.y as f32, coord.z as f32) * cs;
        let max = min + Vec3::splat(cs);

        if let Some(dist) = ray_aabb(ray_origin, ray_dir, min, max) {
            if dist < best_dist {
                best_dist = dist;
                best_ent  = Some(entity);
            }
        }
    }

    if let Some(ent) = best_ent {
        let ctrl = keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        if ctrl {
            selected.toggle(ent);
            if focused.0.is_none() { focused.0 = Some(ent); }
        } else {
            selected.set_single(ent);
            focused.0 = Some(ent);
        }
        changed.send(SelectionChanged);
    }
}

/// Slab method AABB–ray intersection.  Returns the entry distance along the
/// ray if there is an intersection with t > 0, otherwise `None`.
fn ray_aabb(origin: Vec3, dir: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> Option<f32> {    let inv_dir = Vec3::new(
        if dir.x.abs() > 1e-10 { 1.0 / dir.x } else { f32::MAX },
        if dir.y.abs() > 1e-10 { 1.0 / dir.y } else { f32::MAX },
        if dir.z.abs() > 1e-10 { 1.0 / dir.z } else { f32::MAX },
    );

    let t1 = (aabb_min - origin) * inv_dir;
    let t2 = (aabb_max - origin) * inv_dir;

    let t_min = t1.min(t2);
    let t_max = t1.max(t2);

    let t_enter = t_min.x.max(t_min.y).max(t_min.z);
    let t_exit  = t_max.x.min(t_max.y).min(t_max.z);

    if t_exit >= t_enter && t_exit > 0.0 {
        Some(t_enter.max(0.0))
    } else {
        None
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Viewport panel (egui overlay)
// ────────────────────────────────────────────────────────────────────────────

fn draw_viewport_panel(
    mut contexts:      EguiContexts,
    cam_q:             Query<(&Transform, &EditorCameraState), With<EditorCamera>>,
    diagnostics:       Res<DiagnosticsStore>,
    gizmo_mode:        Res<GizmoMode>,
    game_link:         Res<GameLinkState>,
    mut viewport_rect: ResMut<ViewportRect>,
) {
    let ctx = contexts.ctx_mut();

    // Transparent CentralPanel so the 3D render underneath shows through.
    // The info overlay is drawn as a floating Area with its own background,
    // so the rest of the central region stays clear.
    let pixels_per_point = ctx.pixels_per_point();
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            let rect = ui.max_rect();

            // Record the viewport rectangle (logical pixels) so
            // sync_camera_viewport can clip the 3D camera to it.
            viewport_rect.min = Vec2::new(rect.min.x, rect.min.y);
            viewport_rect.max = Vec2::new(rect.max.x, rect.max.y);
            viewport_rect.scale_factor = pixels_per_point;

            // ── Info overlay: floating, anchored to the viewport top-left ─
            let overlay_frame = egui::Frame::none()
                .fill(egui::Color32::from_rgba_unmultiplied(20, 24, 28, 200))
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .rounding(egui::Rounding::same(4.0));

            egui::Area::new(egui::Id::new("atlas_viewport_overlay"))
                .fixed_pos(rect.min + egui::vec2(8.0, 8.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    overlay_frame.show(ui, |ui| {
                        // ── FPS / performance readout ─────────────────────
                        let fps = diagnostics
                            .get(&FrameTimeDiagnosticsPlugin::FPS)
                            .and_then(|d| d.smoothed())
                            .unwrap_or(0.0);
                        let frame_ms = diagnostics
                            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                            .and_then(|d| d.smoothed())
                            .unwrap_or(0.0)
                            * 1000.0;

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{fps:.1} FPS  ({frame_ms:.2} ms)"))
                                    .color(egui::Color32::from_rgb(120, 220, 120)),
                            );
                            ui.separator();
                            ui.label(
                                egui::RichText::new(gizmo_mode.label())
                                    .color(egui::Color32::from_rgb(220, 180, 80)),
                            );
                            ui.separator();
                            ui.label(
                                egui::RichText::new("EDITING")
                                    .color(egui::Color32::from_rgb(80, 160, 255))
                                    .strong(),
                            );
                            ui.separator();
                            if game_link.is_linked() {
                                let proj_name = game_link
                                    .game_path
                                    .as_ref()
                                    .and_then(|p| p.file_name())
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("Nova-Forge");
                                ui.label(
                                    egui::RichText::new(
                                        format!("Nova-Forge project: {proj_name} — viewport ready for game content"),
                                    )
                                    .color(egui::Color32::from_rgb(100, 210, 160))
                                    .italics()
                                    .small(),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("Editor Sandbox — spherical planet (not the game world)")
                                        .color(egui::Color32::from_rgb(200, 160, 80))
                                        .italics()
                                        .small(),
                                );
                            }
                        });

                        // ── Camera info ───────────────────────────────────
                        if let Ok((tf, state)) = cam_q.get_single() {
                            let p     = tf.translation;
                            let dist  = p.length();
                            let alt_km = (dist - PLANET_RADIUS) / 1_000.0;
                            let sun_dist_km = (p - Vec3::new(SUN_DISTANCE, 0.0, 0.0)).length() / 1_000.0;

                            if game_link.is_linked() {
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "Pos ({:.0}, {:.0}, {:.0})  speed {:.0} m/s",
                                        p.x, p.y, p.z, state.speed,
                                    ));
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "Pos ({:.0}, {:.0}, {:.0})  alt {}{:.1} km  speed {:.0} m/s",
                                        p.x, p.y, p.z,
                                        if alt_km < 0.0 { "-" } else { "+" },
                                        alt_km.abs(),
                                        state.speed,
                                    ));
                                    ui.separator();
                                    ui.label(format!("☀  {:.0} km", sun_dist_km));
                                });
                            }

                            ui.label(
                                egui::RichText::new(
                                    "RMB+drag: look · WASD/QE: fly · scroll: speed  |  \
                                     Home: solar system · End: planet surface  |  \
                                     W/E/R: gizmo · G: grid"
                                )
                                .weak()
                                .small(),
                            );
                        }
                    });
                });

            // ── Nova-Forge placeholder: centred message when linked and scene is empty ─
            if game_link.is_linked() {
                let center = rect.center();
                egui::Area::new(egui::Id::new("atlas_nova_forge_placeholder"))
                    .fixed_pos(egui::pos2(center.x - 220.0, center.y - 40.0))
                    .order(egui::Order::Background)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_unmultiplied(10, 14, 18, 180))
                            .inner_margin(egui::Margin::symmetric(24.0, 16.0))
                            .rounding(egui::Rounding::same(8.0))
                            .show(ui, |ui| {
                                ui.set_width(440.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new("🎮  Nova-Forge Project Linked")
                                            .color(egui::Color32::from_rgb(100, 210, 160))
                                            .size(18.0)
                                            .strong(),
                                    );
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "The editor sandbox is suppressed while a game repo is linked.\n\
                                             Open a scene or use File → Import to load game content."
                                        )
                                        .color(egui::Color32::from_rgb(180, 180, 180))
                                        .small(),
                                    );
                                });
                            });
                    });
            }
        });
}

// ────────────────────────────────────────────────────────────────────────────
// Camera viewport sync
// ────────────────────────────────────────────────────────────────────────────

/// Minimum clamp for `pixels_per_point` / window scale-factor values read
/// from egui and winit.  Prevents division/scaling artefacts if either source
/// ever reports 0 (e.g. during an early frame before window metrics are
/// available).  Not user-facing — any positive value below normal DPI scales
/// (typically 1.0–3.0) works; we pick a value small enough to never clamp a
/// legitimate display and large enough to keep f32 math stable.
const MIN_SCALE_FACTOR: f32 = 1.0e-4;

/// Apply [`ViewportRect`] to the editor camera each frame so the 3D scene
/// renders only inside the central panel area, not behind the surrounding
/// egui panels.
///
/// When the rect is empty or we're not in Editing mode, the camera viewport
/// is cleared so the scene renders full-window (the PIE runtime expects
/// full-window rendering).
fn sync_camera_viewport(
    viewport_rect: Res<ViewportRect>,
    windows:       Query<&Window, With<PrimaryWindow>>,
    mode:          Res<State<EditorMode>>,
    mut cam_q:     Query<&mut Camera, With<EditorCamera>>,
) {
    let Ok(mut camera) = cam_q.get_single_mut() else { return };

    // In PIE / Simulate the editor camera is inactive; game cameras render
    // full-window.  Clear any viewport restriction so re-activation on Stop
    // starts from a clean slate.
    if *mode.get() != EditorMode::Editing {
        if camera.viewport.is_some() {
            camera.viewport = None;
        }
        return;
    }

    if viewport_rect.is_empty() {
        if camera.viewport.is_some() {
            camera.viewport = None;
        }
        return;
    }

    let Ok(window) = windows.get_single() else { return };
    let window_w = window.physical_width();
    let window_h = window.physical_height();
    if window_w == 0 || window_h == 0 { return; }

    let scale = viewport_rect.scale_factor.max(MIN_SCALE_FACTOR);

    // Logical → physical pixels, then clamp to window bounds.
    let min_x = ((viewport_rect.min.x * scale).max(0.0) as u32).min(window_w.saturating_sub(1));
    let min_y = ((viewport_rect.min.y * scale).max(0.0) as u32).min(window_h.saturating_sub(1));
    let max_x = ((viewport_rect.max.x * scale).max(0.0) as u32).min(window_w);
    let max_y = ((viewport_rect.max.y * scale).max(0.0) as u32).min(window_h);

    if max_x <= min_x || max_y <= min_y {
        // Degenerate — e.g. side panels cover the entire window.  Leave the
        // camera alone rather than creating a 0-sized viewport (wgpu rejects).
        return;
    }

    // Enforce a 1×1 minimum for safety.
    let size_x = (max_x - min_x).max(1);
    let size_y = (max_y - min_y).max(1);

    let new_vp = Viewport {
        physical_position: UVec2::new(min_x, min_y),
        physical_size:     UVec2::new(size_x, size_y),
        depth:             0.0..1.0,
    };

    // Only write when something changed, to avoid touching Camera every frame.
    let needs_update = match &camera.viewport {
        None => true,
        Some(current) => {
            current.physical_position != new_vp.physical_position
                || current.physical_size != new_vp.physical_size
        }
    };
    if needs_update {
        camera.viewport = Some(new_vp);
    }
}

/// Sphere–ray intersection.  Returns the entry distance along the ray
/// if the ray hits the sphere with t > 0, otherwise `None`.
fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc  = origin - center;
    let b   = oc.dot(dir);
    let c   = oc.dot(oc) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 { return None; }
    let sqrt_disc = disc.sqrt();
    let t0 = -b - sqrt_disc;
    let t1 = -b + sqrt_disc;
    let t = if t0 > 0.0 { t0 } else if t1 > 0.0 { t1 } else { return None; };
    Some(t)
}
