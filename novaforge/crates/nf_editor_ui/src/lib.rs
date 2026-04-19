//! `nf_editor_ui` — egui shell: main menu bar, toolbar, and docking layout.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use nf_editor_core::{EditorMode, RequestEditorMode};

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }
        app.add_systems(Update, draw_menu_bar);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Menu bar
// ────────────────────────────────────────────────────────────────────────────

fn draw_menu_bar(
    mut contexts: EguiContexts,
    mut mode_ev:  EventWriter<RequestEditorMode>,
    mode:         Res<State<EditorMode>>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("nf_menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Scene").clicked() { ui.close_menu(); }
                if ui.button("Open Scene…").clicked() { ui.close_menu(); }
                if ui.button("Save Scene").clicked() { ui.close_menu(); }
                ui.separator();
                if ui.button("Quit").clicked() { std::process::exit(0); }
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() { ui.close_menu(); }
                if ui.button("Redo").clicked() { ui.close_menu(); }
                ui.separator();
                if ui.button("Project Settings…").clicked() { ui.close_menu(); }
            });

            ui.menu_button("Window", |ui| {
                if ui.button("Outliner").clicked() { ui.close_menu(); }
                if ui.button("Details").clicked() { ui.close_menu(); }
                if ui.button("Content Browser").clicked() { ui.close_menu(); }
                if ui.button("Output Log").clicked() { ui.close_menu(); }
            });

            // Toolbar-style play controls inline in the bar
            ui.separator();
            if ui.button("▶  Play").clicked() {
                mode_ev.send(RequestEditorMode(EditorMode::PlayingInEditor));
            }
            if ui.button("⏸  Simulate").clicked() {
                mode_ev.send(RequestEditorMode(EditorMode::Simulating));
            }
        });
    });
}
