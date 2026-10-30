//! The `agree_draw` reply shape.

use serde::Serialize;

/// Result of an `agree_draw` request.
#[derive(Debug, Serialize)]
pub struct DrawReply {
    /// Always `true`.
    pub draw_agreed: bool,
    /// Terminal outcome.
    pub outcome: Option<String>,
}
