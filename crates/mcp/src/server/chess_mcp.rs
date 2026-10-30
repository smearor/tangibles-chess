//! The `ChessMcp` server type and its command dispatch.

use std::sync::Arc;
use std::sync::Mutex;
use async_channel::Sender;
use rmcp::ErrorData;
use rmcp::RoleServer;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::service::Peer;

use crate::CommandKind;
use crate::GameRequest;

/// `(uri, peer)` pairs that subscribed via `resources/subscribe` and
/// receive `notifications/resources/updated` on each state change.
pub(super) type ResourceSubscribers = Arc<Mutex<Vec<(String, Peer<RoleServer>)>>>;

/// MCP server bridging the GTK game controller to remote tools.
#[derive(Clone)]
pub(super) struct ChessMcp {
    /// Command channel into the GTK main loop.
    pub(super) tx: Sender<GameRequest>,
    /// Live resource subscriptions; see [`ResourceSubscribers`].
    pub(super) subscribers: ResourceSubscribers,
}

impl ChessMcp {
    /// Dispatch a command and await its JSON reply.
    pub(super) async fn dispatch(&self, command: CommandKind) -> Result<CallToolResult, ErrorData> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(GameRequest { command, reply })
            .await
            .map_err(|_| ErrorData::internal_error("game controller channel closed", None))?;
        match rx.await {
            Ok(value) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string_pretty(&value).map_err(|err| ErrorData::internal_error(format!("json: {err}"), None))?,
            )])),
            Err(_) => Ok(CallToolResult::error(vec![ContentBlock::text("controller dropped the request".to_string())])),
        }
    }
}
