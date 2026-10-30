//! The `resign` reply shape.

use serde::Serialize;
use tangibles_chess_core::GameOutcome;
use tangibles_chess_core::Side;

/// Result of a `resign` request.
#[derive(Debug, Serialize)]
pub struct ResignReply {
    /// The side that resigned.
    pub resigned: Side,
    /// Terminal outcome.
    pub outcome: Option<GameOutcome>,
}
