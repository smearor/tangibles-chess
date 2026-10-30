//! Why a move request was rejected.

use crate::GamePhase;
use crate::MoveRequest;
use crate::PlayRejection;

/// The reason a [`crate::MoveRequest`] did not apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IllegalReason {
    /// The game is not in [`GamePhase::InProgress`].
    WrongPhase(GamePhase),
    /// No legal move covers the request's `from` -> `to`.
    NotLegal(MoveRequest),
    /// The requested promotion role matches no candidate move.
    BadPromotion,
    /// The engine rejected the play call after validation.
    PlayRejected(PlayRejection),
}

impl IllegalReason {
    /// The Fluent key for the user-facing explanation.
    pub fn l10n_key(&self) -> &'static str {
        match self {
            Self::WrongPhase(_) => "illegal-wrong-phase",
            Self::NotLegal(_) => "illegal-not-legal",
            Self::BadPromotion => "illegal-bad-promotion",
            Self::PlayRejected(_) => "illegal-play-rejected",
        }
    }
}

impl std::fmt::Display for IllegalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongPhase(phase) => write!(f, "game is in phase {phase:?}"),
            Self::NotLegal(request) => write!(f, "{}-{} is not a legal move", request.from, request.to),
            Self::BadPromotion => f.write_str("no matching move for the given promotion"),
            Self::PlayRejected(err) => write!(f, "{err}"),
        }
    }
}
