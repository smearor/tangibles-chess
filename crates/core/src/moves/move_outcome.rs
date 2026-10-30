use shakmaty::Square;

use crate::IllegalReason;
use crate::San;

/// Result of validating a [`crate::MoveRequest`] against the position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveOutcome {
    /// The move was legal and applied; carries the SAN text.
    Applied(San),
    /// The requested move is a pawn promotion; the picker must supply a
    /// role and the request be retried.
    NeedsPromotion {
        /// Promotion target square.
        to: Square,
    },
    /// No legal move matches the request.
    Illegal(IllegalReason),
}
