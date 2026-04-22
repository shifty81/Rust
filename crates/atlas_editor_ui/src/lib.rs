//! `atlas_editor_ui` — egui shell: main menu bar, snap toolbar, docking layout,
//! and floating utility windows (undo history).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use atlas_editor_core::{
    DeleteEntityRequest, DuplicateEntityRequest, EditorMode, EditorPanelOrder, PanelVisibility,
    PrimitiveKind, RequestEditorMode, SpawnEntityRequest,
};
use atlas_editor_scene::{NewSceneRequest, OpenSceneRequest, SaveSceneRequest};
use atlas_editor_play::{StartPie, StopPie, PausePie};
use atlas_editor_viewport::TeleportEditorCamera;
use atlas_commands::{UndoRequested, RedoRequested, CommandHistory};
use atlas_gizmos::{GizmoSpace, SnapSettings};
use atlas_selection::FocusedEntity;
use atlas_scene::{ActiveScenePath, SceneDirty};
use atlas_editor_project::{GameLinkState, LinkGameRequest, UnlinkGameRequest, OpenProjectRequest};
use atlas_editor_export::{ExportToGameRequest, LaunchGameRequest};

/// Placeholder path used when no file dialog is available yet.
const OPEN_SCENE_PLACEHOLDER: &str = "project/Scenes/untitled.atlasscene";

// ────────────────────────────────────────────────────────────────────────────
// Shared UI state (refreshed each frame before the menu bar is drawn)
// ────────────────────────────────────────────────────────────────────────────

/// Lightweight per-frame cache for values read from other crates' resources.
/// Keeping this out of the menu-bar system parameters keeps that system within
/// Bevy's 16-parameter limit.
#[derive(Resource, Default)]
struct EditorUiState {
    /// Label of the command on top of the undo stack (None = empty stack).
    undo_label: Option<String>,
    /// Label of the command on top of the redo stack (None = empty stack).
    redo_label: Option<String>,
    /// Whether the floating Undo History window is currently shown.
    undo_history_visible: bool,
    /// Name of the currently open scene (or "Untitled").
    active_scene_name: String,
    /// Whether the scene has unsaved changes.
    scene_is_dirty: bool,
    /// Set to true by the Edit menu to delete the focused entity.
    delete_entity_requested: bool,
    /// Set to true by the Edit menu to duplicate the focused entity.
    duplicate_entity_requested: bool,
    /// Cached snapshot of [`PanelVisibility`] so the menu bar can render ✔
    /// ticks without pulling the resource into its 16-parameter budget.
    panel_visibility: PanelVisibility,
    /// Pending panel-visibility changes from the View menu — applied to the
    /// actual resource by `dispatch_ui_requests`.
    pending_panel_toggle: Option<PanelToggle>,
    // ── Nova-Forge integration ───────────────────────────────────────────────
    /// Cached game-link state for the Nova-Forge menu status indicator.
    game_link_state: GameLinkState,
    /// Set by the Nova-Forge menu to request an export.
    export_requested: bool,
    /// Set by the Nova-Forge menu to request launching the game.
    launch_game_requested: bool,
    /// Set by the Nova-Forge menu to open a folder picker and link a repo.
    link_requested: bool,
    /// Set by the Nova-Forge menu to unlink the current repo.
    unlink_requested: bool,
    /// Set by the File menu to open a folder picker and load a project.
    open_project_requested: bool,
}

/// One of the panels togglable from the View menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelToggle {
    Outliner,
    Details,
    ContentBrowser,
    OutputLog,
    SnapToolbar,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }
        app
            .init_resource::<EditorUiState>()
            .add_systems(
                Update,
                // Pre-panel: read input and sync cached state before any egui draw.
                (keyboard_shortcuts, sync_ui_state)
                    .chain()
                    .before(EditorPanelOrder::Top),
            )
            .add_systems(
                Update,
                // Top panels: menu bar + snap toolbar (must precede side/central panels).
                (draw_menu_bar, draw_snap_toolbar, draw_undo_history_window)
                    .chain()
                    .in_set(EditorPanelOrder::Top),
            )
            .add_systems(
                Update,
                // Post-panel: dispatch any deferred entity requests after all drawing.
                (dispatch_ui_requests, dispatch_nova_forge_requests)
                    .chain()
                    .after(EditorPanelOrder::Central),
            )
            .add_systems(Update, update_window_title);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Keyboard shortcuts
