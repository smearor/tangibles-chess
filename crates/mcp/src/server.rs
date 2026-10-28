//! rmcp server surface: tools dispatch [`GameRequest`]s onto the GTK
//! main loop and await the JSON reply.

use std::net::SocketAddr;
use std::sync::Arc;

use rmcp::ErrorData;
use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::model::ServerCapabilities;
use rmcp::model::ServerConfig;
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;
use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use tracing::error;
use tracing::info;

use crate::CommandKind;
use crate::GameRequest;

/// MCP server bridging the GTK game controller to remote tools.
#[derive(Clone)]
struct ChessMcp {
    /// Command channel into the GTK main loop.
    tx: async_channel::Sender<GameRequest>,
}

impl ChessMcp {
    /// Dispatch a command and await its JSON reply.
    async fn dispatch(&self, command: CommandKind) -> Result<CallToolResult, ErrorData> {
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

/// `legal_moves` arguments.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct LegalMovesArgs {
    /// Optional origin square (e.g. `"e2"`) - lists only moves from it.
    square: Option<String>,
}

/// `make_move` arguments.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct MakeMoveArgs {
    /// UCI move like `"e2e4"`; promotion appends a role letter
    /// (`"e7e8q"` = promote to queen).
    uci: String,
}

/// `resign` arguments.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ResignArgs {
    /// `"white"` or `"black"` - the resigning side.
    color: String,
}

#[tool_router]
impl ChessMcp {
    /// Snapshot of the current game: phase, turn, FEN, history, outcome.
    #[tool(description = "Get the current game state: phase, turn, FEN, SAN history and outcome.")]
    async fn game_state(&self) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::GameState).await
    }

    /// All legal moves of the side to move, in UCI notation.
    #[tool(description = "List legal moves in UCI notation; optionally restricted to one origin square.")]
    async fn legal_moves(&self, Parameters(args): Parameters<LegalMovesArgs>) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::LegalMoves { square: args.square }).await
    }

    /// Apply a move given in UCI notation.
    #[tool(description = "Make a move in UCI notation, e.g. 'e2e4' or 'e7e8q' for promotion.")]
    async fn make_move(&self, Parameters(args): Parameters<MakeMoveArgs>) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::MakeMove { uci: args.uci }).await
    }

    /// Take back the last half-move (returns the physical undo plan).
    #[tool(description = "Undo the last half-move; returns the physical placement plan.")]
    async fn undo(&self) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::Undo).await
    }

    /// Resign for one side.
    #[tool(description = "Resign the game for 'white' or 'black'.")]
    async fn resign(&self, Parameters(args): Parameters<ResignArgs>) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::Resign { color: args.color }).await
    }

    /// Agree to a draw.
    #[tool(description = "Agree to a draw.")]
    async fn agree_draw(&self) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::AgreeDraw).await
    }

    /// Placement-gate state of the start handshake (tangible modes).
    #[tool(description = "Get the placement-gate status: placed/total, missing and unexpected fields.")]
    async fn placement_status(&self) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::PlacementStatus).await
    }

    /// PNG screenshot of the application window.
    #[tool(description = "Take a screenshot of the app window; returns a PNG image.")]
    async fn screenshot(&self) -> Result<CallToolResult, ErrorData> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(GameRequest {
                command: CommandKind::Screenshot,
                reply,
            })
            .await
            .map_err(|_| ErrorData::internal_error("game controller channel closed", None))?;
        match rx.await {
            Ok(value) => {
                let png = value.get("png_base64").and_then(|png| png.as_str());
                let Some(png) = png else {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(
                        value.get("error").and_then(|e| e.as_str()).unwrap_or("no screenshot produced").to_string(),
                    )]));
                };
                let meta = format!(
                    "{}x{}",
                    value.get("width").and_then(|w| w.as_u64()).unwrap_or(0),
                    value.get("height").and_then(|h| h.as_u64()).unwrap_or(0)
                );
                Ok(CallToolResult::success(vec![
                    ContentBlock::image(png.to_string(), "image/png".to_string()),
                    ContentBlock::text(format!("screenshot ({meta})")),
                ]))
            }
            Err(_) => Ok(CallToolResult::error(vec![ContentBlock::text("controller dropped the request".to_string())])),
        }
    }
}

#[tool_handler]
impl ServerHandler for ChessMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Tangibles Chess - tabletop chess on the tangible-objects surface. \
             Tools introspect the live game (state, legal moves, placement \
             handshake) and control it (make_move, undo, resign, agree_draw).",
        )
    }
}

/// Spawns the MCP Streamable-HTTP endpoint on a dedicated tokio runtime
/// thread; every tool call hops through `tx` onto the GTK main loop.
///
/// The default host allowlist is localhost-only - LAN deployments must
/// allow their own bound address.
pub fn spawn_http(tx: async_channel::Sender<GameRequest>, addr: SocketAddr) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(err) => {
                error!(%err, "MCP: failed to start tokio runtime");
                return;
            }
        };
        runtime.block_on(async move {
            // Host validation: a wildcard bind cannot predict which LAN
            // address clients connect through, so the allowlist is
            // disabled there. A concrete bind restricts the list to
            // loopback plus the bound address itself.
            let config = if addr.ip().is_unspecified() {
                StreamableHttpServerConfig::default().disable_allowed_hosts()
            } else {
                let allowed_hosts = [
                    "localhost".to_string(),
                    "127.0.0.1".to_string(),
                    "::1".to_string(),
                    addr.ip().to_string(),
                    addr.to_string(),
                ];
                StreamableHttpServerConfig::default().with_allowed_hosts(allowed_hosts)
            };
            let service = StreamableHttpService::new(move || Ok(ChessMcp { tx: tx.clone() }), Arc::new(LocalSessionManager::default()), config);
            let app = axum::Router::new().route_service("/mcp", service);
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    error!(%err, %addr, "MCP: bind failed");
                    return;
                }
            };
            info!(%addr, "MCP endpoint listening");
            if let Err(err) = axum::serve(listener, app).await {
                error!(%err, "MCP server failed");
            }
        });
    })
}
