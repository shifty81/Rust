//! `nf_commands` — command-based undo/redo.  All editor mutations are expressed
//! as [`EditorCommand`] implementations so they can be reliably reversed.

use bevy::prelude::*;

// ────────────────────────────────────────────────────────────────────────────
// Command trait
// ────────────────────────────────────────────────────────────────────────────

/// Context passed to every command on apply / undo.
pub struct EditorCommandContext<'w> {
    pub world: &'w mut World,
}

/// A reversible editor action.
///
/// Implement this for every discrete mutation (move, rename, add component,
/// delete entity, etc.).  Use [`CommandHistory`] to execute and stack them.
pub trait EditorCommand: Send + Sync + 'static {
    /// Apply the command (do / redo path).
    fn apply(&mut self, ctx: &mut EditorCommandContext);
    /// Reverse the command (undo path).
    fn undo(&mut self, ctx: &mut EditorCommandContext);
    /// Human-readable label shown in the undo history panel.
    fn label(&self) -> &str;
}

// ────────────────────────────────────────────────────────────────────────────
// Command history resource
// ────────────────────────────────────────────────────────────────────────────

/// Manages the undo/redo stacks.
#[derive(Resource, Default)]
pub struct CommandHistory {
    undo_stack: Vec<Box<dyn EditorCommand>>,
    redo_stack: Vec<Box<dyn EditorCommand>>,
}

impl CommandHistory {
    /// Execute a command and push it onto the undo stack.
    /// Clears the redo stack (branching history is not supported).
    pub fn execute(&mut self, mut cmd: Box<dyn EditorCommand>, world: &mut World) {
        let mut ctx = EditorCommandContext { world };
        cmd.apply(&mut ctx);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
    }

    /// Undo the most recent command.
    pub fn undo(&mut self, world: &mut World) {
        if let Some(mut cmd) = self.undo_stack.pop() {
            let mut ctx = EditorCommandContext { world };
            cmd.undo(&mut ctx);
            self.redo_stack.push(cmd);
        }
    }

    /// Redo the most recently undone command.
    pub fn redo(&mut self, world: &mut World) {
        if let Some(mut cmd) = self.redo_stack.pop() {
            let mut ctx = EditorCommandContext { world };
            cmd.apply(&mut ctx);
            self.undo_stack.push(cmd);
        }
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.label())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.label())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────────────

/// Fired after any undo/redo so panels can refresh.
#[derive(Event)]
pub struct CommandHistoryChanged;

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct CommandHistoryPlugin;

impl Plugin for CommandHistoryPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CommandHistory>()
            .add_event::<CommandHistoryChanged>();
    }
}
