//! The command protocol between the MCP thread and the GTK main loop.

/// One command a tool can dispatch to the game controller.
///
/// The GTK side executes it against the live session and answers with a
/// JSON value on `reply`.
#[derive(Debug)]
pub enum CommandKind {
    /// Snapshot: phase, turn, FEN, SAN history, outcome, config.
    GameState,
    /// Legal moves in UCI notation; `square` narrows to one origin.
    LegalMoves {
        /// Optional origin square like `"e2"`.
        square: Option<String>,
    },
    /// Apply a move in UCI notation (`"e2e4"`, `"e7e8q"`).
    MakeMove {
        /// UCI move text.
        uci: String,
    },
    /// Undo the last half-move; returns the physical undo plan.
    Undo,
    /// Resign for `white` or `black`.
    Resign {
        /// `"white"` or `"black"`.
        color: String,
    },
    /// Agree a draw.
    AgreeDraw,
    /// Placement-gate status (tangible modes): placed/missing/foreign.
    PlacementStatus,
}

/// A command plus the channel back to the waiting tool call.
#[derive(Debug)]
pub struct GameRequest {
    /// What to execute.
    pub command: CommandKind,
    /// Reply channel; carries the JSON answer back to the tool.
    pub reply: tokio::sync::oneshot::Sender<serde_json::Value>,
}
