//! `ServerHandler` on `ChessMcp`: capabilities, the `chess://game-state`
//! resource and its subscribe/unsubscribe.

use rmcp::ErrorData;
use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::model::ListResourcesResult;
use rmcp::model::PaginatedRequestParams;
use rmcp::model::ReadResourceRequestParams;
use rmcp::model::ReadResourceResponse;
use rmcp::model::ReadResourceResult;
use rmcp::model::Resource;
use rmcp::model::ResourceContents;
use rmcp::model::ServerCapabilities;
use rmcp::model::ServerConfig;
use rmcp::model::SubscribeRequestParams;
use rmcp::model::UnsubscribeRequestParams;
use rmcp::service::RequestContext;
use rmcp::tool_handler;

use super::chess_mcp::ChessMcp;
use crate::CommandKind;
use crate::GameRequest;
use crate::McpError;

#[tool_handler]
impl ServerHandler for ChessMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_subscribe()
                .build(),
        )
        .with_instructions(
            "Tangibles Chess - tabletop chess on the tangible-objects surface. \
             Tools introspect the live game (state, legal moves, placement \
             handshake) and control it (make_move, undo, resign, agree_draw). \
             The chess://game-state resource mirrors the game_state tool and \
             supports resources/subscribe for change notifications.",
        )
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, ErrorData>> {
        std::future::ready(Ok(ListResourcesResult::with_all_items(vec![
            Resource::new(crate::GAME_STATE_URI, "game-state")
                .with_title("Live game state")
                .with_description("Current game snapshot: phase, turn, FEN, SAN history and outcome.")
                .with_mime_type("application/json"),
        ])))
    }

    // `tool_handler` rewrites `async fn` back into `fn -> impl Future`,
    // which trips manual_async_fn on the generated code.
    #[allow(clippy::manual_async_fn)]
    async fn read_resource(&self, request: ReadResourceRequestParams, _context: RequestContext<RoleServer>) -> Result<ReadResourceResponse, ErrorData> {
        if request.uri != crate::GAME_STATE_URI {
            return Err(McpError::InvalidParams(format!("unknown resource {}", request.uri)).into());
        }
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(GameRequest {
                command: CommandKind::GameState,
                reply,
            })
            .await
            .map_err(|_| McpError::ChannelClosed)?;
        let reply = rx.await.map_err(|_| McpError::Dropped)?;
        let text = serde_json::to_string_pretty(&reply).map_err(McpError::from)?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(text, crate::GAME_STATE_URI).with_mime_type("application/json")]).into())
    }

    #[allow(deprecated)]
    fn subscribe(&self, request: SubscribeRequestParams, context: RequestContext<RoleServer>) -> impl Future<Output = Result<(), ErrorData>> {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push((request.uri, context.peer.clone()));
        }
        std::future::ready(Ok(()))
    }

    #[allow(deprecated)]
    fn unsubscribe(&self, request: UnsubscribeRequestParams, _context: RequestContext<RoleServer>) -> impl Future<Output = Result<(), ErrorData>> {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|(uri, _)| uri != &request.uri);
        }
        std::future::ready(Ok(()))
    }
}
