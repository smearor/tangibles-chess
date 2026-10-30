//! The `resign` reply shape.

use serde::Serialize;
use tangibles_chess_core::Side;

/// Result of a `resign` request.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ResignReply {
    /// The `color` argument was invalid.
    Declined {
        /// Always `false` for this variant.
        resigned: bool,
        /// Why nothing happened.
        reason: String,
    },
    /// The game ended by resignation.
    Resigned {
        /// The side that resigned.
        resigned: Side,
        /// Terminal outcome.
        outcome: Option<String>,
    },
}
