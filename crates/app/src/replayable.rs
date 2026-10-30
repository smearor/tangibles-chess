//! Stepping through a finished game's recorded plies.

use tangibles_chess_moves::Ply;

/// Replay navigation on a finished game: a cursor walks the recorded
/// move list without mutating the session.
pub trait Replayable {
    /// Enters replay mode on a finished game.
    fn enter_replay(&mut self);

    /// Jumps the replay cursor straight to the position after
    /// `move_index`'s move (0-based ply ordinal).
    fn replay_goto(&mut self, move_index: Ply);

    /// Moves the replay cursor by `delta` plies (clamped).
    fn replay_step(&mut self, delta: i64);

    /// Leaves replay mode and returns to the result overlay.
    fn exit_replay(&mut self);
}
