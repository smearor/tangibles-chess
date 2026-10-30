//! The `make_move` tool arguments.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// `make_move` arguments.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub(super) struct MakeMoveArgs {
    /// UCI move like `"e2e4"`; promotion appends a role letter
    /// (`"e7e8q"` = promote to queen).
    pub(super) uci: String,
}
