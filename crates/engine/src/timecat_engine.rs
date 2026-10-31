//! The embedded engine: `timecat` (NNUE + negamax) running in-process,
//! so the engine opponent works without an external UCI binary.

use shakmaty::Chess;
use shakmaty::Move;
use shakmaty::uci::UciMove;
use tangibles_chess_notation::Fen;
use timecat::Engine;
use timecat::utils::extension_traits::ChessEngine as _;

use crate::EngineError;
use crate::chess_engine::ChessEngine;

/// `timecat` behind the [`ChessEngine`] seam: `set_fen` + `go depth`
/// per request, best move converted back through UCI notation.
#[derive(Default)]
pub struct TimecatEngine {
    /// The embedded engine instance.
    engine: Engine,
}

impl ChessEngine for TimecatEngine {
    fn new_game(&mut self) -> Result<(), EngineError> {
        self.engine.clear_hash();
        Ok(())
    }

    fn best_move(&mut self, position: &Chess, fen: &Fen, depth: u32) -> Result<Option<Move>, EngineError> {
        self.engine.set_fen(&fen.to_string()).map_err(|err| EngineError::Embedded(err.to_string()))?;
        // `Depth` is `i8` upstream - clamp rather than overflow.
        let search_depth = depth.min(i8::MAX as u32) as i8;
        let Some(best) = self.engine.search_depth_quiet(search_depth).get_best_move() else {
            return Ok(None);
        };
        best.to_string()
            .parse::<UciMove>()
            .map_err(|_| EngineError::IllegalBestMove(best.to_string()))?
            .to_move(position)
            .map(Some)
            .map_err(|_| EngineError::BestMoveNotLegal)
    }
}
