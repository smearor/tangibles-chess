use std::fmt::Display;

/// Why a [`crate::MoveRequest`] could not be derived from a legal
/// [`shakmaty::Move`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveRequestError {
    /// The move has no origin square (only possible for synthetic
    /// `Move` values; position-derived legal moves always have one).
    NoOrigin,
}

impl Display for MoveRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOrigin => f.write_str("move has no origin square"),
        }
    }
}

impl std::error::Error for MoveRequestError {}
