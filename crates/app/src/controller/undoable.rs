//! `Undoable` on the controller: session undo/redo or replay stepping.

use tracing::debug;

use super::game_controller::GameController;
use crate::replayable::Replayable;
use crate::undoable::Undoable;

impl Undoable for GameController {
    /// Steps the replay cursor back, or reverts the last half-move.
    fn undo(&mut self) {
        if self.replay_index.is_some() {
            self.replay_step(-1);
            return;
        }
        if let Some(plan) = self.session.undo() {
            for step in &plan.steps {
                debug!("undo step: {step:?}");
            }
            self.last_move = None;
            self.sync_scene();
            self.autosave();
        }
    }

    /// Steps the replay cursor forward, or re-applies an undone move.
    fn redo(&mut self) {
        if self.replay_index.is_some() {
            self.replay_step(1);
            return;
        }
        if let Some(plan) = self.session.redo() {
            for step in &plan.steps {
                debug!("redo step: {step:?}");
            }
            self.last_move = None;
            self.sync_scene();
            self.autosave();
        }
    }
}
