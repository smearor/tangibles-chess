//! The `{"error": "..."}` reply shape.

use serde::Serialize;

/// `{"error": "..."}` - invalid input or unavailable state.
#[derive(Debug, Serialize)]
pub struct ErrorReply {
    /// What failed.
    pub error: String,
}
