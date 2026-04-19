//! `nf_editor_ui` — egui shell: main menu bar, toolbar, and docking layout.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use nf_editor_core::{EditorMode, RequestEditorMode};
use nf_editor_scene::{NewSceneRequest, OpenSceneRequest, SaveSceneRequest};
use nf_editor_play::{StartPie, StopPie, PausePie};
use nf_editor_viewport::TeleportEditorCamera;
use nf_commands::{UndoRequested, RedoRequested, CommandHistory};

/// Placeholder path used when no file dialog is available yet.
const OPEN_SCENE_PLACEHOLDER: &str = "project/Scenes/untitled.nfscene";



pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }
        app.add_systems(Update, (keyboard_shortcuts, draw_menu_bar).chain());
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Keyboard shortcuts
// ────────────────────────────────────────────────────────────────────────────

fn keyboard_shortcuts(
    keys:       Res<ButtonInput<KeyCode>>,
    mut undo_ev: EventWriter<UndoRequested>,
    mut redo_ev: EventWriter<RedoRequested>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    if ctrl && !shift && keys.just_pressed(KeyCode::KeyZ) {
        undo_ev.send(UndoRequested);
    }
    // Ctrl+Y or Ctrl+Shift+Z for redo
    if ctrl && (keys.just_pressed(KeyCode::KeyY) || (shift && keys.just_pressed(KeyCode::KeyZ))) {
        redo_ev.send(RedoRequested);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Menu bar
// ────────────────────────────────────────────────────────────────────────────

fn draw_menu_bar(
    mut contexts:    EguiContexts,
    mut mode_ev:     EventWriter<RequestEditorMode>,
    mode:            Res<State<EditorMode>>,
    mut new_ev:      EventWriter<NewSceneRequest>,
    mut open_ev:     EventWriter<OpenSceneRequest>,
    mut save_ev:     EventWriter<SaveSceneRequest>,
    mut undo_ev:     EventWriter<UndoRequested>,
    mut redo_ev:     EventWriter<RedoRequested>,
    mut start_ev:    EventWriter<StartPie>,
    mut stop_ev:     EventWriter<StopPie>,
    mut pause_ev:    EventWriter<PausePie>,
    mut teleport_ev: EventWriter<TeleportEditorCamera>,
    history:         Res<CommandHistory>,
) {
    let ctx = contexts.ctx_mut();
    let current_mode = *mode.get();

    egui::TopBottomPanel::top("nf_menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // ── File ─────────────────────────────────────────────────────
            ui.menu_button("File", |ui| {
                if ui.button("New Scene").clicked() {
                    new_ev.send(NewSceneRequest);
                    ui.close_menu();
                }
                if ui.button("Open Scene…").clicked() {
                    // Placeholder: real file dialog added in Phase 3.
                    open_ev.send(OpenSceneRequest(OPEN_SCENE_PLACEHOLDER.into()));
                    ui.close_menu();
                }
                if ui.button("Save Scene").clicked() {
                    save_ev.send(SaveSceneRequest);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });

            // ── Edit ─────────────────────────────────────────────────────
            ui.menu_button("Edit", |ui| {
                let undo_label = history
                    .undo_label()
                    .map(|l| format!("Undo \"{l}\""))
                    .unwrap_or_else(|| "Undo".into());
                let redo_label = history
                    .redo_label()
                    .map(|l| format!("Redo \"{l}\""))
                    .unwrap_or_else(|| "Redo".into());

                if ui
                    .add_enabled(history.undo_label().is_some(), egui::Button::new(undo_label))
                    .clicked()
                {
                    undo_ev.send(UndoRequested);
                    ui.close_menu();
                }
                if ui
                    .add_enabled(history.redo_label().is_some(), egui::Button::new(redo_label))
                    .clicked()
                {
                    redo_ev.send(RedoRequested);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Project Settings…").clicked() {
                    ui.close_menu();
                }
            });

            // ── View ─────────────────────────────────────────────────────
            ui.menu_button("View", |ui| {
                ui.label(egui::RichText::new("Editor Camera").weak().small());
                ui.separator();
                if ui.button("🌌  Solar System Overview  [Home]").clicked() {
                    teleport_ev.send(TeleportEditorCamera::SolarSystem);
                    ui.close_menu();
                }
                if ui.button("🌍  Planet Surface Overview  [End]").clicked() {
                    teleport_ev.send(TeleportEditorCamera::PlanetSurface);
                    ui.close_menu();
                }
                ui.separator();
                ui.label(egui::RichText::new("Panels").weak().small());
                ui.separator();
                if ui.button("Outliner").clicked() { ui.close_menu(); }
                if ui.button("Details").clicked() { ui.close_menu(); }
                if ui.button("Content Browser").clicked() { ui.close_menu(); }
                if ui.button("Output Log").clicked() { ui.close_menu(); }
                if ui.button("🌍 World Settings").clicked() { ui.close_menu(); }
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
        });
    });
}
