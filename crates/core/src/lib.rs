//! Game domain logic for tangibles-chess.
//!
//! Thin, fully tested layer over `shakmaty`: a `GameSession` owns the
//! position, move history, undo stack and phase state machine, while the
//! config enums describe how a game is set up (tangible mode, commit mode,
//! theme, orientation, opponent kinds).

mod config;
mod geometry;
mod moves;
mod notation;
mod session;
mod undo;

pub use config::BoardTheme;
pub use config::CommitMode;
pub use config::GameConfig;
pub use config::OpponentKind;
pub use config::TangibleMode;
pub use geometry::BoardOrientation;
pub use geometry::FieldIndex;
pub use geometry::SceneIndex;
pub use moves::IllegalReason;
pub use moves::MoveOutcome;
pub use moves::MoveRequest;
pub use notation::Fen;
pub use notation::San;
pub use notation::Uci;
pub use session::CheckProjection;
pub use session::DrawReason;
pub use session::GameError;
pub use session::GamePhase;
pub use session::GameSession;
pub use session::SavedGame;
pub use session::Side;
pub use undo::UndoPlan;
pub use undo::UndoStep;
