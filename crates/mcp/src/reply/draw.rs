//! The `agree_draw` reply shape.

use serde::Serialize;
use tangibles_chess_session::GameOutcome;

/// Result of an `agree_draw` request.
#[derive(Debug, Serialize)]
pub struct DrawReply {
    /// Always `true`.
    pub draw_agreed: bool,
    /// Terminal outcome.
    pub outcome: Option<GameOutcome>,
}
