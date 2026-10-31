//! `Undoable` on the controller: session undo/redo or replay stepping.

use fluent::FluentArgs;
use tangibles_chess_config::TangibleMode;
use tangibles_chess_undo::UndoPlan;
use tangibles_chess_undo::UndoStep;

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
            self.show_undo_guide(&plan);
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
            self.show_undo_guide(&plan);
            self.last_move = None;
            self.sync_scene();
            self.autosave();
        }
    }
}

impl GameController {
    /// Localized one-line instruction for a physical `UndoStep`
    /// (`"move e4 back to e2"` / `"restore white pawn on c5"`).
    fn undo_step_text(&self, step: &UndoStep) -> String {
        let mut args = FluentArgs::new();
        match step {
            UndoStep::MoveBack { from, to } => {
                args.set("from", from.to_string());
                args.set("to", to.to_string());
                self.l10n.msg_args("undo-step-move", &args).to_string()
            }
            UndoStep::Restore { square, piece } => {
                let name = format!("{} {}", self.l10n.msg(piece.side.l10n_key()), self.l10n.msg(piece.role.l10n_key()));
                args.set("piece", name);
                args.set("square", square.to_string());
                self.l10n.msg_args("undo-step-restore", &args).to_string()
            }
        }
    }

    /// Shows the plan's rebuild steps as a status text and arms the
    /// `missing`/`foreign` guide markers. `NoTangibles` skips: the
    /// rendered board already matches the rewound position.
    fn show_undo_guide(&mut self, plan: &UndoPlan) {
        if self.session.config().tangible_mode == TangibleMode::NoTangibles {
            return;
        }
        let steps = plan.steps.iter().map(|step| self.undo_step_text(step)).collect::<Vec<_>>().join(" · ");
        if !steps.is_empty() {
            self.screen.show_message(&steps);
        }
        self.undo_guided = true;
        self.refresh_undo_guide();
    }

    /// Refreshes the guide markers from the live physical↔logical diff;
    /// clears flag and markers once the board matches the rewound
    /// position again.
    pub(super) fn refresh_undo_guide(&mut self) {
        if !self.undo_guided {
            return;
        }
        let status = self.gate.check(self.session.position(), &self.probe);
        self.screen.surface.scene.update(|state| {
            state.missing = status.missing.clone();
            state.foreign = status.unexpected.clone();
        });
        if status.is_consistent() {
            self.undo_guided = false;
            self.screen.show_message(&self.l10n.msg("game-undo-done"));
        }
    }
}
