//! Application-level errors a command can report.

use serde::Serialize;
use tangibles_chess_notation::Uci;
use thiserror::Error;

/// Why a command failed on the controller side. Serializes as
/// `{"error": "<snake_case kind>"}` plus the variant's payload fields;
/// [`Display`](std::fmt::Display) carries the matching prose for logs
/// and tool error texts.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum CommandError {
    /// No game session is running (setup screen, before `begin`).
    #[error("no active game")]
    NoActiveGame,
    /// The UCI move text could not be applied to the position.
    #[error("invalid move '{uci}'")]
    InvalidMove {
        /// The rejected UCI notation.
        uci: Uci,
    },
    /// The window is not realized; there is no texture to snapshot.
    #[error("window is not realized")]
    WindowNotRealized,
}
