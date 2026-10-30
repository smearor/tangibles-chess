use crate::GamePhase;
use crate::Ply;
use crate::Uci;
use thiserror::Error;

/// Errors of the game domain layer.
#[derive(Debug, Error)]
pub enum GameError {
    /// A FEN string failed to parse or describe a valid position.
    #[error("invalid FEN: {0}")]
    InvalidFen(String),
    /// A saved game could not be serialized or deserialized.
    #[error("saved game error: {0}")]
    SavedGame(String),
    /// A stored move failed to replay onto its position.
    #[error("corrupt saved game: move {move_index} `{uci}` is not legal: {reason}")]
    CorruptSave {
        /// Ordinal of the offending move in the saved move list.
        move_index: Ply,
        /// The UCI move string that failed.
        uci: Uci,
        /// Underlying error message.
        reason: String,
    },
    /// A requested action is not valid in the current phase.
    #[error("action not allowed in phase {0:?}")]
    WrongPhase(GamePhase),
    /// I/O error while reading or writing the autosave file.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
