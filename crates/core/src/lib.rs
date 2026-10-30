//! Game domain logic for tangibles-chess.
//!
//! Thin, fully tested layer over `shakmaty`: a `GameSession` owns the
//! position, move history, undo stack and phase state machine, while the
//! config enums describe how a game is set up (tangible mode, commit mode,
//! theme, orientation, opponent kinds).

mod moves;
mod session;
mod undo;

pub use moves::CheckProjection;
pub use moves::IllegalReason;
pub use moves::MoveOutcome;
pub use moves::MoveRequest;
pub use moves::PlayRejection;
pub use moves::Ply;
pub use session::DrawReason;
pub use session::GameError;
pub use session::GameOutcome;
pub use session::GamePhase;
pub use session::GameSession;
pub use session::SavedGame;
pub use tangibles_chess_config::BoardTheme;
pub use tangibles_chess_config::CommitMode;
pub use tangibles_chess_config::GameConfig;
pub use tangibles_chess_config::OpponentKind;
pub use tangibles_chess_config::TangibleMode;
pub use tangibles_chess_geometry::BoardOrientation;
pub use tangibles_chess_geometry::FieldIndex;
pub use tangibles_chess_geometry::SceneIndex;
pub use undo::UndoPlan;
pub use undo::UndoStep;
