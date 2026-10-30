//! Game session - lifecycle, state and persistence for
//! tangibles-chess.
//!
//! Thin, fully tested layer over `shakmaty`: a `GameSession` owns the
//! position, move history, undo stack and phase state machine, while
//! `SavedGame` persists setup plus move history for autosave/resume.

mod draw_reason;
mod game_error;
mod game_outcome;
mod game_session;
mod saved_game;

pub use draw_reason::DrawReason;
pub use game_error::GameError;
pub use game_outcome::GameOutcome;
pub use game_session::GameSession;
pub use saved_game::SavedGame;
