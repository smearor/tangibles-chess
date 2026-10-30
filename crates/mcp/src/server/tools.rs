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
use crate::GameRequest;

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
            .map_err(|_| ErrorData::invalid_params("invalid square name", None))?;
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
            .map_err(|_| ErrorData::invalid_params("color must be 'white' or 'black'", None))?;
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
