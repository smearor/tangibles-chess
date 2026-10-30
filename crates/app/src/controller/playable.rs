//! `Playable` on the controller: taps, move application, game endings.

use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_config::TangibleMode;
use tangibles_chess_core::GamePhase;
use tangibles_chess_core::MoveOutcome;
use tangibles_chess_core::MoveRequest;
use tangibles_chess_material::Piece;
use tangibles_chess_material::Role;
use tangibles_chess_material::Side;
use tracing::debug;
use tracing::warn;

use super::game_controller::GameController;
use crate::playable::Playable;
use crate::sound_effect::SoundEffect;

impl Playable for GameController {
    /// Touch tap on a scene square (`NoTangibles` mode only).
    fn on_square(&mut self, square: Square) {
        if self.session.config().tangible_mode != TangibleMode::NoTangibles {
            return;
        }
        if self.session.phase() != GamePhase::InProgress {
            return;
        }
        if let Some(from) = self.selected {
            if square == from {
                return;
            }
            if self.session.legal_targets(from).contains(&square) {
                self.apply_request(MoveRequest::new(from, square));
                return;
            }
            let own = self
                .session
                .position()
                .board()
                .piece_at(square)
                .is_some_and(|piece| Side::from(piece.color) == self.session.turn());
            if !own {
                // Tapped an invalid destination for the selection -
                // flash the rejected square and keep the selection.
                self.flash_illegal(MoveRequest::new(from, square));
                return;
            }
            // Otherwise fall through and reselect the tapped own piece.
        }
        // Select own piece, otherwise clear.
        let own = self
            .session
            .position()
            .board()
            .piece_at(square)
            .is_some_and(|piece| Side::from(piece.color) == self.session.turn());
        self.selected = own.then_some(square);
        let targets = self.selected.map(|s| self.session.legal_targets(s)).unwrap_or_default();
        self.screen.surface.scene.update(|state| {
            state.selected = self.selected;
            state.legal_targets = targets;
        });
    }

    /// Validates and applies a move request, dispatching the outcome to
    /// the UI. Returns the session's outcome for remote callers.
    fn apply_request(&mut self, request: MoveRequest) -> MoveOutcome {
        let mover = self.session.turn();
        let victim = self.session.position().board().piece_at(request.to).map(Piece::from).or_else(|| {
            // En passant: the taken pawn is not on the target square.
            let moving = self.session.position().board().piece_at(request.from)?;
            (moving.role == shakmaty::Role::Pawn && request.from.file() != request.to.file()).then_some(Piece {
                side: mover.other(),
                role: Role::Pawn,
            })
        });
        let outcome = self.session.request_move(request);
        match &outcome {
            MoveOutcome::Applied(san) => {
                debug!("move applied: {san}");
                self.selected = None;
                self.last_move = Some((request.from, request.to));
                if self.session.position().is_check() {
                    self.sound.play(SoundEffect::Check);
                } else if san.as_str().contains('x') {
                    self.sound.play(SoundEffect::Capture);
                } else {
                    self.sound.play(SoundEffect::Move);
                }
                if let Some(victim) = victim.filter(|_| san.as_str().contains('x')) {
                    self.screen.show_message(&self.capture_notice(mover, victim, request.to));
                }
                self.sync_scene();
                self.autosave();
                if self.session.phase() == GamePhase::Finished {
                    self.sound.play(SoundEffect::End);
                }
            }
            MoveOutcome::NeedsPromotion { .. } => {
                self.pending_promotion = Some(request);
                self.show_promotion_picker();
            }
            MoveOutcome::Illegal(reason) => {
                warn!("illegal move rejected: {reason}");
                self.rejected_move = Some(request);
                self.screen.show_message_for(self.session.turn(), &self.l10n.msg(reason.l10n_key()));
                self.flash_illegal(request);
            }
        }
        outcome
    }

    /// Resign for `side`.
    fn resign(&mut self, side: Side) {
        if self.replay_index.is_some() {
            return;
        }
        self.session.resign(side);
        self.sync_scene();
        self.show_result();
    }

    /// Agree a draw.
    fn draw(&mut self) {
        if self.replay_index.is_some() {
            return;
        }
        self.session.agree_draw();
        self.sync_scene();
        self.show_result();
    }
}