// ────────────────────────────────────────────────────────────────────────────

fn keyboard_shortcuts(
    keys:         Res<ButtonInput<KeyCode>>,
    mode:         Res<State<EditorMode>>,
    focused:      Res<FocusedEntity>,
    mut undo_ev:  EventWriter<UndoRequested>,
    mut redo_ev:  EventWriter<RedoRequested>,
    mut delete_ev: EventWriter<DeleteEntityRequest>,
    mut dup_ev:   EventWriter<DuplicateEntityRequest>,
) {
    let ctrl  = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft,   KeyCode::ShiftRight]);

    if ctrl && !shift && keys.just_pressed(KeyCode::KeyZ) {
        undo_ev.send(UndoRequested);
    }
    // Ctrl+Y or Ctrl+Shift+Z for redo.
    if ctrl && (keys.just_pressed(KeyCode::KeyY) || (shift && keys.just_pressed(KeyCode::KeyZ))) {
        redo_ev.send(RedoRequested);
    }

    // Only meaningful in Editing mode.
    if *mode.get() != EditorMode::Editing {
        return;
    }

    // Delete key — despawn focused entity.
    if keys.just_pressed(KeyCode::Delete) {
        if let Some(entity) = focused.0 {
            delete_ev.send(DeleteEntityRequest(entity));
        }
    }

    // Ctrl+D — duplicate focused entity.
    if ctrl && !shift && keys.just_pressed(KeyCode::KeyD) {
        if let Some(entity) = focused.0 {
            dup_ev.send(DuplicateEntityRequest(entity));
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// UI state sync (runs before draw_menu_bar)
// ────────────────────────────────────────────────────────────────────────────

fn sync_ui_state(
    history:     Res<CommandHistory>,
    mut state:   ResMut<EditorUiState>,
    active_path: Res<ActiveScenePath>,
    dirty:       Res<SceneDirty>,
    visibility:  Res<PanelVisibility>,
    game_link:   Res<GameLinkState>,
) {
    state.undo_label = history.undo_label().map(str::to_owned);
    state.redo_label = history.redo_label().map(str::to_owned);
    state.active_scene_name = active_path.0
        .as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_owned();
    state.scene_is_dirty = dirty.0;
    state.panel_visibility = *visibility;
    state.game_link_state = game_link.clone();
}

// ────────────────────────────────────────────────────────────────────────────
// Main menu bar  (exactly 16 system parameters)
// ────────────────────────────────────────────────────────────────────────────

fn draw_menu_bar(
    mut contexts:      EguiContexts,
    mut mode_ev:       EventWriter<RequestEditorMode>,
    mode:              Res<State<EditorMode>>,
    mut new_ev:        EventWriter<NewSceneRequest>,
    mut open_ev:       EventWriter<OpenSceneRequest>,
    mut save_ev:       EventWriter<SaveSceneRequest>,
    mut undo_ev:       EventWriter<UndoRequested>,
    mut redo_ev:       EventWriter<RedoRequested>,
    mut start_ev:      EventWriter<StartPie>,
    mut stop_ev:       EventWriter<StopPie>,
    mut pause_ev:      EventWriter<PausePie>,
    mut teleport_ev:   EventWriter<TeleportEditorCamera>,
    mut spawn_ev:      EventWriter<SpawnEntityRequest>,
    mut ui_state:      ResMut<EditorUiState>,
) {
    let ctx = contexts.ctx_mut();
    let current_mode = *mode.get();

    egui::TopBottomPanel::top("atlas_menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // ── File ─────────────────────────────────────────────────────
            ui.menu_button("File", |ui| {
                if ui.button("New Scene").clicked() {
                    new_ev.send(NewSceneRequest);
                    ui.close_menu();
                }
                if ui.button("Open Scene…").clicked() {
                    open_ev.send(OpenSceneRequest(OPEN_SCENE_PLACEHOLDER.into()));
                    ui.close_menu();
                }
                if ui.button("Save Scene").clicked() {
                    save_ev.send(SaveSceneRequest);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("📁  Open Project…").clicked() {
                    ui_state.open_project_requested = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });

            // ── Edit ─────────────────────────────────────────────────────
            ui.menu_button("Edit", |ui| {
                let undo_str = ui_state.undo_label
                    .as_deref()
                    .map(|l| format!("Undo \"{l}\"  Ctrl+Z"))
                    .unwrap_or_else(|| "Undo  Ctrl+Z".into());
                let redo_str = ui_state.redo_label
                    .as_deref()
                    .map(|l| format!("Redo \"{l}\"  Ctrl+Y"))
                    .unwrap_or_else(|| "Redo  Ctrl+Y".into());

                if ui
                    .add_enabled(ui_state.undo_label.is_some(), egui::Button::new(undo_str))
                    .clicked()
                {
                    undo_ev.send(UndoRequested);
                    ui.close_menu();
                }
                if ui
                    .add_enabled(ui_state.redo_label.is_some(), egui::Button::new(redo_str))
                    .clicked()
                {
                    redo_ev.send(RedoRequested);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("⧉ Duplicate  Ctrl+D").clicked() {
                    ui_state.duplicate_entity_requested = true;
                    ui.close_menu();
                }
                if ui.button("🗑 Delete  Del").clicked() {
                    ui_state.delete_entity_requested = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Project Settings…").clicked() {
                    ui.close_menu();
                }
            });

            // ── Create ────────────────────────────────────────────────────
            ui.menu_button("Create", |ui| {
                if ui.button("🔲  Blank Entity").clicked() {
                    spawn_ev.send(SpawnEntityRequest(PrimitiveKind::Blank));
                    ui.close_menu();
                }
                ui.separator();
                ui.label(egui::RichText::new("Primitives").weak().small());
                if ui.button("🟫  Cube").clicked() {
                    spawn_ev.send(SpawnEntityRequest(PrimitiveKind::Cube));
                    ui.close_menu();
                }
                if ui.button("🔵  Sphere").clicked() {
                    spawn_ev.send(SpawnEntityRequest(PrimitiveKind::Sphere));
                    ui.close_menu();
                }
                if ui.button("⬜  Plane").clicked() {
                    spawn_ev.send(SpawnEntityRequest(PrimitiveKind::Plane));
                    ui.close_menu();
                }
                ui.separator();
                ui.label(egui::RichText::new("Lights").weak().small());
                if ui.button("☀  Directional Light").clicked() {
                    spawn_ev.send(SpawnEntityRequest(PrimitiveKind::DirectionalLight));
                    ui.close_menu();
                }
                if ui.button("💡  Point Light").clicked() {
                    spawn_ev.send(SpawnEntityRequest(PrimitiveKind::PointLight));
                    ui.close_menu();
                }
            });

            // ── View ─────────────────────────────────────────────────────
            ui.menu_button("View", |ui| {
                ui.label(egui::RichText::new("Editor Camera").weak().small());
                ui.separator();
                if ui.button("🔭  Far Overview  [Home]").clicked() {
                    teleport_ev.send(TeleportEditorCamera::FarOverview);
                    ui.close_menu();
                }
                if ui.button("🎯  Near Overview  [End]").clicked() {
                    teleport_ev.send(TeleportEditorCamera::NearOverview);
                    ui.close_menu();
                }
                ui.separator();
                ui.label(egui::RichText::new("Panels").weak().small());
                ui.separator();

                let vis = ui_state.panel_visibility;
                let mut toggle_item = |ui: &mut egui::Ui, visible: bool, label: &str, which: PanelToggle| {
                    let text = if visible {
                        format!("✔  {label}")
                    } else {
                        format!("      {label}")
                    };
                    if ui.button(text).clicked() {
                        ui_state.pending_panel_toggle = Some(which);
                        ui.close_menu();
                    }
                };

                toggle_item(ui, vis.outliner,        "Outliner",        PanelToggle::Outliner);
                toggle_item(ui, vis.details,         "Details",         PanelToggle::Details);
                toggle_item(ui, vis.content_browser, "Content Browser", PanelToggle::ContentBrowser);
                toggle_item(ui, vis.output_log,      "Output Log",      PanelToggle::OutputLog);
                toggle_item(ui, vis.snap_toolbar,    "Snap Toolbar",    PanelToggle::SnapToolbar);

                ui.separator();
                let hist_label = if ui_state.undo_history_visible {
                    "✔  Undo History"
                } else {
                    "      Undo History"
                };
                if ui.button(hist_label).clicked() {
                    ui_state.undo_history_visible = !ui_state.undo_history_visible;
                    ui.close_menu();
                }
            });

            // ── Nova-Forge ────────────────────────────────────────────────
            ui.menu_button("Nova-Forge", |ui| {
                let is_linked = ui_state.game_link_state.is_linked();

                // Status header
                let (status_text, status_color) = if is_linked {
                    let path_str = ui_state.game_link_state.game_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("linked");
                    (format!("✅  Linked: {path_str}"), egui::Color32::from_rgb(100, 220, 100))
                } else {
                    ("⚠  Not linked".to_owned(), egui::Color32::from_rgb(220, 140, 60))
                };
                ui.label(egui::RichText::new(status_text).color(status_color).small());
                ui.separator();

                // Link / re-link button is always available
                let link_label = if is_linked {
                    "🔗  Re-link Game Repo…"
                } else {
                    "🔗  Link Game Repo…"
                };
                if ui.button(link_label).clicked() {
                    ui_state.link_requested = true;
                    ui.close_menu();
                }

                if is_linked {
                    if ui.button("🔓  Unlink").clicked() {
                        ui_state.unlink_requested = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("📤  Export to Game").clicked() {
                        ui_state.export_requested = true;
                        ui.close_menu();
                    }
                    if ui.button("▶  Launch Nova-Forge").clicked() {
                        ui_state.launch_game_requested = true;
                        ui.close_menu();
                    }
                }
            });

            // ── Play toolbar ─────────────────────────────────────────────
            ui.separator();
            match current_mode {
                EditorMode::Editing => {
                    if ui.button("▶  Play").clicked() {
                        start_ev.send(StartPie);
                    }
                    if ui.button("⏸  Simulate").clicked() {
                        mode_ev.send(RequestEditorMode(EditorMode::Simulating));
                    }
                }
                EditorMode::PlayingInEditor | EditorMode::Simulating => {
                    if ui.button("⏹  Stop").clicked() {
                        stop_ev.send(StopPie);
                    }
                    if ui.button("⏸  Pause").clicked() {
                        pause_ev.send(PausePie);
                    }
                }
                EditorMode::Paused => {
                    if ui.button("⏹  Stop").clicked() {
                        stop_ev.send(StopPie);
                    }
                    if ui.button("▶  Resume").clicked() {
                        mode_ev.send(RequestEditorMode(EditorMode::PlayingInEditor));
                    }
                }
            }

            // ── Right-aligned: Nova-Forge status + scene name ─────────────
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Scene dirty indicator (rightmost)
                let scene_text = if ui_state.scene_is_dirty {
                    format!("● {}", ui_state.active_scene_name)
                } else {
                    ui_state.active_scene_name.clone()
                };
                let scene_color = if ui_state.scene_is_dirty {
                    egui::Color32::from_rgb(255, 200, 80)
                } else {
                    egui::Color32::from_rgb(160, 160, 160)
                };
                ui.label(egui::RichText::new(scene_text).color(scene_color).small());

                ui.separator();

                // Nova-Forge link status indicator
                let (link_label, link_color) = if ui_state.game_link_state.is_linked() {
                    ("🎮 Linked", egui::Color32::from_rgb(100, 220, 100))
                } else {
                    ("🎮 Not linked", egui::Color32::from_rgb(220, 140, 60))
                };
                ui.label(egui::RichText::new(link_label).small().color(link_color));
            });
        });
    });
}

/// Dispatch Nova-Forge requests queued by `draw_menu_bar`.
///
/// Link/unlink open blocking OS dialogs (via `rfd`); export and launch are
/// forwarded as Bevy events so the actual work happens in `atlas_editor_export`.
fn dispatch_nova_forge_requests(
    mut ui_state:      ResMut<EditorUiState>,
    mut open_proj_ev:  EventWriter<OpenProjectRequest>,
    mut link_ev:       EventWriter<LinkGameRequest>,
    mut unlink_ev:     EventWriter<UnlinkGameRequest>,
    mut export_ev:     EventWriter<ExportToGameRequest>,
    mut launch_ev:     EventWriter<LaunchGameRequest>,
) {
    if ui_state.open_project_requested {
        ui_state.open_project_requested = false;
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Select project root folder")
            .pick_folder()
        {
            open_proj_ev.send(OpenProjectRequest(folder));
        }
    }
    if ui_state.link_requested {
        ui_state.link_requested = false;
        // rfd::FileDialog is a blocking modal OS dialog — the editor frame
        // freezes while the picker is open, which is expected behaviour.
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Select Nova-Forge repository root")
            .pick_folder()
        {
            link_ev.send(LinkGameRequest(folder));
        }
    }
    if ui_state.unlink_requested {
        ui_state.unlink_requested = false;
        unlink_ev.send(UnlinkGameRequest);
    }
    if ui_state.export_requested {
        ui_state.export_requested = false;
        export_ev.send(ExportToGameRequest);
    }
    if ui_state.launch_game_requested {
        ui_state.launch_game_requested = false;
        launch_ev.send(LaunchGameRequest);
    }
}

/// Keep the OS window title in sync with the linked Nova-Forge project name.
fn update_window_title(
    game_link:   Res<GameLinkState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !game_link.is_changed() { return; }
    let Ok(mut window) = windows.get_single_mut() else { return; };
    window.title = if let Some(proj_name) = game_link.game_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
    {
        format!("Nova-Forge Editor — {proj_name}")
    } else {
        "Nova-Forge Editor".to_owned()
    };
}

// ────────────────────────────────────────────────────────────────────────────
// Snap settings toolbar (below menu bar)
// ────────────────────────────────────────────────────────────────────────────

fn draw_snap_toolbar(
    mut contexts:  EguiContexts,
    mut snap:      ResMut<SnapSettings>,
    mut space:     ResMut<GizmoSpace>,
    mode:          Res<State<EditorMode>>,
    visibility:    Res<PanelVisibility>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }
    if !visibility.snap_toolbar {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("atlas_snap_toolbar")
        .exact_height(28.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // ── Gizmo space ───────────────────────────────────────────
                let space_label = match *space {
                    GizmoSpace::World => "🌐 World",
                    GizmoSpace::Local => "📦 Local",
                };
                if ui.small_button(space_label).clicked() {
                    *space = match *space {
                        GizmoSpace::World => GizmoSpace::Local,
                        GizmoSpace::Local => GizmoSpace::World,
                    };
                }

                ui.separator();
                ui.label(egui::RichText::new("Snap:").weak().small());

                // ── Translate ─────────────────────────────────────────────
                ui.checkbox(&mut snap.translate_enabled, "T");
                ui.add_enabled(
                    snap.translate_enabled,
                    egui::DragValue::new(&mut snap.translate_snap)
                        .speed(0.05)
                        .range(0.01..=100.0_f32)
                        .suffix(" m"),
                );

                ui.separator();

                // ── Rotate ────────────────────────────────────────────────
                ui.checkbox(&mut snap.rotate_enabled, "R");
                ui.add_enabled(
                    snap.rotate_enabled,
                    egui::DragValue::new(&mut snap.rotate_snap)
                        .speed(0.5)
                        .range(1.0..=180.0_f32)
                        .suffix("°"),
                );

                ui.separator();

                // ── Scale ─────────────────────────────────────────────────
                ui.checkbox(&mut snap.scale_enabled, "S");
                ui.add_enabled(
                    snap.scale_enabled,
                    egui::DragValue::new(&mut snap.scale_snap)
                        .speed(0.01)
                        .range(0.01..=10.0_f32)
                        .suffix("×"),
                );
            });
        });
}

