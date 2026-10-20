//! Game domain logic for tangibles-chess.
//!
//! Thin, fully tested layer over `shakmaty`: a `GameSession` owns the
//! position, move history, undo stack and phase state machine, while the
//! config enums describe how a game is set up (tangible mode, commit mode,
//! theme, orientation, opponent kinds).

mod board_orientation;
mod board_theme;
mod commit_mode;
mod draw_reason;
mod game_config;
mod game_error;
mod game_phase;
mod game_session;
mod move_outcome;
mod move_request;
mod opponent_kind;
mod saved_game;
mod tangible_mode;
mod undo_plan;
mod undo_step;

pub use board_orientation::BoardOrientation;
pub use board_theme::BoardTheme;
pub use commit_mode::CommitMode;
pub use draw_reason::DrawReason;
pub use game_config::GameConfig;
pub use game_error::GameError;
pub use game_phase::GamePhase;
pub use game_session::GameSession;
pub use move_outcome::MoveOutcome;
pub use move_request::MoveRequest;
pub use opponent_kind::OpponentKind;
pub use saved_game::SavedGame;
pub use tangible_mode::TangibleMode;
pub use undo_plan::UndoPlan;
pub use undo_step::UndoStep;
