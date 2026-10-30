//! The typed reply a command produces on the GTK side.

use serde::Serialize;

use crate::command_error::CommandError;
use crate::reply::DrawReply;
use crate::reply::GameStateReply;
use crate::reply::LegalMovesReply;
use crate::reply::MakeMoveReply;
use crate::reply::PlacementReply;
use crate::reply::ResignReply;
use crate::reply::ScreenshotReply;
use crate::reply::UndoReply;

/// The typed answer crossing the `GameRequest` oneshot back to the MCP
/// server. Untagged serialization keeps each variant's wire shape
/// identical to the reply type it wraps.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CommandReply {
    /// `game_state` snapshot.
    GameState(GameStateReply),
    /// `legal_moves` list.
    LegalMoves(LegalMovesReply),
    /// `make_move` outcome.
    MakeMove(MakeMoveReply),
    /// `undo` outcome.
    Undo(UndoReply),
    /// `resign` outcome.
    Resign(ResignReply),
    /// `agree_draw` outcome.
    Draw(DrawReply),
    /// `placement_status` gate state.
    Placement(PlacementReply),
    /// `screenshot` PNG payload.
    Screenshot(ScreenshotReply),
    /// Application-level failure (`no active game`, invalid move text,
    /// unrealized window, ...).
    Error(CommandError),
}
