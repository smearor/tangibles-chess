//! Scene/panel synchronization and the presentation helpers it needs.

use fluent::FluentArgs;
use shakmaty::Color;
use shakmaty::KnownOutcome;
use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_config::GameConfig;
use tangibles_chess_core::DrawReason;
use tangibles_chess_core::GamePhase;
use tangibles_chess_core::MoveRequest;
use tangibles_chess_core::Ply;
use tangibles_chess_icons::icon_names;
use tangibles_chess_material::Piece;
use tangibles_chess_material::Side;
use tangibles_chess_tangibles::TableEdge;

use super::game_controller::GameController;
use super::registry::with_controller;
use crate::playable::Playable;
use crate::ui::chess_scene::FlashRole;
use crate::ui::chess_scene::IllegalFlash;
use crate::ui::game_result::GameResult;
use crate::ui::promotion_picker::PromotionPicker;

/// How long a rejected move's squares flash red (ms).
const ILLEGAL_FLASH_MS: u64 = 800;

impl GameController {
    /// Syncs scene + panels + sheets with the session state.
    pub(super) fn sync_scene(&mut self) {
        // In replay mode the scene shows the cursor's historical
        // position instead of the live one.
        let position = self
            .replay_index
            .and_then(|index| self.session.position_at(index))
            .unwrap_or_else(|| self.session.position())
            .clone();
        let check = position.is_check().then(|| position.board().king_of(position.turn()));
        let last_move = (self.replay_index.is_none() && self.session.san_history().len().gt(&0))
            .then(|| self.last_move_squares())
            .flatten();
        // Captured pieces park in canonical order: the n-th occupied
        // parking cell (west first, then east) renders the n-th
        // captured piece.
        let mut captured = self.session.captured_pieces().into_iter();
        self.parked_west.assign_pieces(&mut captured);
        self.parked_east.assign_pieces(&mut captured);
        let cap_west = self.parked_west;
        let cap_east = self.parked_east;
        self.screen.surface.scene.update(|state| {
            state.position = Some(position.clone());
            state.check = check.flatten();
            state.last_move = last_move;
            state.selected = self.selected;
            state.legal_targets = self.selected.map(|s| self.session.legal_targets(s)).unwrap_or_default();
            state.cap_west = cap_west;
            state.cap_east = cap_east;
        });
        let sans = self.session.san_history();
        self.screen.surface.white_sheet.set_moves(sans);
        self.screen.surface.black_sheet.set_moves(sans);
        // In replay mode the cursor's producing move is highlighted:
        // position after ply `n` comes from move `n - 1`.
        let active_ply = self.replay_index.and_then(Ply::prev);
        self.screen.surface.white_sheet.set_active_ply(active_ply);
        self.screen.surface.black_sheet.set_active_ply(active_ply);
        let (turn_text, active) = match position.turn() {
            Color::White => (self.l10n.msg("game-white-to-move"), true),
            Color::Black => (self.l10n.msg("game-black-to-move"), false),
        };
        if self.session.phase() == GamePhase::Finished {
            if let Some(index) = self.replay_index {
                let label = format!("{} {}/{}", self.l10n.msg("game-replay"), index, self.session.ply_count());
                self.screen.set_instruction(Side::White, &label);
                self.screen.set_instruction(Side::Black, &label);
            } else {
                self.show_result();
            }
        } else if self.session.phase() == GamePhase::AwaitingPlacement {
            self.update_placement();
        } else {
            self.screen.set_instruction(Side::White, if active { &turn_text } else { "" });
            self.screen.set_instruction(Side::Black, if !active { &turn_text } else { "" });
        }
        self.screen.surface.white_panel.set_active(active);
        self.screen.surface.black_panel.set_active(!active);
        let (can_undo, can_redo) = match self.replay_index {
            // In replay the buttons walk the cursor instead of the undo
            // stack.
            Some(index) => (!index.is_start(), index < self.session.ply_count()),
            None => (self.session.can_undo(), self.session.can_redo()),
        };
        self.screen.surface.white_panel.set_undo_enabled(can_undo);
        self.screen.surface.white_panel.set_redo_enabled(can_redo);
        self.screen.surface.black_panel.set_undo_enabled(can_undo);
        self.screen.surface.black_panel.set_redo_enabled(can_redo);
        let play_enabled = self.replay_index.is_none() && self.session.phase() == GamePhase::InProgress;
        self.screen.surface.white_panel.set_play_enabled(play_enabled);
        self.screen.surface.black_panel.set_play_enabled(play_enabled);
        self.screen.surface.white_panel.set_replay_mode(self.replay_index.is_some());
        self.screen.surface.black_panel.set_replay_mode(self.replay_index.is_some());
        self.notify_mcp_state();
    }

    /// Squares of the last applied move (tracked in `apply_request`).
    fn last_move_squares(&self) -> Option<(Square, Square)> {
        self.last_move
    }

