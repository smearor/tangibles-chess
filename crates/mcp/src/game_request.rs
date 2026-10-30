//! The command-plus-reply pair crossing from the MCP thread into the
//! GTK main loop.

use crate::command::CommandKind;
use serde_json::Value;
use tokio::sync::oneshot::Sender;

/// A command plus the channel back to the waiting tool call.
#[derive(Debug)]
pub struct GameRequest {
    /// What to execute.
    pub command: CommandKind,
    /// Reply channel; carries the JSON answer back to the tool.
    pub reply: Sender<Value>,
}
