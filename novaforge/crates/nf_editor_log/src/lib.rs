//! `nf_editor_log` — Output Log panel.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use nf_editor_core::EditorMode;

// ────────────────────────────────────────────────────────────────────────────
// Log record
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct LogRecord {
    pub level:   LogLevel,
    pub message: String,
}

// ────────────────────────────────────────────────────────────────────────────
// Log buffer resource
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct OutputLog {
    pub records: Vec<LogRecord>,
    /// Maximum number of records retained.
    pub capacity: usize,
}

impl OutputLog {
    pub fn push(&mut self, level: LogLevel, message: impl Into<String>) {
        self.records.push(LogRecord { level, message: message.into() });
        let cap = if self.capacity == 0 { 2_000 } else { self.capacity };
        if self.records.len() > cap {
            self.records.drain(0..self.records.len() - cap);
        }
    }

    pub fn info(&mut self, msg: impl Into<String>)    { self.push(LogLevel::Info,    msg); }
    pub fn warn(&mut self, msg: impl Into<String>)    { self.push(LogLevel::Warning, msg); }
    pub fn error(&mut self, msg: impl Into<String>)   { self.push(LogLevel::Error,   msg); }
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct EditorLogPlugin;

impl Plugin for EditorLogPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<OutputLog>()
            .add_systems(Update, draw_log_panel);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Panel
// ────────────────────────────────────────────────────────────────────────────

fn draw_log_panel(
    mut contexts: EguiContexts,
    log:          Res<OutputLog>,
    mode:         Res<State<EditorMode>>,
) {
    if *mode.get() != EditorMode::Editing {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::bottom("nf_output_log")
        .default_height(140.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Output Log");
                if ui.small_button("Clear").clicked() {
                    // Clearing handled via command in Phase 2.
                }
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for record in &log.records {
                        let (prefix, color) = match record.level {
                            LogLevel::Error   => ("❌ ", egui::Color32::RED),
                            LogLevel::Warning => ("⚠️ ", egui::Color32::YELLOW),
                            LogLevel::Info    => ("ℹ️ ", egui::Color32::GRAY),
                        };
                        ui.colored_label(color, format!("{prefix}{}", record.message));
                    }
                });
        });
}
