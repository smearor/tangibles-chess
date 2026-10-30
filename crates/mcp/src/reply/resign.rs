//! The `resign` reply shape.

use serde::Serialize;
use tangibles_chess_material::Side;
use tangibles_chess_session::GameOutcome;

/// Result of a `resign` request.
#[derive(Debug, Serialize)]
pub struct ResignReply {
    /// The side that resigned.
    pub resigned: Side,
    /// Terminal outcome.
    pub outcome: Option<GameOutcome>,
}
