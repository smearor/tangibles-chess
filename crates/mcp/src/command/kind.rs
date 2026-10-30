//! The command protocol between the MCP thread and the GTK main loop.

use shakmaty::Square;
use tangibles_chess_material::Side;
use tangibles_chess_notation::Uci;

/// One command a tool can dispatch to the game controller.
///
/// The GTK side executes it against the live session and answers with a
/// JSON value on `reply`. Arguments arrive pre-validated: the tool
/// boundary rejects malformed input before a command is built.
#[derive(Debug)]
pub enum CommandKind {
    /// Snapshot: phase, turn, FEN, SAN history, outcome, config.
    GameState,
    /// Legal moves in UCI notation; `square` narrows to one origin.
    LegalMoves {
        /// Optional origin square like `"e2"`.
        square: Option<Square>,
    },
    /// Apply a move in UCI notation (`"e2e4"`, `"e7e8q"`).
    MakeMove {
        /// UCI move text.
        uci: Uci,
    },
    /// Undo the last half-move; returns the physical undo plan.
    Undo,
    /// Resign for `white` or `black`.
    Resign {
        /// The resigning side.
        color: Side,
    },
    /// Agree a draw.
    AgreeDraw,
    /// Placement-gate status (tangible modes): placed/missing/foreign.
    PlacementStatus,
    /// Window screenshot as a PNG (base64 in the JSON reply).
    Screenshot,
}
