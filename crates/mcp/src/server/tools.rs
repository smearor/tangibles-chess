//! The `#[tool_router]` surface: one tool method per [`CommandKind`].

use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::tool;
use rmcp::tool_router;
use tangibles_chess_core::Side;
use tangibles_chess_core::Uci;

use super::chess_mcp::ChessMcp;
use super::legal_moves_args::LegalMovesArgs;
use super::make_move_args::MakeMoveArgs;
use super::resign_args::ResignArgs;
use crate::CommandKind;
use crate::CommandReply;
use crate::GameRequest;
use crate::McpError;

#[tool_router(vis = "pub(super)")]
impl ChessMcp {
    /// Snapshot of the current game: phase, turn, FEN, history, outcome.
    #[tool(description = "Get the current game state: phase, turn, FEN, SAN history and outcome.")]
    async fn game_state(&self) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::GameState).await
    }

    /// All legal moves of the side to move, in UCI notation.
    #[tool(description = "List legal moves in UCI notation; optionally restricted to one origin square.")]
    async fn legal_moves(&self, Parameters(args): Parameters<LegalMovesArgs>) -> Result<CallToolResult, ErrorData> {
        let square = args
            .square
            .as_deref()
            .map(str::parse)
            .transpose()
            .map_err(|_| McpError::InvalidParams("invalid square name".to_string()))?;
        self.dispatch(CommandKind::LegalMoves { square }).await
    }

    /// Apply a move given in UCI notation.
    #[tool(description = "Make a move in UCI notation, e.g. 'e2e4' or 'e7e8q' for promotion.")]
    async fn make_move(&self, Parameters(args): Parameters<MakeMoveArgs>) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::MakeMove { uci: Uci::from(args.uci) }).await
    }

    /// Take back the last half-move (returns the physical undo plan).
    #[tool(description = "Undo the last half-move; returns the physical placement plan.")]
    async fn undo(&self) -> Result<CallToolResult, ErrorData> {
        self.dispatch(CommandKind::Undo).await
    }

    /// Resign for one side.
    #[tool(description = "Resign the game for 'white' or 'black'.")]
    async fn resign(&self, Parameters(args): Parameters<ResignArgs>) -> Result<CallToolResult, ErrorData> {
        let color = args
            .color
            .parse::<Side>()
            .map_err(|_| McpError::InvalidParams("color must be 'white' or 'black'".to_string()))?;
        self.dispatch(CommandKind::Resign { color }).await
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
            .map_err(|_| McpError::ChannelClosed)?;
        match rx.await.map_err(|_| McpError::Dropped)? {
            CommandReply::Screenshot(reply) => Ok(CallToolResult::success(vec![
                ContentBlock::image(reply.png_base64, "image/png".to_string()),
                ContentBlock::text(format!("screenshot ({}x{})", reply.width, reply.height)),
            ])),
            CommandReply::Error(err) => Ok(CallToolResult::error(vec![ContentBlock::text(err.to_string())])),
            _ => Err(McpError::InvalidParams("unexpected reply kind for screenshot".to_string()).into()),
        }
    }
}
