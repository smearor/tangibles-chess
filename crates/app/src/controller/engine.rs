//! The engine opponent: a worker thread owns the blocking
//! [`EngineClient`]; the GTK side sends positions and polls replies so
//! engine thinking never stalls the main loop.

use std::sync::mpsc;
use std::time::Duration;

use gtk4::glib;
use gtk4::glib::ControlFlow;
use shakmaty::Chess;
use shakmaty::Move;
use tangibles_chess_config::OpponentKind;
use tangibles_chess_engine::ChessEngine;
use tangibles_chess_engine::EngineClient;
use tangibles_chess_engine::TimecatEngine;
use tangibles_chess_material::Side;
use tangibles_chess_moves::GamePhase;
use tangibles_chess_moves::MoveRequest;
use tangibles_chess_notation::Fen;
use tracing::warn;

use super::game_controller::GameController;
use super::registry::with_controller;
use crate::playable::Playable;

/// Fixed search depth for the engine opponent (B2 baseline).
const ENGINE_DEPTH: u32 = 8;

/// Poll cadence for engine replies (ms).
const ENGINE_POLL_MS: u64 = 50;

/// One answer from the engine worker.
pub(super) enum EngineReply {
    /// The engine produced a move (`None` = `bestmove (none)`, e.g.
    /// terminal position).
    Move(Option<Move>),
    /// The engine session broke mid-game (lost process, bad reply).
    Failed(String),
}

impl GameController {
    /// Spawns the engine worker when either side is an engine opponent.
    /// The worker performs the `uci`/`isready` handshake off the main
    /// loop and then serves `position + go depth` requests.
    pub(super) fn spawn_engine_worker(&mut self) {
        let config = self.session.config();
        if config.white != OpponentKind::Engine && config.black != OpponentKind::Engine {
            return;
        }
        let (request_tx, request_rx) = mpsc::channel::<(Chess, Fen)>();
        let (reply_tx, reply_rx) = mpsc::channel::<EngineReply>();
        let path = self.settings.engine_path.clone();
        std::thread::spawn(move || {
            // `engine_path = "embedded"` selects the in-process engine;
            // an external binary is tried first and falls back to it.
            let mut engine: Box<dyn ChessEngine> = if path == "embedded" {
                Box::new(TimecatEngine::default())
            } else {
                match EngineClient::spawn(&path) {
                    Ok(client) => Box::new(client),
                    Err(err) => {
                        warn!("external engine at `{path}` unavailable ({err}) - falling back to embedded engine");
                        Box::new(TimecatEngine::default())
                    }
                }
            };
            let _ = engine.new_game();
            for (position, fen) in request_rx {
                let reply = match engine.best_move(&position, &fen, ENGINE_DEPTH) {
                    Ok(mv) => EngineReply::Move(mv),
                    Err(err) => EngineReply::Failed(err.to_string()),
                };
                if reply_tx.send(reply).is_err() {
                    return;
                }
            }
        });
        self.engine_tx = Some(request_tx);
        self.engine_rx = Some(reply_rx);
        self.start_engine_poll();
    }

    /// Requests a move whenever the side to move is engine-controlled.
    /// Called after every applied move and at `begin()` - the engine's
    /// reply lands via [`EngineReply`] polling and flows back through
    /// `apply_request` like any other move source.
    pub(super) fn maybe_engine_move(&mut self) {
        if self.engine_thinking || self.session.phase() != GamePhase::InProgress {
            return;
        }
        let turn = self.session.turn();
        let kind = match turn {
            Side::White => self.session.config().white,
            Side::Black => self.session.config().black,
        };
        if kind != OpponentKind::Engine {
            return;
        }
        let Some(tx) = &self.engine_tx else {
            return;
        };
        let fen = self.session.fen();
        if tx.send((self.session.position().clone(), fen.clone())).is_err() {
            return;
        }
        self.engine_thinking = true;
        self.engine_pending_fen = Some(fen);
        self.screen.set_instruction(turn, &self.l10n.msg("engine-thinking"));
    }

    /// Polls the engine reply channel on the GTK main loop. Stale
    /// replies (position moved on while thinking - undo, human move)
    /// are dropped and a fresh request is issued.
    fn start_engine_poll(&mut self) {
        glib::timeout_add_local(Duration::from_millis(ENGINE_POLL_MS), move || {
            with_controller(|controller| {
                let mut controller = controller.borrow_mut();
                let Some(rx) = &controller.engine_rx else {
                    return;
                };
                let Ok(reply) = rx.try_recv() else {
                    return;
                };
                controller.engine_thinking = false;
                match reply {
                    EngineReply::Move(Some(mv)) => {
                        let stale = controller.engine_pending_fen.as_ref() != Some(&controller.session.fen());
                        controller.engine_pending_fen = None;
                        if !stale && let Ok(request) = MoveRequest::try_from(&mv) {
                            controller.apply_request(request);
                        }
                        controller.maybe_engine_move();
                    }
                    EngineReply::Move(None) => {
                        controller.engine_pending_fen = None;
                    }
                    EngineReply::Failed(err) => {
                        warn!("engine opponent unavailable: {err}");
                        controller.engine_tx = None;
                        controller.engine_rx = None;
                        controller.engine_pending_fen = None;
                        controller.screen.show_message(&controller.l10n.msg("engine-unavailable"));
                    }
                }
            });
            ControlFlow::Continue
        });
    }
}
