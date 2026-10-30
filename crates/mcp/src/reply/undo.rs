//! The `undo` reply shape.

use serde::Serialize;
use tangibles_chess_notation::Fen;

/// Result of an `undo` request.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UndoReply {
    /// A half-move was undone.
    Done {
        /// Always `true` for this variant.
        undone: bool,
        /// Physical marker steps to restore the board.
        steps: Vec<String>,
        /// Position after the undo.
        fen: Fen,
    },
    /// There was nothing to undo.
    Empty {
        /// Always `false` for this variant.
        undone: bool,
        /// Why nothing happened.
        reason: String,
    },
}
