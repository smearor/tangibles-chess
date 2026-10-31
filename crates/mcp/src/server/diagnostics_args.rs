//! The `set_diagnostics` tool arguments.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// `set_diagnostics` arguments.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct DiagnosticsArgs {
    /// `true` opens the diagnostics overlay, `false` closes it.
    pub(super) enabled: bool,
}