// ────────────────────────────────────────────────────────────────────────────
// Undo History floating window
// ────────────────────────────────────────────────────────────────────────────

fn draw_undo_history_window(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<EditorUiState>,
    history:      Res<CommandHistory>,
    mut undo_ev:  EventWriter<UndoRequested>,
    mut redo_ev:  EventWriter<RedoRequested>,
) {
    if !ui_state.undo_history_visible {
        return;
    }

    let ctx = contexts.ctx_mut();
    let mut open = true;

    egui::Window::new("🕰 Undo History")
        .open(&mut open)
        .default_width(220.0)
        .resizable(true)
        .show(ctx, |ui| {
            let undo_labels = history.undo_stack_labels();
            let redo_labels = history.redo_stack_labels();

            // ── Redo stack (shown greyed — future actions on top) ─────────
            if !redo_labels.is_empty() {
                ui.label(
                    egui::RichText::new("── Redo stack ──")
                        .color(egui::Color32::from_rgb(140, 140, 140))
                        .small(),
                );
                egui::ScrollArea::vertical()
                    .id_source("redo_scroll")
                    .max_height(100.0)
                    .show(ui, |ui| {
                        for (i, lbl) in redo_labels.iter().enumerate() {
                            let text = egui::RichText::new(format!("↷  {lbl}"))
                                .color(egui::Color32::from_rgb(140, 200, 140))
                                .small();
                            if ui.selectable_label(i == 0, text).clicked() && i == 0 {
                                redo_ev.send(RedoRequested);
                            }
                        }
                    });
                ui.separator();
            }

            // ── Undo stack (most recent first) ────────────────────────────
            ui.label(
                egui::RichText::new("── Undo stack ──")
                    .color(egui::Color32::from_rgb(200, 200, 200))
                    .small(),
            );
            if undo_labels.is_empty() {
                ui.label(
                    egui::RichText::new("(nothing to undo)")
                        .color(egui::Color32::GRAY)
                        .small()
                        .italics(),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_source("undo_scroll")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for (i, lbl) in undo_labels.iter().enumerate() {
                            let color = if i == 0 {
                                egui::Color32::from_rgb(255, 220, 100)
                            } else {
                                egui::Color32::from_rgb(180, 180, 180)
                            };
                            let row = ui.selectable_label(
                                i == 0,
                                egui::RichText::new(format!("↩  {lbl}")).color(color).small(),
                            );
                            if row.clicked() && i == 0 {
                                undo_ev.send(UndoRequested);
                            }
                        }
                    });
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!undo_labels.is_empty(), egui::Button::new("↩ Undo"))
                    .clicked()
                {
                    undo_ev.send(UndoRequested);
                }
                if ui
                    .add_enabled(!redo_labels.is_empty(), egui::Button::new("↷ Redo"))
                    .clicked()
                {
                    redo_ev.send(RedoRequested);
                }
            });
        });

    if !open {
        ui_state.undo_history_visible = false;
    }
}


