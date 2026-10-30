//! The `make_move` reply shape.

use serde::Serialize;
use tangibles_chess_notation::Fen;
use tangibles_chess_notation::San;
use tangibles_chess_notation::SquareName;

/// Result of a `make_move` request.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MakeMoveReply {
    /// The move was legal and applied.
    Applied {
        /// SAN of the applied move.
        applied: San,
        /// Position after the move.
        fen: Fen,
    },
    /// A promotion role must be supplied and the move retried.
    NeedsPromotion {
        /// Promotion target square.
        needs_promotion: SquareName,
        /// How to retry the request.
        hint: String,
    },
    /// The move was rejected.
    Illegal {
        /// Why it was rejected.
        illegal: String,
    },
}
