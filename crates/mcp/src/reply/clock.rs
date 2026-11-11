//! The chess-clock snapshot inside `game_state`.

use serde::Serialize;
use tangibles_chess_material::Side;

/// Live clock snapshot of a timed game (FR-ENG-3).
#[derive(Debug, Serialize)]
pub struct ClockReply {
    /// White's remaining budget in milliseconds.
    pub white_ms: u64,
    /// Black's remaining budget in milliseconds.
    pub black_ms: u64,
    /// The side currently being debited, if the countdown runs.
    pub running: Option<Side>,
    /// The side whose flag fell, if any.
    pub flagged: Option<Side>,
}
