//! `Position::play` rejecting a candidate move.

use shakmaty::Move;
use tangibles_chess_notation::Uci;

/// `Position::play` rejected a validated candidate move - the
/// domain-side mirror of `shakmaty::PlayError`, keeping the rejected
/// move and dropping the position (the caller holds it anyway).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayRejection {
    /// The move the engine refused to play.
    pub mv: Move,
}

impl std::fmt::Display for PlayRejection {
    /// `"engine rejected e1g1"` - the rejected move in UCI notation.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "engine rejected {}", Uci::from_move(&self.mv))
    }
}

impl From<shakmaty::PlayError<shakmaty::Chess>> for PlayRejection {
    /// The domain rejection, discarding the cloned position.
    fn from(err: shakmaty::PlayError<shakmaty::Chess>) -> Self {
        Self { mv: err.m }
    }
}
