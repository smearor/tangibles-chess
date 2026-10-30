//! `Replayable` on the controller: the finished-game replay cursor.

use tangibles_chess_core::GamePhase;
use tangibles_chess_core::Ply;

use super::game_controller::GameController;
use crate::replayable::Replayable;

impl Replayable for GameController {
    /// Enters replay mode on a finished game: dismisses the result
    /// overlay and lets the undo/redo buttons step through the recorded
    /// plies without mutating the session.
    fn enter_replay(&mut self) {
        if self.session.phase() != GamePhase::Finished || self.session.ply_count().is_start() {
            return;
        }
        if let Some(widget) = self.result_widget.take() {
            self.screen.pop_overlay(&widget);
        }
        self.replay_index = Some(self.session.ply_count());
        self.sync_scene();
    }

    /// Jumps the replay cursor straight to the position after
    /// `move_index`'s move (score-sheet click, 0-based ply ordinal).
    /// Enters replay mode implicitly on a finished game.
    fn replay_goto(&mut self, move_index: Ply) {
        if self.session.phase() != GamePhase::Finished {
            return;
        }
        if let Some(widget) = self.result_widget.take() {
            self.screen.pop_overlay(&widget);
        }
        self.replay_index = Some(move_index.next().min(self.session.ply_count()));
        self.sync_scene();
    }

    /// Moves the replay cursor by `delta` plies (clamped).
    fn replay_step(&mut self, delta: i64) {
        let Some(index) = self.replay_index else {
            return;
        };
        let next = index.step(delta).min(self.session.ply_count());
        self.replay_index = Some(next);
        self.sync_scene();
    }

    /// Leaves replay mode and returns to the result overlay.
    fn exit_replay(&mut self) {
        if self.replay_index.is_none() {
            return;
        }
        self.replay_index = None;
        self.result_shown = false;
        self.sync_scene();
    }
}