    /// Presents the game result once: a modal, dual-facing overlay
    /// (outcome icon + confirm button leading to the setup dialog).
    pub(super) fn show_result(&mut self) {
        if self.result_shown {
            return;
        }
        let (text, icon) = match self.session.outcome() {
            Some(KnownOutcome::Decisive { winner }) => match winner {
                Color::White => (self.l10n.msg("game-white-wins"), icon_names::TCH_TROPHY_SYMBOLIC),
                Color::Black => (self.l10n.msg("game-black-wins"), icon_names::TCH_TROPHY_SYMBOLIC),
            },
            Some(KnownOutcome::Draw) => {
                // The overlay states the draw's rule (stalemate,
                // repetition, material, agreement, ...) when known.
                let reason = self.session.draw_reason().map(|reason| match reason {
                    DrawReason::Stalemate => self.l10n.msg("game-draw-stalemate"),
                    DrawReason::InsufficientMaterial => self.l10n.msg("game-draw-insufficient"),
                    DrawReason::ThreefoldRepetition => self.l10n.msg("game-draw-threefold"),
                    DrawReason::FivefoldRepetition => self.l10n.msg("game-draw-fivefold"),
                    DrawReason::FiftyMoves => self.l10n.msg("game-draw-fifty-moves"),
                    DrawReason::SeventyFiveMoves => self.l10n.msg("game-draw-seventy-five-moves"),
                    DrawReason::Agreement => self.l10n.msg("game-draw-agreement"),
                });
                let mut text = self.l10n.msg("game-drawn");
                if let Some(reason) = reason {
                    text = format!("{text} — {reason}");
                }
                (text, icon_names::TCH_HANDSHAKE_SYMBOLIC)
            }
            None => return,
        };
        self.result_shown = true;
        let confirm = self.l10n.msg("game-new");
        let replay = self.l10n.msg("game-replay");
        let quit = self.l10n.msg("setup-quit");
        let result = GameResult::new(&text, icon, &confirm, Some(&replay), &quit);
        self.screen.push_overlay(&result.widget);
        self.result_widget = Some(result.widget);
    }

    /// Status-line notice for a capture, shown on both sides:
    /// "{color} captures {piece} on {square}".
    pub(super) fn capture_notice(&self, mover: Side, victim: Piece, square: Square) -> String {
        let color_key = mover.l10n_key();
        let piece_key = victim.role.l10n_key();
        let mut args = FluentArgs::new();
        args.set("color", self.l10n.msg(color_key).to_string());
        args.set("piece", self.l10n.msg(piece_key).to_string());
        args.set("square", square.to_string());
        self.l10n.msg_args("capture-notice", &args).to_string()
    }

    /// Briefly flashes the rejected move's target square red - plus,
    /// when the naive move would have left the own king in check, the
    /// threatened king and its attackers.
    pub(super) fn flash_illegal(&mut self, request: MoveRequest) {
        self.illegal_seq += 1;
        let seq = self.illegal_seq;
        let mut squares = vec![(request.to, FlashRole::Target)];
        if let Some(projection) = self.session.projected_check(&request) {
            squares.push((projection.king, FlashRole::King));
            squares.extend(projection.attackers.iter().map(|attacker| (*attacker, FlashRole::Attacker)));
        }
        self.screen.surface.scene.update(move |state| {
            state.illegal = Some(IllegalFlash { squares, generation: seq });
        });
        let scene = self.screen.surface.scene.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(ILLEGAL_FLASH_MS), move || {
            scene.update(|state| {
                if state.illegal.as_ref().is_some_and(|flash| flash.generation == seq) {
                    state.illegal = None;
                }
            });
        });
    }

    /// Shows the promotion picker facing the moving player.
    pub(super) fn show_promotion_picker(&self) {
        let mover = self.session.turn();
        let facing_north = self.geometry.side_edge(mover) == TableEdge::North;
        let picker = PromotionPicker::new(
            mover,
            facing_north,
            move |role| {
                with_controller(|controller| {
                    let mut controller = controller.borrow_mut();
                    let Some(request) = controller.pending_promotion.take() else {
                        return;
                    };
                    if let Some(widget) = controller.promotion_widget.borrow_mut().take() {
                        controller.screen.pop_overlay(&widget);
                    }
                    controller.apply_request(MoveRequest {
                        from: request.from,
                        to: request.to,
                        promotion: Some(role),
                    });
                });
            },
            &self.l10n,
        );
        self.screen.push_overlay(&picker.widget);
        *self.promotion_widget.borrow_mut() = Some(picker.widget.clone());
    }

    /// Applies the setup options that are safe to change mid-game
    /// (board theme, sound, undo) to the running session. Structural
    /// options (input mode, commit mode, handshake, orientation,
    /// opponents) only take effect on the next game - they live in
    /// daemon registrations, field mapping and session setup.
    pub fn apply_live_config(&mut self, config: &GameConfig) {
        {
            let session_config = self.session.config_mut();
            session_config.theme = config.theme;
            session_config.sound_enabled = config.sound_enabled;
            session_config.allow_undo = config.allow_undo;
        }
        self.settings.sound_enabled = config.sound_enabled;
        self.settings.save();
        self.sound.set_enabled(config.sound_enabled);
        self.screen.surface.scene.update(|state| state.theme = config.theme);
        self.sync_scene();
        self.autosave();
    }

    /// Writes the autosave file.
    pub(super) fn autosave(&self) {
        let dir = glib::user_data_dir().join("tangibles-chess");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(text) = self.session.to_saved().to_toml() {
            let _ = std::fs::write(dir.join("autosave.toml"), text);
        }
    }
}