// ────────────────────────────────────────────────────────────────────────────
// Dispatch deferred UI action requests (flags set by draw_menu_bar)
// ────────────────────────────────────────────────────────────────────────────

fn dispatch_ui_requests(
    mut ui_state: ResMut<EditorUiState>,
    focused:      Res<FocusedEntity>,
    mut delete_ev: EventWriter<DeleteEntityRequest>,
    mut dup_ev:   EventWriter<DuplicateEntityRequest>,
    mut visibility: ResMut<PanelVisibility>,
) {
    if ui_state.delete_entity_requested {
        ui_state.delete_entity_requested = false;
        if let Some(entity) = focused.0 {
            delete_ev.send(DeleteEntityRequest(entity));
        }
    }
    if ui_state.duplicate_entity_requested {
        ui_state.duplicate_entity_requested = false;
        if let Some(entity) = focused.0 {
            dup_ev.send(DuplicateEntityRequest(entity));
        }
    }
    if let Some(kind) = ui_state.pending_panel_toggle.take() {
        match kind {
            PanelToggle::Outliner       => visibility.outliner        = !visibility.outliner,
            PanelToggle::Details        => visibility.details         = !visibility.details,
            PanelToggle::ContentBrowser => visibility.content_browser = !visibility.content_browser,
            PanelToggle::OutputLog      => visibility.output_log      = !visibility.output_log,
            PanelToggle::SnapToolbar    => visibility.snap_toolbar    = !visibility.snap_toolbar,
        }
    }
}
