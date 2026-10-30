//! Why an MCP request failed before or during dispatch.

use rmcp::ErrorData;

/// Dispatch-layer failures of the MCP server. These are transport or
/// boundary errors, distinct from application-level [`crate::CommandError`]
/// payloads the controller itself produces.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// The GTK-side command channel is gone (controller exited).
    #[error("game controller channel closed")]
    ChannelClosed,
    /// The controller dropped the oneshot without replying.
    #[error("controller dropped the request")]
    Dropped,
    /// A reply or payload failed JSON serialization.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// A tool argument failed boundary validation.
    #[error("{0}")]
    InvalidParams(String),
}

impl From<McpError> for ErrorData {
    /// Maps the failure to its JSON-RPC error: `InvalidParams` becomes
    /// `invalid_params`, everything else `internal_error`.
    fn from(err: McpError) -> Self {
        match err {
            McpError::InvalidParams(message) => ErrorData::invalid_params(message, None),
            other => ErrorData::internal_error(other.to_string(), None),
        }
    }
}
