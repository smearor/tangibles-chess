//! The `set_diagnostics` reply shape.

use serde::Serialize;

/// Result of a `set_diagnostics` request.
#[derive(Debug, Serialize)]
pub struct DiagnosticsReply {
    /// The overlay state now in effect.
    pub enabled: bool,
}
