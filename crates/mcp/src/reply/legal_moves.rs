//! The `legal_moves` reply shape.

use serde::Serialize;
use tangibles_chess_material::Side;
use tangibles_chess_notation::Uci;

/// Legal-move list (`legal_moves` tool).
#[derive(Debug, Serialize)]
pub struct LegalMovesReply {
    /// UCI moves, optionally narrowed to one origin square.
    pub legal_moves: Vec<Uci>,
    /// Side to move.
    pub turn: Side,
}
