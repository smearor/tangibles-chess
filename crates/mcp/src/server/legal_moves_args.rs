//! The `legal_moves` tool arguments.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// `legal_moves` arguments.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct LegalMovesArgs {
    /// Optional origin square (e.g. `"e2"`) - lists only moves from it.
    pub(super) square: Option<String>,
}
