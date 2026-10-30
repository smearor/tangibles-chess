//! The `resign` tool arguments.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// `resign` arguments.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct ResignArgs {
    /// `"white"` or `"black"` - the resigning side.
    pub(super) color: String,
}
