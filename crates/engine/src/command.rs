//! A UCI protocol command line sent to the engine.

use tangibles_chess_notation::Fen;

/// One command line of the UCI (Universal Chess Interface) protocol.
/// [`Display`](std::fmt::Display) renders the wire form.
#[derive(Debug)]
pub enum EngineCommand {
    /// Enter UCI mode (`uci`).
    Uci,
    /// Ping; the engine answers `readyok` (`isready`).
    IsReady,
    /// Reset for a fresh game (`ucinewgame`).
    UciNewGame,
    /// Set the position to `fen` (`position fen <fen>`).
    Position {
        /// FEN of the position to analyze.
        fen: Fen,
    },
    /// Search at fixed depth (`go depth <n>`).
    Go {
        /// Search depth in plies.
        depth: u32,
    },
    /// Abort the running search (`stop`).
    Stop,
    /// End the engine session (`quit`).
    Quit,
}

impl std::fmt::Display for EngineCommand {
    /// The wire command line (no trailing newline - `send` adds it).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uci => f.write_str("uci"),
            Self::IsReady => f.write_str("isready"),
            Self::UciNewGame => f.write_str("ucinewgame"),
            Self::Position { fen } => write!(f, "position fen {fen}"),
            Self::Go { depth } => write!(f, "go depth {depth}"),
            Self::Stop => f.write_str("stop"),
            Self::Quit => f.write_str("quit"),
        }
    }
}
