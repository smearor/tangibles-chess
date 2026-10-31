use shakmaty::Chess;
use shakmaty::Move;
use tangibles_chess_notation::Fen;

use crate::EngineError;

/// A chess move generator - the seam between "how a move is found" and
/// how the app consumes it.
///
/// Implementations cover the process-backed UCI client
/// ([`crate::EngineClient`]) and any embedded engine (a library with a
/// blocking `best_move`); both run on the controller's engine worker
/// thread, so blocking is fine.
pub trait ChessEngine: Send {
    /// Starts a fresh game (clears any cached state).
    fn new_game(&mut self) -> Result<(), EngineError>;

    /// Computes the engine's move for `position` at `depth`; `fen` is
    /// the wire form for engines that need it (`position fen ...`).
    /// `Ok(None)` when there is no move (terminal position).
    fn best_move(&mut self, position: &Chess, fen: &Fen, depth: u32) -> Result<Option<Move>, EngineError>;
}
