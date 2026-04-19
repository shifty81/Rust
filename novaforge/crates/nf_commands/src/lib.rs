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

/// Request the command history to perform an undo.
#[derive(Event)]
pub struct UndoRequested;

/// Request the command history to perform a redo.
#[derive(Event)]
pub struct RedoRequested;

// ────────────────────────────────────────────────────────────────────────────
// Cursor resource for exclusive undo/redo system
// ────────────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct UndoRedoCursor {
    undo: bevy::ecs::event::ManualEventReader<UndoRequested>,
    redo: bevy::ecs::event::ManualEventReader<RedoRequested>,
}

// ────────────────────────────────────────────────────────────────────────────
// Plugin
// ────────────────────────────────────────────────────────────────────────────

pub struct CommandHistoryPlugin;

impl Plugin for CommandHistoryPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CommandHistory>()
            .init_resource::<UndoRedoCursor>()
            .add_event::<CommandHistoryChanged>()
            .add_event::<UndoRequested>()
            .add_event::<RedoRequested>()
            .add_systems(Update, apply_undo_redo);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Exclusive undo/redo system
// ────────────────────────────────────────────────────────────────────────────

/// Drain all pending events of type `E` from the cursor and return the count.
fn drain_events<E: Event>(world: &mut World, cursor: &mut bevy::ecs::event::ManualEventReader<E>) -> usize {
    world.resource_scope(|_world, events: Mut<Events<E>>| cursor.read(&*events).count())
}

fn apply_undo_redo(world: &mut World) {
    let undo_count = world.resource_scope(|world, mut cursor: Mut<UndoRedoCursor>| {
        drain_events::<UndoRequested>(world, &mut cursor.undo)
    });

    let redo_count = world.resource_scope(|world, mut cursor: Mut<UndoRedoCursor>| {
        drain_events::<RedoRequested>(world, &mut cursor.redo)
    });

    for _ in 0..undo_count {
        world.resource_scope(|world, mut history: Mut<CommandHistory>| {
            history.undo(world);
        });
        world.send_event(CommandHistoryChanged);
    }

    for _ in 0..redo_count {
        world.resource_scope(|world, mut history: Mut<CommandHistory>| {
            history.redo(world);
        });
        world.send_event(CommandHistoryChanged);
    }
}
