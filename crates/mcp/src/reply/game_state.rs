//! The `game_state` reply shape.

use serde::Serialize;
use tangibles_chess_core::DrawReason;
use tangibles_chess_core::Fen;
use tangibles_chess_core::GameOutcome;
use tangibles_chess_core::GamePhase;
use tangibles_chess_core::San;
use tangibles_chess_core::Side;
use tangibles_chess_core::TangibleMode;
use tangibles_chess_core::Uci;

/// Snapshot of the running game (`game_state` tool).
#[derive(Debug, Serialize)]
pub struct GameStateReply {
    /// Current game phase.
    pub phase: GamePhase,
    /// Side to move.
    pub turn: Side,
    /// Position FEN.
    pub fen: Fen,
    /// SAN moves played so far.
    pub san_history: Vec<San>,
    /// UCI moves played so far (for replay/verification).
    pub uci_history: Vec<Uci>,
    /// Terminal outcome, if the game is over.
    pub outcome: Option<GameOutcome>,
    /// Active tangible mode.
    pub tangible_mode: TangibleMode,
    /// Draws claimable in the current position.
    pub claimable_draws: Vec<DrawReason>,
}
